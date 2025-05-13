#!/bin/sh
(cd client/client; cargo build -r)
wasm-bindgen target/wasm32-unknown-unknown/release/client.wasm --out-dir www --target web
(cd client/worker; cargo build -r)
wasm-bindgen target/wasm32-unknown-unknown/release/worker.wasm --out-dir www --target no-modules
cargo build -r --bin server
