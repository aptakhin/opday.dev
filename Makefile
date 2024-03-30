deploy::
	./opday docker build-push-deploy

deployd::
	RUST_LOG=debug RUST_BACKTRACE=1 ./opday docker build-push-deploy
