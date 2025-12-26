# VexLake: Separation of Compute and Storage Vector Database Design (v4.1)

> Evolving from an in-memory vector database to a cloud-native separation of compute and storage architecture

---

## 📋 Design Evolution Overview

### Legacy Design (Vex v3.0) - In-Memory Vector Database
- **Positioning**: Lightweight, production-grade in-memory vector database
- **Core Features**: 
  - RESP protocol compatible (connect directly with redis-cli)
  - 32 shards storage + cache-line padding to prevent false sharing
  - Automatic vector normalization to optimize cosine similarity calculation
  - Built-in benchmark tools and observability system
- **Limitations**:
  - Pure in-memory storage, no persistence
  - Single-node architecture, cannot scale horizontally
  - Tightly coupled compute and storage

### New Design (VexLake v4.1) - Cloud-Native Separation of Compute and Storage
- **Positioning**: Serverless vector analytics engine based on object storage
- **Core Philosophy**: **Sandwich Architecture** - Go handles the facade (Network/Protocol), Rust handles the core (ALL Compute + I/O)
- **Goal**: Redis protocol compatible + S3 storage + DataFusion compute
- **Design Philosophy**: "Don't leave Rust just for I/O" - Let Rust handle both data transport AND data processing

---

## 🧠 Core Design Principles

### 1. Sandwich Architecture (v4.1 Key Improvement)

> **"Go handles the facade, Rust handles the core"**

The original v4.0 design had a **critical performance bottleneck**: TopK vector computation was done in Go while Rust only handled I/O. This is like "buying a Ferrari chassis (Rust I/O) but installing a 2.0T engine (Go Compute) instead of a V12 (Rust Compute)."

**Key Insight**: Vector search (whether brute-force cosine similarity or HNSW graph traversal) is **extremely CPU-intensive**. Go's compiler (gc) is far weaker than Rust (LLVM) at **SIMD (Single Instruction Multiple Data)** auto-vectorization. Rust vector libraries (like `faiss-rs` or `lance`) typically leverage AVX-512, NEON instruction sets for extreme optimization.

**v4.1 Architecture Principle**:
- **Go**: Only handles network protocol (RESP parsing, client management)
- **Rust**: Handles **ALL heavy lifting** - index loading, TopK computation, Parquet reading, result assembly

### 2. LanceDB Philosophy (Not Code Integration)

> **Borrow LanceDB's design philosophy, not its library code**

**Reasons**:
- LanceDB core is pure Rust; Go calling it requires CGO, which is complex to build and debug
- Directly embedding LanceDB would turn the project into "a Go shell for LanceDB," deviating from customization goals
- Core value lies in **implementing index and data separation architecture ourselves**

**LanceDB Core Concepts**:
- **Data is the Format**: The database isn't a black-box daemon process, but a set of libraries managing file formats
- **Separation of Index and Data**: Solves the "columnar formats not suitable for vector retrieval" problem
- **Designed for Object Storage**: Naturally suited to run on S3

### 3. SeaweedFS as Storage Backend

> **Why SeaweedFS over MinIO**

**SeaweedFS Advantages**:
- Based on Facebook Haystack paper, optimized for massive small files
- Separation of metadata (Filer) and data blocks (Volume), O(1) disk read efficiency
- Lower latency than Ceph/MinIO when handling small objects
- Excellent S3 API compatibility

### 4. Technology Stack Mapping

| LanceDB Component | VexLake Alternative | Rationale |
|-------------------|---------------------|-----------|
| Lance Format | **Parquet** | Mature native support in DataFusion |
| ObjectStore | **SeaweedFS (S3 API)** | Better performance, optimized for small files |
| DataFusion | **DataFusion (via CGO/FFI)** | Linked as .so/.a library, not RPC |
| Lance IVF-PQ | **Rust index with SIMD** | Core value, leverages LLVM optimization |

---

## 🏗️ Architecture Design (v4.1 - Sandwich Architecture)

### Overall Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                          Client Layer                              │
│         redis-cli / redis-py / Jedis / Any Redis Client            │
└─────────────────────────────┬──────────────────────────────────────┘
                              │ RESP Protocol (port 6379)
                              ▼
