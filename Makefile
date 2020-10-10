default:
	cd wasm && yarn webpack
	cargo build --release -p pyckitup
