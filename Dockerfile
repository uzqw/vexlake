# syntax=docker/dockerfile:1

# Stage 1: Build Rust library
FROM rust:1.75-bookworm AS rust-builder

WORKDIR /app

# Copy Rust source
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release
RUN cargo build --release

# Stage 2: Build Go binary
FROM golang:1.22-bookworm AS go-builder

WORKDIR /app

# Copy Go source
COPY go.mod go.sum ./
RUN go mod download

COPY cmd ./cmd
COPY pkg ./pkg
COPY internal ./internal

# Copy Rust library from previous stage
COPY --from=rust-builder /app/target/release/libvexlake_core.so /usr/local/lib/

# Build Go binary
ARG VERSION=dev
RUN CGO_ENABLED=0 GOOS=linux go build \
    -ldflags="-s -w -X main.version=${VERSION}" \
    -o /vexlake-server ./cmd/vexlake-server

# Stage 3: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries
COPY --from=go-builder /vexlake-server /usr/local/bin/
COPY --from=rust-builder /app/target/release/libvexlake_core.so /usr/local/lib/

# Set library path
ENV LD_LIBRARY_PATH=/usr/local/lib

# Create non-root user
RUN useradd -r -u 1000 vexlake
USER vexlake

EXPOSE 6379

ENTRYPOINT ["/usr/local/bin/vexlake-server"]
CMD ["-host", "0.0.0.0", "-port", "6379"]
