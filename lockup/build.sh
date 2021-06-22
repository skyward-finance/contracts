#!/bin/bash
set -e
pushd "$(dirname $0)"

for f in data/lockup*.borsh
do
  echo $f
  cp $f data/accounts.borsh
  RUSTFLAGS='-C link-arg=-s' cargo build --target wasm32-unknown-unknown --release
  mkdir -p ./res
  cp target/wasm32-unknown-unknown/release/lockup.wasm ./res/$(basename $f .borsh).wasm
done

popd
