#!/bin/bash
set -e
cd "$(dirname $0)"

[ "$#" -eq 1 ] || die "One Account ID argument required, $# provided"

ACCOUNT_ID=$1
SKYWARD_TOKEN_ID=token.$ACCOUNT_ID
CONTRACT_ID=$ACCOUNT_ID
WRAP_NEAR_TOKEN_ID=wrap.near

START_TIME=1625097600
near call $CONTRACT_ID --accountId=$CONTRACT_ID sale_create '{"sale": {
    "title": "SKYWARD 25% Initial sale",
    "out_tokens": [{
      "token_account_id": "'$SKYWARD_TOKEN_ID'",
      "balance": "250000000000000000000000",
      "referral_bpt": 100
    }],
    "in_token_account_id": "'$WRAP_NEAR_TOKEN_ID'",
    "start_time": "'$START_TIME'000000000",
    "duration": "604800000000000"
}}'

START_TIME=1627776000
near call $CONTRACT_ID --accountId=$CONTRACT_ID sale_create '{"sale": {
    "title": "SKYWARD 20% August sale",
    "out_tokens": [{
      "token_account_id": "'$SKYWARD_TOKEN_ID'",
      "balance": "200000000000000000000000",
      "referral_bpt": 100
    }],
    "in_token_account_id": "'$WRAP_NEAR_TOKEN_ID'",
    "start_time": "'$START_TIME'000000000",
    "duration": "604800000000000"
}}'

START_TIME=1630454400
near call $CONTRACT_ID --accountId=$CONTRACT_ID sale_create '{"sale": {
    "title": "SKYWARD 15% September sale",
    "out_tokens": [{
      "token_account_id": "'$SKYWARD_TOKEN_ID'",
      "balance": "150000000000000000000000",
      "referral_bpt": 100
    }],
    "in_token_account_id": "'$WRAP_NEAR_TOKEN_ID'",
    "start_time": "'$START_TIME'000000000",
    "duration": "604800000000000"
}}'

START_TIME=1633046400
near call $CONTRACT_ID --accountId=$CONTRACT_ID sale_create '{"sale": {
    "title": "SKYWARD 10% October sale",
    "out_tokens": [{
      "token_account_id": "'$SKYWARD_TOKEN_ID'",
      "balance": "100000000000000000000000",
      "referral_bpt": 100
    }],
    "in_token_account_id": "'$WRAP_NEAR_TOKEN_ID'",
    "start_time": "'$START_TIME'000000000",
    "duration": "604800000000000"
}}'

START_TIME=1635724800
near call $CONTRACT_ID --accountId=$CONTRACT_ID sale_create '{"sale": {
    "title": "SKYWARD 10% November sale",
    "out_tokens": [{
      "token_account_id": "'$SKYWARD_TOKEN_ID'",
      "balance": "100000000000000000000000",
      "referral_bpt": 100
    }],
    "in_token_account_id": "'$WRAP_NEAR_TOKEN_ID'",
    "start_time": "'$START_TIME'000000000",
    "duration": "604800000000000"
}}'

START_TIME=1638316800
near call $CONTRACT_ID --accountId=$CONTRACT_ID sale_create '{"sale": {
    "title": "SKYWARD 10% Final sale",
    "out_tokens": [{
      "token_account_id": "'$SKYWARD_TOKEN_ID'",
      "balance": "100000000000000000000000",
      "referral_bpt": 100
    }],
    "in_token_account_id": "'$WRAP_NEAR_TOKEN_ID'",
    "start_time": "'$START_TIME'000000000",
    "duration": "604800000000000"
}}'
