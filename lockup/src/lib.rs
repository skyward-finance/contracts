use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, WrappedBalance};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, is_promise_success, log, near_bindgen, AccountId, Balance, BorshStorageKey,
    CryptoHash, Gas, PanicOnDefault, Promise, PromiseOrValue, Timestamp,
};
use std::cmp::Ordering;

near_sdk::setup_alloc!();

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
}

pub type TimestampSec = u32;
pub type TokenAccountId = AccountId;

const CRYPTO_HASH_SIZE: usize = 32;
const GAS_FOR_FT_TRANSFER: Gas = 10_000_000_000_000;
const GAS_FOR_AFTER_FT_TRANSFER: Gas = 10_000_000_000_000;
const GAS_FOR_FT_TRANSFER_CALL: Gas = 50_000_000_000_000;
const LOCKUP_DATA: &[u8] = include_bytes!("../data/accounts.borsh");
const SIZE_OF_FIXED_SIZE_ACCOUNT: usize = 60;
const BALANCE_OFFSET: usize = 44;
const NUM_LOCKUP_ACCOUNTS: usize = LOCKUP_DATA.len() / SIZE_OF_FIXED_SIZE_ACCOUNT;

const MAX_STORAGE_PER_ACCOUNT: u64 = 121;
const SELF_STORAGE: u64 = 1000;

const ONE_YOCTO: Balance = 1;
const NO_DEPOSIT: Balance = 0;

uint::construct_uint! {
    pub struct U256(4);
}

#[ext_contract(ext_self)]
trait SelfCallbacks {
    fn after_ft_transfer(&mut self, account_id: AccountId, amount: WrappedBalance) -> bool;
    fn after_donation(&mut self, amount: WrappedBalance);
}

trait SelfCallbacks {
    fn after_ft_transfer(&mut self, account_id: AccountId, amount: WrappedBalance) -> bool;
    fn after_donation(&mut self, amount: WrappedBalance);
}