┌────────────────────────────────────────────────────────────────────┐
│              Go Layer (The Facade - Network Only)                  │
│  ┌─────────────┐  ┌─────────────────┐  ┌────────────────────────┐  │
│  │ RESP Parser │  │ Connection Mgr  │  │   Result Formatter     │  │
│  │ (redcon)    │  │ (Client Pool)   │  │   (Arrow → RESP)       │  │
│  └─────────────┘  └─────────────────┘  └────────────────────────┘  │
└─────────────────────────────┬──────────────────────────────────────┘
                              │ CGO / FFI (Arrow C Data Interface)
                              │ ★ Single call, Zero-Copy return ★
                              ▼
┌────────────────────────────────────────────────────────────────────┐
│         Rust Core (libvector_engine.so - ALL Heavy Lifting)        │
│  ┌─────────────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │ Index Manager   │  │ TopK Compute │  │   OpenDAL              │ │
│  │ (HNSW/IVF+SIMD) │  │ (SIMD Accel) │  │   (S3 I/O)             │ │
│  └─────────────────┘  └──────────────┘  └────────────────────────┘ │
│  ┌─────────────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │ DataFusion      │  │ Parquet Read │  │   Result Assembly      │ │
│  │ (SQL Engine)    │  │ (Range Req)  │  │   (Arrow RecordBatch)  │ │
│  └─────────────────┘  └──────────────┘  └────────────────────────┘ │
└─────────────────────────────┬──────────────────────────────────────┘
                              │ S3 API
                              ▼
┌────────────────────────────────────────────────────────────────────┐
│                  Storage Layer (SeaweedFS)                         │
│   s3://vex-bucket/                                                 │
│   ├── _metadata/           # Version metadata (JSON)               │
│   ├── data/                # Raw data (Parquet format)             │
│   └── index/               # Vector index (binary files)           │
└────────────────────────────────────────────────────────────────────┘
```

---

## 🔧 Core Component Design (v4.1)

### 1. Go Layer (The Facade) - Network Only

**Responsibilities** (Minimal):
- **RESP Parser**: Parse Redis protocol commands
- **Connection Manager**: Manage client connections
- **Result Formatter**: Convert Arrow RecordBatch to RESP format

**What Go Does NOT Do**:
- ❌ Index loading/management
- ❌ Vector TopK computation
- ❌ Parquet file reading
- ❌ Any CPU-intensive work

**Tech Stack**:
- `tidwall/redcon` - High-performance RESP protocol parsing
- CGO - Link to Rust library (libvector_engine.so)
- Arrow C Data Interface - Zero-copy data exchange with Rust

### 2. Rust Core (libvector_engine.so) - The Muscle

**Responsibilities** (Everything Compute):
- **Index Manager**: Load, cache, and query HNSW/IVF indexes (with SIMD acceleration)
- **TopK Compute**: Calculate vector distances using AVX-512/NEON instructions
- **S3 I/O**: Async read from SeaweedFS using OpenDAL
- **Parquet Reader**: Precision fetch with Range Request
- **Result Assembly**: Build Arrow RecordBatch for return

**Tech Stack**:
- `datafusion` - SQL execution engine
- `opendal` - Object storage access abstraction
- `faiss-rs` / custom SIMD - Vector distance computation
- `tokio` - Async runtime
- Arrow C Data Interface - Export results to Go without copy

**Custom Vector UDFs**:
```sql
-- L2 distance (SIMD accelerated in Rust)
SELECT id, text FROM embeddings
ORDER BY l2_distance(vector_col, [0.1, 0.2, ...]) LIMIT 10;

-- Cosine similarity
SELECT id, cosine_similarity(vector_col, $query) as score
FROM embeddings ORDER BY score DESC LIMIT K;
```

### 3. Storage Layer (SeaweedFS + Parquet + Index Separation)

**Design Principles**: 
- S3 can only overwrite, not modify → Use **LSM-Tree + MVCC** approach
- Physically separate index from data → Solve Parquet's poor random read performance

**Directory Structure**:
```
s3://vex-bucket/
├── _metadata/               # Metadata (managed by Rust)
│   └── version_N.json       
├── data/                    # Raw data (Parquet)
│   └── partition_N/data_N.parquet
└── index/                   # Vector index (binary files)
    ├── hnsw_v1.bin          # Store only Vector ID + Embedding
    └── ivf_v1.bin
```

---

## 🔄 Core Workflows (v4.1 - Optimized)

### Read Path - Rust Closed-Loop (Key Improvement)

```
1. Client ─────► VSEARCH "[0.1, 0.2, ...]" 10
                    │
