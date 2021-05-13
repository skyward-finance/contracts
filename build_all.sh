#!/bin/bash
set -e
pushd "$(dirname $0)"

skyward/build.sh
lockup_csv_to_borsh/run_example.sh
lockup/build.sh

popd
