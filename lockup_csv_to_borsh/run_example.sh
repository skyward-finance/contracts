#!/bin/bash
set -e
pushd "$(dirname $0)"

cargo run -- example.csv example_out.borsh 1000000000000000
mkdir -p "../lockup/data"
cp example_out.borsh ../lockup/data/

popd
