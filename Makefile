default:
	cd wasm && cargo-web deploy --release
	cargo build
	cargo build --release
