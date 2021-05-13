#!/bin/bash
set -e
pushd "$(dirname $0)"

pushd skyward
cargo test
popd

pushd lockup
cargo test
popd

popd
