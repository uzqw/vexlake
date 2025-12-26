# Contributing to VexLake

Thank you for your interest in contributing to VexLake! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- **Rust 1.75+** - [Install via rustup](https://rustup.rs/)
- **Go 1.22+** - [Download](https://go.dev/dl/)
- **Make** - For build commands

### Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/vexlake.git
   cd vexlake
   ```
3. Add the upstream remote:
   ```bash
   git remote add upstream https://github.com/uzqw/vexlake.git
   ```

## Development Workflow

### Building

```bash
# Build everything (Rust library + Go binaries)
make build

# Build only Rust
make build-rust

# Build only Go
make build-go
```

### Running Tests

```bash
# Run all tests
make test

# Run only Rust tests
make test-rust

# Run only Go tests
make test-go
```

### Formatting and Linting

```bash
# Format all code
make fmt

# Run Rust clippy
make clippy

# Run Go linter
make lint
```

### Running All CI Checks Locally

Before submitting a pull request, **you must run all verification checks locally**:

```bash
make verify
```

This command runs:
- `cargo fmt --check` - Rust formatting check
- `cargo clippy` - Rust linter
- `cargo test` - Rust tests
- `go vet` - Go static analysis
- `go test -race` - Go tests with race detector

## Pull Request Process

1. Create a new branch for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. Make your changes and commit them with clear, descriptive messages

3. **Run all verification checks locally before submitting**:
   ```bash
   make verify
   ```
   Your PR will not be merged if CI checks fail.

4. Push to your fork and create a Pull Request

5. Ensure the PR description clearly describes the problem and solution

## Code Style

### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Run `cargo fmt` before committing
- Ensure `cargo clippy` passes with no warnings
- Add documentation for public APIs
- Add tests for new functionality

### Go

- Follow [Effective Go](https://golang.org/doc/effective_go) guidelines
- Run `go fmt` before committing
- Ensure `go vet` passes
- Add tests for new functionality
- Keep functions small and focused

## Commit Messages

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <description>

[optional body]

[optional footer(s)]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Formatting, missing semicolons, etc.
- `refactor`: Code restructuring without changing behavior
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Build process or auxiliary tool changes

Examples:
```
feat(index): add HNSW index implementation
fix(storage): handle S3 connection timeout
docs(readme): add benchmarking instructions
```

## Reporting Bugs

When reporting bugs, please include:

- A clear and descriptive title
- Steps to reproduce the issue
- Expected behavior
- Actual behavior
- Rust version (`rustc --version`)
- Go version (`go version`)
- Operating system and version

## Feature Requests

Feature requests are welcome! Please provide:

- A clear description of the feature
- The motivation or use case
- Any relevant examples or references

## Architecture

VexLake uses a "Sandwich Architecture":

- **Go Layer**: Network protocol (RESP), client management
- **Rust Core**: Computation, storage I/O, indexing
- **Storage**: SeaweedFS via S3 API

When contributing, understand which layer your changes affect and follow the appropriate guidelines.

## Code of Conduct

Please note that this project follows a [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## License

By contributing to VexLake, you agree that your contributions will be licensed under the Apache License 2.0.
