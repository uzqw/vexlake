# VexLake Development Plan v1.0

> Strict Test-Driven Development with Small Iterations

This development plan follows the **Sandwich Architecture** defined in the design documents, implementing VexLake as a hybrid Go + Rust cloud-native vector database with SeaweedFS storage.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Go Layer (cmd/vexlake-server)                │
│   • RESP protocol (tidwall/redcon)                              │
│   • Client connection management                                │
│   • Arrow → RESP result formatting                              │
└───────────────────────────┬─────────────────────────────────────┘
                            │ CGO / FFI (Arrow C Data Interface)
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│              Rust Core (crates/vexlake-core)                    │
│   • libvector_engine.so                                         │
│   • Index Manager (HNSW/IVF + SIMD)                             │
│   • TopK Computation (AVX-512/NEON)                             │
│   • S3 I/O via OpenDAL                                          │
│   • Parquet Read/Write via DataFusion                           │
└───────────────────────────┬─────────────────────────────────────┘
                            │ S3 API
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                     SeaweedFS Storage                           │
│   s3://vexlake/data/*.parquet                                   │
│   s3://vexlake/index/*.bin                                      │
│   s3://vexlake/_metadata/version_N.json                         │
└─────────────────────────────────────────────────────────────────┘
```

---

## Development Principles

### 1. Test-Driven Development (TDD)
- **Write tests BEFORE implementation**
- Every feature must have unit tests with >80% coverage
- Integration tests for cross-component interactions
- Benchmark tests for performance-critical paths

### 2. Small Iterations
- Each milestone should be completable in **1-3 days**
- Each PR should be **<500 lines** of code
- Feature flags for incomplete features
- Continuous integration on every commit

### 3. Documentation-First
- Update docs BEFORE or WITH implementation
- API documentation for all public interfaces
- Architecture Decision Records (ADRs) for key decisions

---

## Phase 1: Foundation (Weeks 1-2)

### Milestone 1.1: Project Setup
**Duration:** 1 day

| Task | Description | Verification |
|------|-------------|--------------|
| 1.1.1 | Initialize Cargo workspace | `cargo build` succeeds |
| 1.1.2 | Initialize Go module | `go mod tidy` succeeds |
| 1.1.3 | Set up CI/CD | GitHub Actions green |
| 1.1.4 | Configure linting | `make verify` passes |

**Test Requirements:**
- [ ] `cargo test` runs (even if no tests yet)
- [ ] `go test ./...` runs
- [ ] CI pipeline completes successfully

---

### Milestone 1.2: Rust Core Skeleton
**Duration:** 2 days

| Task | Description | Verification |
|------|-------------|--------------|
| 1.2.1 | Create `vexlake-core` crate structure | Compiles |
| 1.2.2 | Define FFI boundary with `#[no_mangle]` | `cbindgen` generates header |
| 1.2.3 | Implement basic health check FFI | Go can call Rust |
| 1.2.4 | Set up error handling across FFI | Panics don't crash Go |

**Test Requirements:**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_health_check() {
        assert!(health_check());
    }
    
    #[test]
    fn test_ffi_error_handling() {
        // Verify panic recovery via catch_unwind
    }
}
```

**Integration Test:**
```go
func TestRustHealthCheck(t *testing.T) {
    result := C.vexlake_health_check()
    if result != 1 {
        t.Fatal("Rust health check failed")
    }
}
```

---

### Milestone 1.3: Go Gateway Skeleton
**Duration:** 2 days

| Task | Description | Verification |
|------|-------------|--------------|
| 1.3.1 | Set up `tidwall/redcon` server | PING returns PONG |
| 1.3.2 | Implement PING/ECHO commands | redis-cli test |
| 1.3.3 | CGO wrapper for Rust library | Compiles with CGO |
| 1.3.4 | Graceful shutdown handling | SIGTERM works |

**Test Requirements:**
```go
func TestPingCommand(t *testing.T) {
    conn := connectToServer(t)
    defer conn.Close()
    
    resp := sendCommand(conn, "PING")
    assert.Equal(t, "+PONG\r\n", resp)
}

func TestEchoCommand(t *testing.T) {
    conn := connectToServer(t)
    resp := sendCommand(conn, "ECHO hello")
    assert.Contains(t, resp, "hello")
}
```

---

## Phase 2: Storage Layer (Weeks 3-4)

### Milestone 2.1: OpenDAL Integration
**Duration:** 2 days

| Task | Description | Verification |
|------|-------------|--------------|
| 2.1.1 | Add `opendal` dependency | Compiles |
| 2.1.2 | Implement S3 operator factory | Unit test with mock |
| 2.1.3 | Implement basic read/write | Integration test |
| 2.1.4 | Add connection pooling | Benchmark shows improvement |

**Test Requirements:**
```rust
#[tokio::test]
async fn test_s3_write_read() {
    let op = create_test_operator(); // Use memory backend for tests
    op.write("test/key", "value").await.unwrap();
    let data = op.read("test/key").await.unwrap();
    assert_eq!(data, b"value");
}