2. Go Gateway ◄─────┘ Parse RESP command only
                    │
                    ▼ CGO / FFI call (pass query vector & params)
                    │
┌───────────────────┴────────────────────────────────────────────────┐
│                   Rust Core (Closed Loop)                          │
│                                                                    │
│   A. Load/Query Index ──► SIMD-accelerated TopK computation        │
│                    │                                               │
│   B. Get TopK IDs ─┘                                               │
│                    │                                               │
│   C. Parallel S3 Read ──► Range Request for Parquet rows           │
│                    │                                               │
│   D. Assemble Arrow RecordBatch                                    │
│                                                                    │
└───────────────────┬────────────────────────────────────────────────┘
                    │
                    ▼ Arrow C Data Interface (Zero-Copy pointer)
                    │
3. Go ◄─────────────┘ Receives pointer to Arrow Record
                    │
4. Go ──────────────► Client: Serialize to RESP Array and return
```

**Why This Is Better**:
1. **Compute Pushdown**: TopK calculation moved to Rust, 2-10x faster due to SIMD
2. **Reduced Cross-Language Calls**: 
   - Old: Go → (call) → Rust → (call) → Go (chatty)
   - New: Go → (single call) → Rust (does everything) → (single return) → Go
3. **Zero-Copy Return**: Arrow C Data Interface means no memory copy between Rust and Go

### Write Path - Index and Data Separation

```
1. Client ─────► SET key "[0.1, 0.2, ...]"
                    │
2. Go Gateway ◄─────┘ Parse RESP, forward to Rust
                    │
                    ▼ CGO call
                    │
┌───────────────────┴────────────────────────────────────────────────┐
│                   Rust Core (Handles All Writes)                   │
│                                                                    │
│   A. Buffer in MemTable (wait for 10MB fill)                       │
│                    │                                               │
│   B. Flush to S3:  ├── Parquet (data) → S3                         │
│                    ├── Index file → S3                             │
│                    └── version.json → S3 (atomic commit)           │
│                                                                    │
└───────────────────┬────────────────────────────────────────────────┘
                    │
3. Go ◄─────────────┘ Return +OK
```

### Background Compaction

```
Rust scheduled task detects:
  → version_latest has 100 small files?
  → Merge into 1 large file, rebuild index
  → Atomically update metadata
  → Old files can be deleted later (GC)
