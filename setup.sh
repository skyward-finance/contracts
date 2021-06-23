#!/bin/bash
set -e
cd "$(dirname $0)"

[ "$#" -eq 1 ] || die "One Account ID argument required, $# provided"

export ACCOUNT_ID=$1
ONE_YOCTO=0.000000000000000000000001

export SKYWARD_TOKEN_ID=token.$ACCOUNT_ID
export CONTRACT_ID=$ACCOUNT_ID

near create-account $SKYWARD_TOKEN_ID --masterAccount=$ACCOUNT_ID --initialBalance=3
near deploy $SKYWARD_TOKEN_ID common/fungible_token.wasm new '{"owner_id": "'$ACCOUNT_ID'", "total_supply": "1000000000000000000000000", "metadata": {"spec": "ft-1.0.0", "name": "Skyward Finance Token", "symbol": "SKYWARD", "icon": "data:image/png;base64,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", "decimals": 18}}'

export WRAP_NEAR_TOKEN_ID=wrap.near

declare -a LOCKUP_BALANCES=("5314188080644000000000" "25312260092366000000000" "17491819666921000000000" "51881732160069000000000")

# use for loop to read all values and indexes
for (( i=0; i<4; i++ ));
do
  LOCKUP_BALANCE=${LOCKUP_BALANCES[$i]}
  LOCKUP_ACCOUNT_ID=lockup$i.$ACCOUNT_ID
  echo "Lockup ${LOCKUP_ACCOUNT_ID} with balance ${LOCKUP_BALANCE}"
  near create-account $LOCKUP_ACCOUNT_ID --masterAccount=$ACCOUNT_ID --initialBalance=20
  near deploy $LOCKUP_ACCOUNT_ID release/lockup$i.wasm new '{
    "token_account_id": "'$SKYWARD_TOKEN_ID'",
    "skyward_account_id": "'$CONTRACT_ID'",
    "claim_expiration_timestamp": 1656633600
  }' --initGas=200000000000000
  near call $SKYWARD_TOKEN_ID --accountId=$ACCOUNT_ID storage_deposit '{"account_id": "'$LOCKUP_ACCOUNT_ID'"}' --amount=0.00125
  near call $WRAP_NEAR_TOKEN_ID --accountId=$ACCOUNT_ID storage_deposit '{"account_id": "'$LOCKUP_ACCOUNT_ID'"}' --amount=0.00125
  near call $SKYWARD_TOKEN_ID --accountId=$ACCOUNT_ID ft_transfer '{"receiver_id": "'$LOCKUP_ACCOUNT_ID'", "amount": "'$LOCKUP_BALANCE'"}' --amount=$ONE_YOCTO

  near view $LOCKUP_ACCOUNT_ID get_stats
done

# 2021-08-01 = 1627776000
# 2021-09-01 = 1630454400
# 2022-01-01 = 1640995200
# 2022-07-01 = 1656633600

near deploy $CONTRACT_ID release/skyward.wasm new '{
  "skyward_token_id": "'$SKYWARD_TOKEN_ID'",
  "skyward_vesting_schedule": [{
    "start_timestamp": 1627776000,
    "end_timestamp": 1630454400,
    "amount": "10000000000000000000000"
  }, {
    "start_timestamp": 1640995200,
    "end_timestamp": 1656633600,
    "amount": "90000000000000000000000"
  }],
  "listing_fee_near": "10000000000000000000000000",
  "w_near_token_id": "'$WRAP_NEAR_TOKEN_ID'"
}'

near call $SKYWARD_TOKEN_ID --accountId=$ACCOUNT_ID storage_deposit '{"account_id": "'$CONTRACT_ID'"}' --amount=0.00125
near call $WRAP_NEAR_TOKEN_ID --accountId=$ACCOUNT_ID storage_deposit '{"account_id": "'$CONTRACT_ID'"}' --amount=0.00125