#[tokio::test]
async fn test_s3_list_objects() {
    let op = create_test_operator();
    // ... test listing
}
```

**Environment Test (Manual):**
```bash
# Start local SeaweedFS
docker-compose up -d seaweedfs

# Run integration tests against real S3
VEXLAKE_S3_ENDPOINT=http://localhost:8333 cargo test --features integration
```

---

### Milestone 2.2: Parquet Read/Write
**Duration:** 3 days

| Task | Description | Verification |
|------|-------------|--------------|
| 2.2.1 | Add `arrow` and `parquet` dependencies | Compiles |
| 2.2.2 | Define vector record schema | Schema validates |
| 2.2.3 | Implement Parquet writer | Write + read roundtrip |
| 2.2.4 | Implement Range Request reader | Partial read works |
| 2.2.5 | Optimize Row Group size | Benchmark comparison |

**Test Requirements:**
```rust
#[test]
fn test_vector_schema() {
    let schema = VectorRecordSchema::new(128); // 128-dim vectors
    assert_eq!(schema.fields().len(), 3); // id, vector, metadata
}

#[tokio::test]
async fn test_parquet_roundtrip() {
    let records = generate_test_records(1000);
    let bytes = write_parquet(&records).await.unwrap();
    let loaded = read_parquet(&bytes).await.unwrap();
    assert_eq!(records, loaded);
}

#[tokio::test]
async fn test_range_request_read() {
    // Verify only specific rows are fetched
    let metrics = read_with_metrics(row_ids: vec![5, 10, 15]).await;
    assert!(metrics.bytes_read < total_file_size * 0.1); // <10% of file
}
```

---

### Milestone 2.3: Metadata Management
**Duration:** 2 days

| Task | Description | Verification |
|------|-------------|--------------|
| 2.3.1 | Define version metadata JSON schema | Schema validates |
| 2.3.2 | Implement atomic version updates | Concurrent test passes |
| 2.3.3 | Implement version listing | List returns all versions |
| 2.3.4 | Implement snapshot isolation | MVCC test passes |

**Test Requirements:**
```rust
#[tokio::test]
async fn test_version_atomicity() {
    // Concurrent writes should not corrupt metadata
    let handles: Vec<_> = (0..10).map(|i| {
        tokio::spawn(async move {
            update_version(i).await
        })
    }).collect();
    
    for h in handles { h.await.unwrap(); }
    
    let versions = list_versions().await;
    assert_eq!(versions.len(), 10);
}

#[tokio::test]
async fn test_snapshot_isolation() {
    let v1 = create_snapshot().await;
    write_new_data().await;
    let v2 = create_snapshot().await;
    
    // Reading from v1 should not see new data
    let data_v1 = read_at_version(v1).await;
    let data_v2 = read_at_version(v2).await;
    assert!(data_v2.len() > data_v1.len());
}
```

---

## Phase 3: Vector Index (Weeks 5-7)

### Milestone 3.1: Vector Distance Computation
**Duration:** 2 days

| Task | Description | Verification |
|------|-------------|--------------|
| 3.1.1 | Implement naive cosine similarity | Correctness test |
| 3.1.2 | Add SIMD-optimized version | 5x+ speedup |
| 3.1.3 | Implement L2 distance | Correctness test |
| 3.1.4 | Implement dot product | Correctness test |
| 3.1.5 | Add automatic normalization | Unit test |

**Test Requirements:**
```rust
#[test]
fn test_cosine_similarity_correctness() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);
    
    let c = vec![1.0, 0.0, 0.0];
    assert!((cosine_similarity(&a, &c) - 1.0).abs() < 1e-6);
}

#[test]
fn test_simd_matches_naive() {
    let a = random_vector(128);
    let b = random_vector(128);
    let naive = cosine_similarity_naive(&a, &b);
    let simd = cosine_similarity_simd(&a, &b);
    assert!((naive - simd).abs() < 1e-5);
}
```

**Benchmark Test:**
```rust
#[bench]
fn bench_cosine_naive(b: &mut Bencher) {
    let v1 = random_vector(128);
    let v2 = random_vector(128);
    b.iter(|| cosine_similarity_naive(&v1, &v2));
}

