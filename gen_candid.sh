#!/bin/bash

cargo build --release --target wasm32-unknown-unknown -p swap_backend
candid-extractor target/wasm32-unknown-unknown/release/swap_backend.wasm > canisters/swap_backend/swap_backend.did || true
