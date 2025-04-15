# Default target
all: build

# Build the project in release mode with optimization flags (defined in Cargo.toml)
build:
	@cargo build --release --timings

# Run tests
test:
	@cargo test --release

# Check the codebase (clippy and formatting)
check: fmt clippy

# Format the code
fmt:
	@cargo fmt -- --check

# Run clippy for linting
clippy:
	@cargo clippy --release -- -D warnings

# Clean the project
clean:
	@cargo clean

.PHONY: all build run test check fmt clippy clean help