#[bench]
fn bench_cosine_simd(b: &mut Bencher) {
    let v1 = random_vector(128);
    let v2 = random_vector(128);
    b.iter(|| cosine_similarity_simd(&v1, &v2));
}
```

---

### Milestone 3.2: Brute-Force TopK
**Duration:** 2 days

| Task | Description | Verification |
|------|-------------|--------------|
| 3.2.1 | Implement single-threaded TopK | Correctness test |
| 3.2.2 | Add parallel TopK with Rayon | Speedup test |
| 3.2.3 | Optimize memory allocation | Reduced allocations |
| 3.2.4 | Add early termination | Benchmark improvement |

**Test Requirements:**
```rust
#[test]
fn test_topk_correctness() {
    let query = random_vector(128);
    let dataset = generate_vectors(10000, 128);
    
    let topk = brute_force_topk(&query, &dataset, 10);
    
    // Verify sorted by similarity
    for i in 1..topk.len() {
        assert!(topk[i-1].score >= topk[i].score);
    }
}

#[test]
fn test_topk_parallel_matches_sequential() {
    let query = random_vector(128);
    let dataset = generate_vectors(10000, 128);
    
    let seq = brute_force_topk_seq(&query, &dataset, 10);
    let par = brute_force_topk_par(&query, &dataset, 10);
    
    assert_eq!(seq, par);
}
```

---

### Milestone 3.3: HNSW Index
**Duration:** 5 days

| Task | Description | Verification |
|------|-------------|--------------|
| 3.3.1 | Implement HNSW graph structure | Unit tests |
| 3.3.2 | Implement insert algorithm | Insert correctness |
| 3.3.3 | Implement search algorithm | Search correctness |
| 3.3.4 | Add serialization/deserialization | Roundtrip test |
| 3.3.5 | Integrate with S3 storage | Integration test |
| 3.3.6 | Add memory caching | Cache hit rate >90% |

**Test Requirements:**
```rust
#[test]
fn test_hnsw_insert_and_search() {
    let mut index = HnswIndex::new(HnswConfig::default());
    
    for (id, vec) in generate_vectors(1000, 128).iter().enumerate() {
        index.insert(id as u64, vec);
    }
    
    let query = random_vector(128);
    let results = index.search(&query, 10);
    
    // Verify recall against brute force
    let brute_force = brute_force_topk(&query, &all_vectors, 10);
    let recall = calculate_recall(&results, &brute_force);
    assert!(recall > 0.95); // >95% recall
}

#[test]
fn test_hnsw_serialization() {
    let index = build_test_index(1000);
    let bytes = index.serialize().unwrap();
    let loaded = HnswIndex::deserialize(&bytes).unwrap();
    
    // Search results should be identical
    let query = random_vector(128);
    assert_eq!(index.search(&query, 10), loaded.search(&query, 10));
}
```

---

## Phase 4: Go-Rust Integration (Weeks 8-9)

### Milestone 4.1: Arrow C Data Interface
**Duration:** 3 days

| Task | Description | Verification |
|------|-------------|--------------|
| 4.1.1 | Implement Arrow export in Rust | FFI struct correct |
| 4.1.2 | Implement Arrow import in Go | Data accessible |
| 4.1.3 | Verify zero-copy transfer | Memory not duplicated |
| 4.1.4 | Handle memory lifecycle | No leaks (valgrind) |

**Test Requirements:**
```rust
#[test]
fn test_arrow_export() {
    let batch = create_test_batch();
    let ffi = export_to_c_data_interface(&batch);
    
    // Verify FFI struct is valid
    assert!(!ffi.array.is_null());
    assert!(!ffi.schema.is_null());
}
```

```go
func TestArrowImport(t *testing.T) {
    // Create data in Rust
    ffi := C.create_test_batch()
    defer C.release_batch(ffi)
    
    // Import in Go
    batch := arrow.ImportCData(ffi)
    
    // Verify data is accessible
    assert.Equal(t, 100, batch.NumRows())
}

