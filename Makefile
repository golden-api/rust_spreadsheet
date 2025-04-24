all: build

build:
	@cargo build --release --timings

test:
	@cargo test -- --test-threads 1

coverage:
	@cargo tarpaulin

ext1:
	@cargo build --release --features gui
	@./target/release/spreadsheet 999 18278

check: fmt clippy

fmt:
	@rustfmt --check src/*.rs

clippy:
	@cargo clippy --release
	@cargo clippy --release --features gui

docs:
	@cargo doc --open

clean:
	@cargo clean

.PHONY: all build test coverage ext1 check fmt clippy clean