```

---

## ⚡ Performance Comparison: v4.0 vs v4.1

| Aspect | v4.0 (Go computes TopK) | v4.1 (Rust computes TopK) |
|--------|-------------------------|---------------------------|
| **TopK Latency** | 5-20ms (Go, no SIMD) | 1-5ms (Rust, AVX-512) |
| **Cross-Language Calls** | 2+ per query | 1 per query |
| **Memory Copy** | Possible (if not careful) | Zero-Copy (Arrow C Interface) |
| **SIMD Utilization** | ❌ Go compiler weak | ✅ LLVM fully optimized |
| **Scalability** | Limited by Go compute | Limited by network I/O |

---

## ✨ Design Highlights (v4.1)

| Feature | Description |
|---------|-------------|
| **Sandwich Architecture** | Go for network facade, Rust for all compute |
| **SIMD-Accelerated TopK** | 2-10x faster vector computation in Rust |
| **Zero-Copy Interface** | Arrow C Data Interface between Go and Rust |
| **Single Call Pattern** | "Don't be chatty" - Rust completes all work in one call |
| **Redis Protocol Compatible** | No special client needed, redis-cli works directly |
| **Separation of Compute and Storage** | Storage on SeaweedFS (cheap), compute in Rust (high-performance) |
| **MVCC Transactions** | Snapshot isolation via versioned metadata |

---

## 📊 Comparison with Legacy Design

| Dimension | Vex v3.0 (Legacy) | VexLake v4.1 (New) |
|-----------|-------------------|---------------------|
| **Storage** | Pure in-memory | SeaweedFS (S3 API) |
| **Persistence** | ❌ | ✅ (Parquet + Index files) |
| **Horizontal Scaling** | ❌ Single node | ✅ Multiple Rust compute nodes |
| **Protocol** | RESP | RESP (backward compatible) |
| **Compute Engine** | Native Go | DataFusion + SIMD (Rust) |
| **TopK Performance** | Moderate | Extreme (SIMD optimized) |
| **Cost** | High (memory) | Low (object storage + on-demand compute) |

---

## ⚠️ Pitfall Guide

### 1. Don't Do Vector Computation in Go
- Go compiler (gc) is weak at SIMD auto-vectorization
- **Push ALL compute to Rust** - let Go only handle network

### 2. Use Arrow C Data Interface for Zero-Copy
- If you copy data between Go and Rust, you lose all performance gains
- **Must use Arrow C Data Interface** for zero-copy pointer passing

### 3. Minimize Cross-Language Call Frequency
- "Chatty interfaces are slow"
- **Let Rust complete all work in a single call**, return only final results

### 4. Don't Use RPC for Go-Rust Communication
- gRPC/HTTP between Go and Rust adds serialization overhead
- **Use CGO/FFI** - compile Rust as .so/.a and link directly

### 5. Be Aware of S3 Eventual Consistency
- Reads immediately after writes may not see the data
- **Metadata updates need atomicity guarantees**

---

## 🚀 Implementation Roadmap (v4.1)

### Phase 1: Rust Core Library
- [ ] Create Rust library with C-compatible FFI exports
- [ ] Implement basic vector search with SIMD (faiss-rs or custom)
- [ ] Implement Arrow C Data Interface for result export

### Phase 2: Go-Rust Integration
- [ ] Set up CGO wrapper for Rust library
- [ ] Implement zero-copy data exchange via Arrow C Interface
- [ ] Verify no memory leaks across language boundary

### Phase 3: Storage Layer (SeaweedFS)
- [ ] Deploy SeaweedFS cluster (Master + Volume + Filer)
- [ ] Rust: Integrate `opendal` to access SeaweedFS S3 API
- [ ] Rust: Implement version metadata management

### Phase 4: Data Format (Parquet + Index)
- [ ] Rust: Implement Parquet read/write with DataFusion
- [ ] Rust: Implement HNSW/IVF index building and serialization
- [ ] Rust: Implement index caching in memory

### Phase 5: RESP Gateway
- [ ] Go: Implement RESP protocol parsing with `redcon`
- [ ] Go: Implement command routing (SET/GET/VSEARCH)
- [ ] Go: Implement Arrow → RESP result conversion

### Phase 6: Production Hardening
- [ ] Implement Write Buffer (MemTable) and batch flushing
- [ ] Implement background Compaction (LSM-Tree)
- [ ] Add observability (metrics/tracing)
- [ ] Performance benchmarks

---

## 🛤️ Architecture Routes

### Route A: Go as Pure Glue (Recommended)
Build a Rust dynamic library (libvector_engine.so) that encapsulates DataFusion and index logic. Go only wraps it with CGO to provide Redis protocol service.

- **Performance**: ⭐⭐⭐⭐⭐ (Extreme)
- **Development Difficulty**: ⭐⭐⭐⭐ (Requires Rust FFI and Unsafe knowledge)

### Route B: Go Handles Compute, Rust Only Reads Files (NOT Recommended)
- **Performance**: ⭐⭐⭐ (Moderate)
- **When Acceptable**: If vector data is small (hundreds of thousands), Go computes fast enough, and bottleneck is mainly network I/O
- **Remediation**: Use assembly-optimized Go libraries (`gonum` or SIMD Go libraries), not pure Go `for` loops

---

## 📚 Technical References

- [DataFusion](https://github.com/apache/datafusion) - Rust SQL execution engine
- [OpenDAL](https://github.com/apache/opendal) - Rust storage abstraction layer
- [SeaweedFS](https://github.com/seaweedfs/seaweedfs) - High-performance distributed file system
- [Arrow C Data Interface](https://arrow.apache.org/docs/format/CDataInterface.html) - Zero-copy cross-language data exchange
- [faiss-rs](https://github.com/Enet4/faiss-rs) - Rust bindings for FAISS
- [tidwall/redcon](https://github.com/tidwall/redcon) - Go RESP server framework
- [LanceDB](https://github.com/lancedb/lance) - Design philosophy reference

---

*This document consolidates design ideas from `new_design.md`, `resp+s3.md`, `04_add_seaweedfs.md`, `05_why_not_lancedb.md`, and **`07_improve.md`**, reflecting the optimized Sandwich Architecture for maximum performance.*