func TestZeroCopy(t *testing.T) {
    // Verify memory addresses match (no copy)
    ffi := C.create_test_batch()
    batch := arrow.ImportCData(ffi)
    
    // Check that Go is reading from Rust memory
    rustPtr := uintptr(ffi.array.buffers[1])
    goPtr := batch.Column(0).Data().Buffers()[1].Address()
    assert.Equal(t, rustPtr, goPtr)
}
```

---

### Milestone 4.2: Complete RESP Commands
**Duration:** 3 days

| Task | Description | Verification |
|------|-------------|--------------|
| 4.2.1 | Implement VSET command | redis-cli test |
| 4.2.2 | Implement VGET command | redis-cli test |
| 4.2.3 | Implement VSEARCH command | redis-cli test |
| 4.2.4 | Implement VDEL command | redis-cli test |
| 4.2.5 | Implement STATS command | JSON response |

**Test Requirements:**
```go
func TestVSetVGet(t *testing.T) {
    conn := connectToServer(t)
    
    // Set a vector
    resp := sendCommand(conn, `VSET key1 "[0.1, 0.2, 0.3]"`)
    assert.Equal(t, "+OK\r\n", resp)
    
    // Get it back
    resp = sendCommand(conn, "VGET key1")
    assert.Contains(t, resp, "0.1")
}

func TestVSearch(t *testing.T) {
    conn := connectToServer(t)
    
    // Insert test vectors
    for i := 0; i < 100; i++ {
        sendCommand(conn, fmt.Sprintf(`VSET vec:%d "[%s]"`, i, randomVector()))
    }
    
    // Search
    resp := sendCommand(conn, `VSEARCH "[0.1, 0.2, 0.3]" 5`)
    results := parseArrayResponse(resp)
    assert.Len(t, results, 5)
}
```

---

## Phase 5: Production Hardening (Weeks 10-12)

### Milestone 5.1: Write Buffer (MemTable)
**Duration:** 3 days

| Task | Description | Verification |
|------|-------------|--------------|
| 5.1.1 | Implement in-memory buffer | Unit test |
| 5.1.2 | Implement threshold-based flush | Flush triggers at 10MB |
| 5.1.3 | Implement WAL for durability | Crash recovery test |
| 5.1.4 | Add concurrent write support | Race detector clean |

---

### Milestone 5.2: Background Compaction
**Duration:** 3 days

| Task | Description | Verification |
|------|-------------|--------------|
| 5.2.1 | Implement file merging logic | Correctness test |
| 5.2.2 | Implement index rebuilding | Search still works |
| 5.2.3 | Add scheduled compaction | Runs on interval |
| 5.2.4 | Add garbage collection | Old files deleted |

---

### Milestone 5.3: Observability
**Duration:** 2 days

| Task | Description | Verification |
|------|-------------|--------------|
| 5.3.1 | Add Prometheus metrics | Metrics endpoint works |
| 5.3.2 | Add structured logging | JSON logs |
| 5.3.3 | Add distributed tracing | Trace propagation |
| 5.3.4 | Add health checks | /health endpoint |

---

### Milestone 5.4: Performance Benchmarks
**Duration:** 2 days

| Task | Description | Verification |
|------|-------------|--------------|
| 5.4.1 | Create benchmark suite | Benchmarks run |
| 5.4.2 | Insert performance baseline | >50k ops/sec |
| 5.4.3 | Search performance baseline | P99 <10ms |
| 5.4.4 | Document performance | README updated |

---

## Testing Strategy Summary

### Unit Tests
- **Coverage Target:** >80%
- **Run:** `cargo test` + `go test ./...`
- **CI:** Every commit

### Integration Tests
- **Coverage:** Cross-component interactions
- **Run:** `make integration-test`
- **CI:** Every PR

### Benchmark Tests
- **Coverage:** Performance-critical paths
- **Run:** `make benchmark`
- **CI:** Weekly + release

### Manual Tests
- **Coverage:** End-to-end flows
- **Checklist:** Before each release

---

## CI/CD Pipeline

```yaml
# Triggered on every push
test:
  - cargo fmt --check
  - cargo clippy -- -D warnings
  - cargo test
  - go fmt ./...
  - go vet ./...
  - go test -race ./...

# Triggered on PR merge to main
integration:
  - docker-compose up -d seaweedfs
  - make integration-test
  - make benchmark

# Triggered on tag
release:
  - Build multi-platform binaries
  - Generate checksums
  - Create GitHub release
```

---

## Documentation Requirements

| Phase | Required Docs |
|-------|---------------|
| Phase 1 | README, CONTRIBUTING, API stubs |
| Phase 2 | Storage layer design doc |
| Phase 3 | Index algorithms doc |
| Phase 4 | FFI interface doc |
| Phase 5 | Operations guide, Performance tuning |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| CGO complexity | Keep FFI surface minimal, use Arrow interface |
| Memory leaks across FFI | Valgrind CI checks, explicit ownership |
| SeaweedFS compatibility | Test matrix with multiple versions |
| SIMD portability | Runtime feature detection, fallback to scalar |

---

*This development plan follows strict TDD principles with small, verifiable milestones. Each phase builds on the previous, ensuring a stable foundation before adding complexity.*
