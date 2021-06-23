#!/bin/bash
set -e

cd "$(dirname $0)"

[ "$#" -eq 5 ] || die "Skyward account ID, Owner Id, Token Id Want, Token Id Sold, Sold amount argument required, $# provided"

ACCOUNT_ID=$1

CONTRACT_ID=$ACCOUNT_ID

OWNER_ID=$2
TOKEN_ACCOUNT_ID_IN=$3
TOKEN_ACCOUNT_ID_OUT=$4
AMOUNT=$5

near call $TOKEN_ACCOUNT_ID_OUT --accountId=$OWNER_ID storage_deposit '{"account_id": "'$CONTRACT_ID'"}' --amount=0.00125
near call $CONTRACT_ID --accountId=$OWNER_ID register_token '{"token_account_id": "'$TOKEN_ACCOUNT_ID_OUT'"}' --amount=0.01
near call $TOKEN_ACCOUNT_ID_OUT --accountId=$OWNER_ID ft_transfer_call '{"receiver_id": "'$CONTRACT_ID'", "amount": "'$AMOUNT'", "msg":"\"AccountDeposit\""}' --amount=0.000000000000000000000001

TIME=$(date +%s)
START_TIME=$(expr $TIME + 604805)
near call $CONTRACT_ID --accountId=$OWNER_ID sale_create '{"sale": {
    "title": "[TESTNET] Custom token sale",
    "out_tokens": [{
      "token_account_id": "'$TOKEN_ACCOUNT_ID_OUT'",
      "balance": "'$AMOUNT'",
      "referral_bpt": 100
    }],
    "in_token_account_id": "'$TOKEN_ACCOUNT_ID_IN'",
    "start_time": "'$START_TIME'000000000",
    "duration": "604800000000000"
}}' --amount=11
