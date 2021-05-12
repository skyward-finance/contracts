## Build and Init

```bash
./build.sh
near dev-deploy res/skyward.was
export CONTRACT_ID=skyward.testnet
export SKYWARD_TOKEN_ID=token.skyward.testnet

near call $CONTRACT_ID --accountId=$CONTRACT_ID new '{"skyward_token_id": "'$SKYWARD_TOKEN_ID'", "skyward_total_supply": "1000000000000000000000000", "listing_fee_near": "10000000000000000000000000"}'
```

## Register tokens for ACCOUNT_ID

```bash
export TOKEN1=token1.testnet
export TOKEN2=token2.testnet
export ACCOUNT_ID=account1.testnet
export ACCOUNT_ID2=account2.testnet

# Init tokens
# near call $TOKEN1 --accountId=$ACCOUNT_ID new_default_meta '{"owner_id": "'$ACCOUNT_ID'", "total_supply": "1000000000000000000000000000000000"}'
# near call $TOKEN2 --accountId=$ACCOUNT_ID2 new_default_meta '{"owner_id": "'$ACCOUNT_ID2'", "total_supply": "1000000000000000000000000000000000"}'

# Register both tokens for $ACCOUNT_ID (even though only TOKEN1) is needed now
near call $CONTRACT_ID --accountId=$ACCOUNT_ID register_tokens '{"token_account_ids": ["'$TOKEN1'", "'$TOKEN2'"]}' --amount=0.01

near view $CONTRACT_ID get_account_balances '{"account_id": "'$ACCOUNT_ID'"}'
```

## Register contract with tokens

```bash
near call $TOKEN1 --accountId=$ACCOUNT_ID storage_deposit '{"account_id": "'$CONTRACT_ID'"}' --amount=0.00125
near call $TOKEN2 --accountId=$ACCOUNT_ID storage_deposit '{"account_id": "'$CONTRACT_ID'"}' --amount=0.00125
```

## Deposit TOKEN1

```bash
export AMOUNT=1000000000000000000000000000000
near call $TOKEN1 --accountId=$ACCOUNT_ID ft_transfer_call '{"receiver_id": "'$CONTRACT_ID'", "amount": "'$AMOUNT'", "memo": "Yolo for sale", "msg": "\"AccountDeposit\""}' --amount=0.000000000000000000000001
near view $CONTRACT_ID get_account_balances '{"account_id": "'$ACCOUNT_ID'"}'
```

## Register 2nd account with contract
```bash
near call $CONTRACT_ID --accountId=$ACCOUNT_ID2 register_token '{"token_account_id": "'$TOKEN2'"}' --amount=0.01
```

## Deposit TOKEN2 from 2nd account to contract
```bash
export AMOUNT=1000000000000000000000000000000
near call $TOKEN2 --accountId=$ACCOUNT_ID2 ft_transfer_call '{"receiver_id": "'$CONTRACT_ID'", "amount": "'$AMOUNT'", "memo": "BUY BUY BUY", "msg": "\"AccountDeposit\""}' --amount=0.000000000000000000000001
near view $CONTRACT_ID get_account_balances '{"account_id": "'$ACCOUNT_ID2'"}'
```

## Creating sale
```bash
# Mac + fish
# echo (date -v +10M '+%s')"000000000"
export START_TIMESTAMP=1600000000000000000
export SALE_AMOUNT=500000000000000000000000000000
near call $CONTRACT_ID --accountId=$ACCOUNT_ID sale_create '{"sale": {"out_token_account_id": "'$TOKEN1'", "out_token_balance": "'$SALE_AMOUNT'", "in_token_account_id": "'$TOKEN2'", "start_time": "'$START_TIMESTAMP'", "duration": "3600000000000"}}' --amount=0.1

near view $CONTRACT_ID get_sales
````

## Joining SALE

```bash
export AMOUNT=1000000000000000000000000000
export SALE_ID=0
near call $CONTRACT_ID --accountId=$ACCOUNT_ID2 sale_deposit_in_token '{"sale_id": '$SALE_ID', "amount": "'$AMOUNT'"}' --amount=0.01

near view $CONTRACT_ID get_sales '{"account_id": "'$ACCOUNT_ID2'"}'
```

## Claiming from SALE

```bash
near call $CONTRACT_ID --accountId=$ACCOUNT_ID2 sale_claim_out_tokens '{"sale_id": '$SALE_ID'}'

near view $CONTRACT_ID get_sales '{"account_id": "'$ACCOUNT_ID2'"}'
```
