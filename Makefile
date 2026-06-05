.PHONY: fmt check test audit ci build build-release clean

fmt:
	cargo fmt
	cargo clippy --all-targets --fix --allow-dirty --allow-staged -- -D warnings

check:
	cargo fmt -- --check
	cargo clippy --locked --all-targets -- -D warnings

test:
	cargo test --locked

build:
	cargo build --locked

build-release:
	cargo build --locked --release

audit:
	cargo deny check

ci:
	RUSTFLAGS="-Dwarnings" $(MAKE) --no-print-directory check test

clean:
	cargo clean
