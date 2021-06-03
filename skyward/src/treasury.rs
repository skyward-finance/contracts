use crate::*;
use near_sdk::collections::{LazyOption, UnorderedMap};
use near_sdk::json_types::WrappedBalance;
use near_sdk::{assert_one_yocto, Timestamp};

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct VestingIntervalInput {
    pub start_timestamp: TimestampSec,
    pub end_timestamp: TimestampSec,
    pub amount: WrappedBalance,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct VestingInterval {
    pub start_timestamp: Timestamp,
    pub end_timestamp: Timestamp,
    pub amount: Balance,
}

impl From<VestingIntervalInput> for VestingInterval {
    fn from(vs: VestingIntervalInput) -> Self {
        Self {
            start_timestamp: to_nano(vs.start_timestamp),
            end_timestamp: to_nano(vs.end_timestamp),
            amount: vs.amount.into(),
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Treasury {
    pub balances: UnorderedMap<TokenAccountId, Balance>,
    pub skyward_token_id: TokenAccountId,

    pub skyward_burned_amount: Balance,
    pub skyward_vesting_schedule: LazyOption<Vec<VestingInterval>>,

    pub listing_fee_near: Balance,

    pub w_near_token_id: TokenAccountId,
}

impl Treasury {
    pub fn new(
        skyward_token_id: TokenAccountId,
        skyward_vesting_schedule: Vec<VestingIntervalInput>,
        listing_fee_near: Balance,
        w_near_token_id: TokenAccountId,
    ) -> Self {
        assert_ne!(skyward_token_id, w_near_token_id);
        Self {
            balances: UnorderedMap::new(StorageKey::TreasuryBalances),
            skyward_token_id,
            skyward_burned_amount: 0,
            skyward_vesting_schedule: LazyOption::new(
                StorageKey::VestingSchedule,
                Some(
                    &skyward_vesting_schedule
                        .into_iter()
                        .map(|vs| vs.into())
                        .collect(),
                ),
            ),
            listing_fee_near,
            w_near_token_id,
        }
    }

    pub fn internal_deposit(&mut self, token_account_id: &AccountId, amount: Balance) {
        if token_account_id == &self.skyward_token_id {
            env::panic(errors::TREASURY_CAN_NOT_CONTAIN_SKYWARD.as_bytes());
        }
        let balance = self.balances.get(token_account_id).unwrap_or(0);
        let new_balance = balance.checked_add(amount).expect(errors::BALANCE_OVERFLOW);
        self.balances.insert(&token_account_id, &new_balance);
    }

    pub fn internal_withdraw(&mut self, token_account_id: &AccountId, amount: Balance) {
        let balance = self.balances.get(token_account_id).unwrap_or(0);
        let new_balance = balance
            .checked_sub(amount)
            .expect(errors::NOT_ENOUGH_BALANCE);
        self.balances.insert(&token_account_id, &new_balance);
    }

    pub fn internal_donate(&mut self, token_account_id: &AccountId, amount: Balance) {
        if token_account_id == &self.skyward_token_id {
            self.skyward_burned_amount += amount;
        } else {
            self.internal_deposit(token_account_id, amount);
        }
    }
}

#[near_bindgen]
impl Contract {
    pub fn get_treasury_balance(&self, token_account_id: ValidAccountId) -> Option<WrappedBalance> {
        self.treasury
            .balances
            .get(token_account_id.as_ref())
            .map(|a| a.into())
    }

    pub fn get_treasury_balances(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<(TokenAccountId, WrappedBalance)> {
        let keys = self.treasury.balances.keys_as_vector();
        let values = self.treasury.balances.values_as_vector();
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(keys.len());
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| (keys.get(index).unwrap(), values.get(index).unwrap().into()))
            .collect()
    }

    pub fn get_treasury_num_balances(&self) -> u64 {
        self.treasury.balances.len()
    }

    pub fn get_skyward_token_id(self) -> TokenAccountId {
        self.treasury.skyward_token_id
    }

    pub fn get_skyward_circulating_supply(&self) -> WrappedBalance {
        let mut balance = 0;
        let skyward_vesting_schedule = self.treasury.skyward_vesting_schedule.get().unwrap();
        let current_timestamp = env::block_timestamp();
        for vesting_interval in skyward_vesting_schedule {
            balance += if current_timestamp <= vesting_interval.start_timestamp {
                0
            } else if current_timestamp >= vesting_interval.end_timestamp {
                vesting_interval.amount
            } else {
                let total_duration =
                    vesting_interval.end_timestamp - vesting_interval.start_timestamp;
                let passed_duration = current_timestamp - vesting_interval.start_timestamp;
                (U256::from(passed_duration) * U256::from(vesting_interval.amount)
                    / U256::from(total_duration))
                .as_u128()
            };
        }
        (balance - self.treasury.skyward_burned_amount).into()
    }

    pub fn get_listing_fee(&self) -> WrappedBalance {
        self.treasury.listing_fee_near.into()
    }

    #[payable]
    pub fn redeem_skyward(
        &mut self,
        skyward_amount: WrappedBalance,
        token_account_ids: Vec<ValidAccountId>,
    ) {
        assert_one_yocto();
        let skyward_amount: Balance = skyward_amount.into();
        assert!(skyward_amount > 0, "{}", errors::ZERO_SKYWARD);
        let account_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&account_id);
        account.internal_token_withdraw(&self.treasury.skyward_token_id, skyward_amount);
        let numerator = U256::from(skyward_amount);
        let denominator = U256::from(self.get_skyward_circulating_supply().0);
        self.treasury.skyward_burned_amount += skyward_amount;
        for token_account_id in token_account_ids {
            let treasury_balance = self
                .treasury
                .balances
                .get(token_account_id.as_ref())
                .expect(errors::TOKEN_NOT_REGISTERED);
            let amount = (U256::from(treasury_balance) * numerator / denominator).as_u128();
            if amount > 0 {
                let new_balance = treasury_balance
                    .checked_sub(amount)
                    .expect(errors::NOT_ENOUGH_BALANCE);
                self.treasury
                    .balances
                    .insert(token_account_id.as_ref(), &new_balance);
                account.internal_token_deposit(token_account_id.as_ref(), amount);
            }
        }
        self.accounts.insert(&account_id, &account.into());
    }

    pub fn wrap_extra_near(&mut self) -> Promise {
        let unused_near_balance =
            env::account_balance() - Balance::from(env::storage_usage()) * env::storage_byte_cost();
        assert!(
            unused_near_balance > MIN_EXTRA_NEAR,
            "{}",
            errors::NOT_ENOUGH_BALANCE
        );
        let extra_near = unused_near_balance - EXTRA_NEAR;
        Promise::new(self.treasury.w_near_token_id.clone())
            .function_call(
                b"storage_deposit".to_vec(),
                b"{}".to_vec(),
                STORAGE_DEPOSIT,
                STORAGE_DEPOSIT_GAS,
            )
            .function_call(
                b"near_deposit".to_vec(),
                b"{}".to_vec(),
                extra_near,
                NEAR_DEPOSIT_GAS,
            )
            .then(ext_self::after_near_deposit(
                extra_near.into(),
                &env::current_account_id(),
                NO_DEPOSIT,
                AFTER_NEAR_DEPOSIT_GAS,
            ))
    }
}
