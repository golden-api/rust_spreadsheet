all: build

build:
	@cargo build --release --features autograder --timings

test:
	@cargo test --features autograder -- --test-threads 1

coverage:
	@cargo tarpaulin --features autograder -- --test-threads 1

ext1:
	@cargo build --release --features gui
	@./target/release/spreadsheet 999 18278

check: fmt clippy

fmt:
	@rustfmt --check src/*.rs

clippy:
	@cargo clippy --release --all-features

docs:
	@cargo doc --open --all-features

clean:
	@cargo clean

.PHONY: all build test coverage ext1 check fmt clippy clean