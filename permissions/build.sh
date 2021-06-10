#!/bin/bash
set -e
pushd "$(dirname $0)"

RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
mkdir -p ./res
cp target/wasm32-unknown-unknown/release/permissions.wasm ./res/

popd
