#!/bin/bash
set -e
pushd "$(dirname $0)"
HOST_DIR="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"

echo "Building the docker image"
docker build -t contract-builder .

echo "Building the skyward contract"
docker run \
     --mount type=bind,source=$HOST_DIR,target=/host \
     --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
     -i -t contract-builder \
     host/skyward/build.sh

echo "Comparing to the release"
cmp --silent release/skyward.wasm skyward/res/skyward.wasm || echo "files are different"

