#!/bin/bash

cargo build --release --target wasm32-unknown-unknown --package swap_backend

candid-extractor ./target/wasm32-unknown-unknown/release/swap_backend.wasm > canisters/swap_backend/swap_backend.did || true

cargo build --release --target wasm32-unknown-unknown --package ord_canister

candid-extractor ./target/wasm32-unknown-unknown/release/ord_canister.wasm > canisters/ord_canister/ord_indexer.did || true
