testw::
	(cd opday-dev && RUST_LOG=debug RUST_BACKTRACE=1 ../cargo watch -x test)

test::
	RUST_LOG=debug RUST_BACKTRACE=1 ./cargo test --verbose --manifest-path opday-dev/Cargo.toml

build::
	./cargo build --manifest-path opday-dev/Cargo.toml

rund::
	RUST_LOG=debug RUST_BACKTRACE=1 ./cargo run --manifest-path opday-dev/Cargo.toml -- --port 3003

fmt::
	./cargo fmt --manifest-path opday-dev/Cargo.toml

fmt-check::
	./cargo fmt --check  --manifest-path opday-dev/Cargo.toml

lint::
	./cargo clippy  --manifest-path opday-dev/Cargo.toml  -- -D warnings

doc::
	./cargo doc --no-deps  --manifest-path opday-dev/Cargo.toml

docw::
	./cargo watch -x doc --no-deps  --manifest-path opday-dev/Cargo.toml

deploy::
	./opday docker build-push-deploy

deployd::
	RUST_LOG=debug RUST_BACKTRACE=1 ./opday docker build-push-deploy
