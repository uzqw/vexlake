# VexLake

[![CI](https://github.com/uzqw/vexlake/actions/workflows/ci.yml/badge.svg)](https://github.com/uzqw/vexlake/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/uzqw/vexlake/branch/main/graph/badge.svg)](https://codecov.io/gh/uzqw/vexlake)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

A cloud-native, serverless vector analytics engine with separation of compute and storage. Built with Go + Rust for maximum performance.

## Architecture

VexLake follows the **"Sandwich Architecture"** - Go handles the network facade, Rust handles all heavy computation:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Go Layer (Network Facade)                    │
│   • RESP protocol (Redis-compatible)                            │
│   • Client connection management                                │
│   • Result formatting (Arrow → RESP)                            │
└───────────────────────────┬─────────────────────────────────────┘
                            │ CGO / FFI (Arrow C Data Interface)
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│              Rust Core (libvector_engine.so)                    │
│   • SIMD-accelerated vector search (AVX-512/NEON)               │
│   • HNSW/IVF index management                                   │
│   • Parquet read/write via DataFusion                           │
│   • S3 I/O via OpenDAL                                          │
└───────────────────────────┬─────────────────────────────────────┘
                            │ S3 API
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                     SeaweedFS Storage                           │
│   • Parquet data files                                          │
│   • Binary index files                                          │
│   • Version metadata                                            │
└─────────────────────────────────────────────────────────────────┘
```

## Features

- **Redis Protocol Compatible**: Connect with any Redis client (redis-cli, redis-py, Jedis)
- **SIMD-Accelerated Search**: 2-10x faster vector computation using AVX-512/NEON
- **Zero-Copy Interface**: Arrow C Data Interface between Go and Rust
- **Cloud-Native Storage**: SeaweedFS backend with S3 API compatibility
- **Separation of Compute and Storage**: Scale compute and storage independently
- **MVCC Transactions**: Snapshot isolation via versioned metadata

## Quick Start

### Prerequisites

- **Rust 1.75+** - [Install](https://rustup.rs/)
- **Go 1.22+** - [Install](https://go.dev/dl/)
- **Make** - For build commands

### Build and Run

```bash
# Build everything
make build

# Run the server
make run

# Run tests
make test

# Run all CI checks locally
make verify
```

### Using Docker

```bash
# Build the image
make docker-build

# Run the container
make docker-run
```

### Connect with redis-cli

```bash
redis-cli -p 6379

# Test connection
127.0.0.1:6379> PING
PONG

# Store a vector
127.0.0.1:6379> VSET vec:1 "[0.1, 0.2, 0.3, 0.4]"
OK

# Retrieve a vector
127.0.0.1:6379> VGET vec:1
"[0.100000, 0.200000, 0.300000, 0.400000]"

# Search for similar vectors
127.0.0.1:6379> VSEARCH "[0.1, 0.2, 0.3, 0.4]" 5
1) "vec:1"
2) "vec:42"
3) "vec:17"
```

## Commands

### Basic Commands

| Command | Description |
|---------|-------------|
| `PING [message]` | Test connection |
| `ECHO message` | Echo back a message |
| `STATS` / `INFO` | Get server statistics |
| `QUIT` | Close connection |

### Vector Commands

| Command | Description |
|---------|-------------|
| `VSET key "[...]"` | Store a vector |
| `VGET key` | Retrieve a vector |
| `VSEARCH "[...]" K` | Find top K similar vectors |
| `VDEL key` | Delete a vector |
| `CLEAR` | Remove all vectors |

## Project Structure

```
vexlake/
├── cmd/
│   ├── vexlake-server/     # Go RESP server
│   └── vexlake-bench/      # Go benchmark tool
├── crates/
│   ├── vexlake-core/       # Rust core library
│   └── vexlake-bench/      # Rust benchmark tool
├── docs/
│   ├── 01_vexlake_design.md        # Architecture design
│   ├── 02_seaweedfs_integration.md # Storage integration
│   └── 03_development_plan.md      # Development roadmap
├── .github/workflows/      # CI/CD
├── Cargo.toml              # Rust workspace
├── go.mod                  # Go module
└── Makefile                # Build commands
```

## Development

### Available Make Targets

```bash
make help           # Show all available commands
make build          # Build Rust library and Go binaries
make test           # Run all tests (Rust + Go)
make fmt            # Format all code
make clippy         # Run Rust linter
make lint           # Run all linters
make verify         # Run all CI checks locally
make doc            # Generate Rust documentation
make clean          # Clean build artifacts
```

### Running Benchmarks

```bash
# Rust vector operation benchmarks
make run-rust-bench

# Go client benchmarks
./bin/vexlake-bench -mode=insert -n=100000 -concurrency=50
./bin/vexlake-bench -mode=search -n=50000 -concurrency=50
```

## Performance

| Metric | Value |
|--------|-------|
| Insert Throughput | >50,000 ops/sec |
| Search Latency (P99) | <10ms |
| TopK Computation | 2-10x faster than Go (SIMD) |
| Memory Transfer | Zero-copy (Arrow C Interface) |

## Documentation

- [Architecture Design](docs/01_vexlake_design.md) - Detailed design specification
- [SeaweedFS Integration](docs/02_seaweedfs_integration.md) - Storage layer details
- [Development Plan](docs/03_development_plan.md) - Roadmap and milestones

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines.

## Acknowledgments

- [DataFusion](https://github.com/apache/datafusion) - Rust SQL execution engine
- [OpenDAL](https://github.com/apache/opendal) - Rust storage abstraction
- [SeaweedFS](https://github.com/seaweedfs/seaweedfs) - Distributed file system
- [Arrow](https://arrow.apache.org/) - Cross-language data format
- [tidwall/redcon](https://github.com/tidwall/redcon) - Go RESP server framework
