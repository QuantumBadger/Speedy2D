
build-example-webgl:
	@echo "Building WebGL example code..."
	cd examples/webgl && cargo build --target wasm32-unknown-unknown
	mkdir -p examples/webgl/generated
	wasm-bindgen \
			target/wasm32-unknown-unknown/debug/speedy2d-webgl-hello-world.wasm \
			--out-dir examples/webgl/generated \
			--target web
	cp examples/webgl/index.html examples/webgl/generated
	@echo "Done! Host the contents of examples/webgl/generated/ on a webserver and view index.html."
	@echo "Note: for security reasons, some web browsers may not load the script from a local directory -- a webserver is required."

precommit:
	cargo test
	cargo test --no-default-features --lib --examples --tests
	cargo clippy
	cargo clippy --target wasm32-unknown-unknown
	cargo +nightly fmt -- --check
	cargo doc
	cargo build --target wasm32-unknown-unknown
	cargo build --target wasm32-unknown-unknown --no-default-features
