#!/bin/bash
set -e
pushd "$(dirname $0)"

mkdir -p ./data
cp ./example.csv ./data/lockup.csv
./run.sh

popd
