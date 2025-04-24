# Default target
all: build

# Build the project in release mode with optimization flags (defined in Cargo.toml)
build:
	@cargo build --release --timings

test:
	@cargo test -- --test-threads 1

coverage:
	@cargo tarpaulin

# Run tests
ext1:
	@cargo build --release --features gui
	@./target/release/spreadsheet 999 18278

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