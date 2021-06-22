#!/bin/bash
set -e
pushd "$(dirname $0)"

cargo run -- data/lockup.csv data/lockup 1000000000
mkdir -p "../lockup/data"
cp data/lockup*.borsh ../lockup/data/

popd
