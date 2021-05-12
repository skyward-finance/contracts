#!/bin/bash
set -e
cd "$(dirname $0)"

[ "$#" -eq 1 ] || die "One Account ID argument required, $# provided"

export ACCOUNT_ID=$1

export SKYWARD_TOKEN_ID=token.$ACCOUNT_ID

near create-account $SKYWARD_TOKEN_ID --masterAccount=$ACCOUNT_ID --initialBalance=3
near deploy $SKYWARD_TOKEN_ID tests/res/fungible_token.wasm new '{"owner_id": "'$ACCOUNT_ID'", "total_supply": "1000000000000000000000000", "metadata": {"spec": "ft-1.0.0", "name": "Test Skyward Finance Token", "symbol": "TEST_SKYWARD", "decimals": 18}}'

export WRAP_NEAR_TOKEN_ID=wrap_near.$ACCOUNT_ID

near create-account $WRAP_NEAR_TOKEN_ID --masterAccount=$ACCOUNT_ID --initialBalance=3
near deploy $WRAP_NEAR_TOKEN_ID tests/res/w_near.wasm new '{}'

near call $WRAP_NEAR_TOKEN_ID --accountId=$ACCOUNT_ID near_deposit '{}' --amount=10

export CONTRACT_ID=app1.$ACCOUNT_ID

near create-account $ACCOUNT_ID --masterAccount=$ACCOUNT_ID --initialBalance=4
near deploy $CONTRACT_ID res/skyward.wasm new '{"skyward_token_id": "'$SKYWARD_TOKEN_ID'", "skyward_total_supply": "1000000000000000000000000", "listing_fee_near": "10000000000000000000000000"}'

near call $SKYWARD_TOKEN_ID --accountId=$ACCOUNT_ID storage_deposit '{"account_id": "'$CONTRACT_ID'"}' --amount=0.00125
near call $WRAP_NEAR_TOKEN_ID --accountId=$ACCOUNT_ID storage_deposit '{"account_id": "'$CONTRACT_ID'"}' --amount=0.00125

export ALICE=alice.$ACCOUNT_ID
near create-account $ALICE --masterAccount=$ACCOUNT_ID --initialBalance=20

export BOB=bob.$ACCOUNT_ID
near create-account $BOB --masterAccount=$ACCOUNT_ID --initialBalance=20

export COBB=cobb.$ACCOUNT_ID
near create-account $COBB --masterAccount=$ACCOUNT_ID --initialBalance=20
