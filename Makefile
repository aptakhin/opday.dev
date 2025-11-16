testw::
	(cd opday && RUST_LOG=debug RUST_BACKTRACE=1 cargo watch -x 'cargo test --show-output')

test::
	RUST_LOG=debug RUST_BACKTRACE=1 cargo test --verbose --manifest-path opday-dev/Cargo.toml

build::
	cargo build --manifest-path opday/Cargo.toml

run::
	RUST_LOG=debug RUST_BACKTRACE=1 OPDAY_SECRET_KEY=1 \
	  cargo run --manifest-path opday/Cargo.toml

fmt::
	(cd opday && cargo fmt)

fmt-check::
	cargo fmt --check  --manifest-path opday/Cargo.toml

lint::
	cargo clippy  --manifest-path opday/Cargo.toml  -- -D warnings