#[derive(BorshDeserialize)]
pub struct FixedSizeAccount {
    pub account_hash: CryptoHash,
    pub start_timestamp: TimestampSec,
    pub cliff_timestamp: TimestampSec,
    pub end_timestamp: TimestampSec,
    pub balance: u128,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Account {
    pub index: u32,
    pub claimed_balance: Balance,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct AccountOutput {
    pub start_timestamp: TimestampSec,
    pub cliff_timestamp: TimestampSec,
    pub end_timestamp: TimestampSec,
    pub balance: WrappedBalance,
    pub claimed_balance: WrappedBalance,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub accounts: LookupMap<AccountId, Account>,

    pub token_account_id: TokenAccountId,

    pub skyward_account_id: AccountId,

    pub claim_expiration_timestamp: TimestampSec,

    pub total_balance: Balance,

    pub untouched_balance: Balance,

    pub total_claimed: Balance,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct Stats {
    pub token_account_id: TokenAccountId,

    pub skyward_account_id: AccountId,

    pub claim_expiration_timestamp: TimestampSec,

    pub total_balance: WrappedBalance,

    pub untouched_balance: WrappedBalance,

    pub total_claimed: WrappedBalance,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(
        token_account_id: ValidAccountId,
        skyward_account_id: ValidAccountId,
        claim_expiration_timestamp: TimestampSec,
    ) -> Self {
        let total_balance = compute_total_balance();
        let required_storage_cost = Balance::from(
            env::storage_usage()
                + SELF_STORAGE
                + MAX_STORAGE_PER_ACCOUNT * (NUM_LOCKUP_ACCOUNTS as u64),
        ) * env::storage_byte_cost();
        assert!(env::account_balance() >= required_storage_cost);
        Self {
            accounts: LookupMap::new(StorageKey::Accounts),
            token_account_id: token_account_id.into(),
            skyward_account_id: skyward_account_id.into(),
            claim_expiration_timestamp,
            total_balance,
            untouched_balance: total_balance,
            total_claimed: 0,
        }
    }

    pub fn get_account(&self, account_id: ValidAccountId) -> Option<AccountOutput> {
        self.internal_get_account(account_id.as_ref())
            .map(|(account, fixed_size_account)| AccountOutput {
                start_timestamp: fixed_size_account.start_timestamp,
                cliff_timestamp: fixed_size_account.cliff_timestamp,
                end_timestamp: fixed_size_account.end_timestamp,
                balance: fixed_size_account.balance.into(),
                claimed_balance: account.claimed_balance.into(),
            })
    }

    pub fn claim(&mut self) -> PromiseOrValue<bool> {
        let account_id = env::predecessor_account_id();
        let (
            mut account,
            FixedSizeAccount {
                start_timestamp,
                cliff_timestamp,
                end_timestamp,
                balance,
                ..
            },
        ) = self
            .internal_get_account(&account_id)
            .expect("The claim is not found");
        let current_timestamp = env::block_timestamp();
        let unlocked_balance: Balance = if current_timestamp < to_nano(cliff_timestamp) {
            0
        } else if current_timestamp >= to_nano(end_timestamp) {
            balance
        } else {
            let total_duration = to_nano(end_timestamp - start_timestamp);
            let passed_duration = current_timestamp - to_nano(start_timestamp);
            (U256::from(passed_duration) * U256::from(balance) / U256::from(total_duration))
                .as_u128()
        };
        let claim_balance = unlocked_balance - account.claimed_balance;
        account.claimed_balance = unlocked_balance;
        self.total_claimed += claim_balance;
        if self.accounts.insert(&account_id, &account).is_none() {
            // New claim, have to remove this from untouched balance.
            self.untouched_balance -= balance;
        }
        if claim_balance > 0 {
            ext_fungible_token::ft_transfer(
                account_id.clone(),
                claim_balance.into(),
                Some(format!(
                    "Claiming unlocked {} balance from {}",
                    claim_balance,
                    env::current_account_id()
                )),
                &self.token_account_id,
                ONE_YOCTO,
                GAS_FOR_FT_TRANSFER,
            )
            .then(ext_self::after_ft_transfer(
                account_id,
                claim_balance.into(),
                &env::current_account_id(),
                NO_DEPOSIT,
                GAS_FOR_AFTER_FT_TRANSFER,
            ))
            .into()
        } else {
            PromiseOrValue::Value(true)
        }
    }

    pub fn get_stats(&self) -> Stats {
        Stats {
            token_account_id: self.token_account_id.clone(),
            skyward_account_id: self.skyward_account_id.clone(),
            claim_expiration_timestamp: self.claim_expiration_timestamp,
            total_balance: self.total_balance.into(),
            untouched_balance: self.untouched_balance.into(),
            total_claimed: self.total_claimed.into(),
        }
    }

    pub fn donate_to_treasury(&mut self) -> Promise {
        assert!(
            env::block_timestamp() >= to_nano(self.claim_expiration_timestamp),
            "The claims are not expired yet"
        );
        if self.untouched_balance > 0 {
            let message = format!(
                "Donating remaining {} untouched token balance from {} to Skyward treasury",
                self.untouched_balance,
                env::current_account_id()
            );
            log!(message);
            ext_fungible_token::ft_transfer_call(
                self.skyward_account_id.clone(),
                self.untouched_balance.into(),
                Some(message),
                "\"DonateToTreasury\"".to_string(),
                &self.token_account_id,
                ONE_YOCTO,
                GAS_FOR_FT_TRANSFER_CALL,
            ).then(ext_self::after_donation(
                self.untouched_balance.into(),
                &env::current_account_id(),
                NO_DEPOSIT,
                GAS_FOR_AFTER_FT_TRANSFER,
            ));
            self.total_balance -= self.untouched_balance;
            self.untouched_balance = 0;
        }
        let unused_near_balance =
            env::account_balance() - Balance::from(env::storage_usage()) * env::storage_byte_cost();
        log!("Donating {} NEAR to Skyward", unused_near_balance);
        Promise::new(self.skyward_account_id.clone()).transfer(unused_near_balance)
    }

    fn internal_get_account(&self, account_id: &AccountId) -> Option<(Account, FixedSizeAccount)> {
        self.accounts
            .get(account_id)
            .map(|account| {
                let fixed_size_account = get_fixed_size_account(account.index as usize);
                (account, fixed_size_account)
            })
            .or_else(|| {
                if env::block_timestamp() < to_nano(self.claim_expiration_timestamp) {
                    if let Some(index) = find_account(&account_id) {
                        return Some((
                            Account {
                                index: index as u32,
                                claimed_balance: 0,
                            },
                            get_fixed_size_account(index),
                        ));
                    }
                }
                None
            })
    }
}

#[near_bindgen]
impl SelfCallbacks for Contract {
    #[private]
    fn after_ft_transfer(&mut self, account_id: AccountId, amount: WrappedBalance) -> bool {
        let promise_success = is_promise_success();
        if !promise_success {
            let mut account = self
                .accounts
                .get(&account_id)
                .expect("The claim is not found");
            account.claimed_balance -= amount.0;
            self.total_claimed -= amount.0;
            self.accounts.insert(&account_id, &account);
        }
        promise_success
    }

    #[private]
    fn after_donation(&mut self, amount: WrappedBalance) {
        if !is_promise_success() {
            let amount: Balance = amount.into();
            self.total_balance += amount;
            self.untouched_balance = amount;
            log!("Donating failed, counting {} back to untouched", amount);
        }
    }
}

fn hash_account(account_id: &AccountId) -> CryptoHash {
    let value_hash = env::sha256(account_id.as_bytes());
    let mut res = CryptoHash::default();
    res.copy_from_slice(&value_hash);

    res
}

fn find_account(expected_account_id: &AccountId) -> Option<usize> {
    let expected_account_hash = hash_account(expected_account_id);
    // Less or equal to expected_account_hash (inclusive)
    let mut left = 0;
    // Strictly greater than expected_account_hash
    let mut right = NUM_LOCKUP_ACCOUNTS;
    while left < right {
        let mid = (left + right) / 2;
        let account_hash = get_account_hash_at(mid);
        match expected_account_hash.cmp(&account_hash) {
            Ordering::Less => right = mid,
            Ordering::Equal => return Some(mid),
            Ordering::Greater => left = mid + 1,
        }
    }
    None
}

fn get_account_hash_at(index: usize) -> CryptoHash {
    let offset = index * SIZE_OF_FIXED_SIZE_ACCOUNT;
    let mut res = CryptoHash::default();
    res.copy_from_slice(&LOCKUP_DATA[offset..offset + CRYPTO_HASH_SIZE]);
    res
}

fn get_fixed_size_account(index: usize) -> FixedSizeAccount {
    FixedSizeAccount::try_from_slice(
        &LOCKUP_DATA
            [(index * SIZE_OF_FIXED_SIZE_ACCOUNT)..((index + 1) * SIZE_OF_FIXED_SIZE_ACCOUNT)],
    )
    .unwrap()
}

fn get_fixed_size_account_balance(index: usize) -> Balance {
    Balance::try_from_slice(
        &LOCKUP_DATA[(index * SIZE_OF_FIXED_SIZE_ACCOUNT + BALANCE_OFFSET)
            ..((index + 1) * SIZE_OF_FIXED_SIZE_ACCOUNT)],
    )
    .unwrap()
}

fn to_nano(timestamp: TimestampSec) -> Timestamp {
    Timestamp::from(timestamp) * 10u64.pow(9)
}

fn compute_total_balance() -> Balance {
    (0..NUM_LOCKUP_ACCOUNTS)
        .map(|index| get_fixed_size_account_balance(index))
        .sum()
}
