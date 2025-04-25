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
	@cargo fmt --all --check

clippy:
	@cargo clippy --all-features -- -D warnings

docs:
	@cargo doc --open --all-features &
	@pdflatex -interaction=batchmode report.tex

clean:
	@cargo clean
	@rm -f report.aux report.log report.out report.pdf
	
.PHONY: all build test coverage ext1 check fmt clippy clean