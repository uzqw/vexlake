.PHONY: all build build-rust build-go run test test-rust test-go \
        fmt fmt-rust fmt-go clippy lint verify clean help

# Build output directory
BUILD_DIR := bin
RUST_LIB := target/release/libvexlake_core.so

# Rust commands
CARGO := cargo

# Go commands
GOCMD := go
GOBUILD := $(GOCMD) build
GOTEST := $(GOCMD) test
GOFMT := $(GOCMD) fmt
GOVET := $(GOCMD) vet
GOMOD := $(GOCMD) mod

# Version info
VERSION := $(shell git describe --tags --always --dirty 2>/dev/null || echo "dev")
GO_LDFLAGS := -ldflags "-X main.version=$(VERSION)"

# Default target
all: fmt verify build

#
# Build targets
#

build: build-rust build-go

build-rust:
	@echo "Building Rust library..."
	$(CARGO) build --release

build-go: build-rust
	@echo "Building Go binaries..."
	@mkdir -p $(BUILD_DIR)
	$(GOBUILD) $(GO_LDFLAGS) -o $(BUILD_DIR)/vexlake-server ./cmd/vexlake-server
	$(GOBUILD) $(GO_LDFLAGS) -o $(BUILD_DIR)/vexlake-bench ./cmd/vexlake-bench

#
# Run targets
#

run: build-go
	@echo "Starting VexLake server..."
	./$(BUILD_DIR)/vexlake-server

run-rust-bench:
	@echo "Running Rust benchmark..."
	$(CARGO) run --release -p vexlake-bench

#
# Test targets
#

test: test-rust test-go

test-rust:
	@echo "Running Rust tests..."
	$(CARGO) test --all-features

test-go:
	@echo "Running Go tests..."
	$(GOTEST) -v -race ./...

test-coverage:
	@echo "Running Rust tests with coverage..."
	$(CARGO) llvm-cov --all-features --html
	@echo "Coverage report generated: target/llvm-cov/html/index.html"

#
# Format targets
#

fmt: fmt-rust fmt-go

fmt-rust:
	@echo "Formatting Rust code..."
	$(CARGO) fmt

fmt-go:
	@echo "Formatting Go code..."
	$(GOFMT) ./...

#
# Lint targets
#

clippy:
	@echo "Running Rust clippy..."
	$(CARGO) clippy --all-targets --all-features -- -D warnings

lint: clippy
	@echo "Running Go linter..."
	golangci-lint run ./...

vet:
	@echo "Running go vet..."
	$(GOVET) ./...

#
# Verify (CI check locally)
#

verify: fmt-rust fmt-go vet
	@echo "=== Running CI checks locally ==="
	@echo ""
	@echo ">>> Checking Rust formatting..."
	$(CARGO) fmt --all -- --check
	@echo ""
	@echo ">>> Running Rust clippy..."
	$(CARGO) clippy --all-targets --all-features -- -D warnings
	@echo ""
	@echo ">>> Running Rust tests..."
	$(CARGO) test --all-features
	@echo ""
	@echo ">>> Running golangci-lint..."
	golangci-lint run ./...
	@echo ""
	@echo ">>> Running Go tests with race detector..."
	$(GOTEST) -v -race ./...
	@echo ""
	@echo "=== All CI checks passed! ==="

#
# Utility targets
#

tidy:
	@echo "Tidying dependencies..."
	$(GOMOD) tidy

clean:
	@echo "Cleaning..."
	$(CARGO) clean
	rm -rf $(BUILD_DIR)

doc:
	@echo "Generating Rust documentation..."
	$(CARGO) doc --no-deps --open

#
# Docker targets
#

docker-build:
	@echo "Building Docker image..."
	docker build -t vexlake:$(VERSION) .

docker-run: docker-build
	docker run -p 6379:6379 vexlake:$(VERSION)

#
# Help
#

help:
	@echo "VexLake Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  all              - Format, verify, and build everything (default)"
	@echo "  build            - Build Rust library and Go binaries"
	@echo "  build-rust       - Build only the Rust library"
	@echo "  build-go         - Build only the Go binaries"
	@echo "  run              - Run the VexLake server"
	@echo "  run-rust-bench   - Run the Rust benchmark"
	@echo "  test             - Run all tests (Rust + Go)"
	@echo "  test-rust        - Run Rust tests only"
	@echo "  test-go          - Run Go tests only"
	@echo "  test-coverage    - Run tests with coverage report"
	@echo "  fmt              - Format all code (Rust + Go)"
	@echo "  clippy           - Run Rust clippy linter"
	@echo "  lint             - Run all linters"
	@echo "  vet              - Run go vet"
	@echo "  verify           - Run all CI checks locally"
	@echo "  tidy             - Tidy Go modules"
	@echo "  clean            - Clean build artifacts"
	@echo "  doc              - Generate and open Rust documentation"
	@echo "  docker-build     - Build Docker image"
	@echo "  docker-run       - Run Docker container"
	@echo "  help             - Show this help message"
