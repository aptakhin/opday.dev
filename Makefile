testw::
	(cd opday-dev && RUST_LOG=debug RUST_BACKTRACE=1 DATABASE_DSN=postgresql://postgres:postgres@localhost:5432/postgres ../cargo watch -x test)

test::
	RUST_LOG=debug RUST_BACKTRACE=1 DATABASE_DSN=postgresql://postgres:postgres@localhost:5432/postgres ./cargo test --verbose --manifest-path opday-dev/Cargo.toml

build::
	./cargo build --manifest-path opday-dev/Cargo.toml

rund::
	RUST_LOG=debug RUST_BACKTRACE=1 DATABASE_DSN=postgresql://postgres:postgres@localhost:5432/postgres \
	  ./cargo run --manifest-path opday-dev/Cargo.toml -- --port 3003

fmt::
	(cd opday-dev && ../cargo fmt)

fmt-check::
	./cargo fmt --check  --manifest-path opday-dev/Cargo.toml

lint::
	./cargo clippy  --manifest-path opday-dev/Cargo.toml  -- -D warnings
