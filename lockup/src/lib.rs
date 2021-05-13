use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, WrappedBalance};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, log, near_bindgen, AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault, Promise,
    Timestamp,
};

near_sdk::setup_alloc!();

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
}

pub type TimestampSec = u32;
pub type TokenAccountId = AccountId;

const GAS_FOR_FT_TRANSFER: Gas = 10_000_000_000_000;
const GAS_FOR_FT_TRANSFER_CALL: Gas = 50_000_000_000_000;
const LOCKUP_DATA: &[u8] = include_bytes!("../data/accounts.borsh");
const SIZE_OF_FIXED_SIZE_ACCOUNT: usize = 93;
const NUM_LOCKUP_ACCOUNTS: usize = LOCKUP_DATA.len() / SIZE_OF_FIXED_SIZE_ACCOUNT;

const MAX_STORAGE_PER_ACCOUNT: u64 = 137;
const SELF_STORAGE: u64 = 1000;

const ONE_YOCTO: Balance = 1;

uint::construct_uint! {
    pub struct U256(4);
}

#[derive(BorshDeserialize)]
pub struct FixedSizeAccount {
    pub account_len: u8,
    pub account_id: [u8; 64],
    pub start_timestamp: TimestampSec,
    pub cliff_timestamp: TimestampSec,
    pub end_timestamp: TimestampSec,
    pub balance: u128,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct Account {
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
        total_balance: WrappedBalance,
    ) -> Self {
        let total_balance = total_balance.into();
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

    pub fn get_account(
        &self,
        account_id: ValidAccountId,
        lockup_index: Option<u32>,
    ) -> Option<Account> {
        self.internal_get_accounts(account_id.as_ref(), lockup_index)
    }

    pub fn claim(&mut self, lockup_index: Option<u32>) {
        let account_id = env::predecessor_account_id();
        let mut account = self
            .internal_get_accounts(&account_id, lockup_index)
            .expect("The claim is not found");
        let current_timestamp = env::block_timestamp();
        let unlocked_balance: Balance = if current_timestamp < to_nano(account.cliff_timestamp) {
            0
        } else if current_timestamp >= to_nano(account.end_timestamp) {
            account.balance.0
        } else {
            let total_duration = to_nano(account.end_timestamp - account.start_timestamp);
            let passed_duration = current_timestamp - to_nano(account.start_timestamp);
            (U256::from(passed_duration) * U256::from(account.balance.0)
                / U256::from(total_duration))
            .as_u128()
        };
        let claim_balance = unlocked_balance - account.claimed_balance.0;
        account.claimed_balance = unlocked_balance.into();
        self.total_claimed += claim_balance;
        if self.accounts.insert(&account_id, &account).is_none() {
            // New claim, have to remove this from untouched balance.
            self.untouched_balance -= account.balance.0;
        }
        if claim_balance > 0 {
            ext_fungible_token::ft_transfer(
                account_id,
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
            .as_return();
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
            );
            self.total_balance -= self.untouched_balance;
            self.untouched_balance = 0;
        }
        let unused_near_balance =
            env::account_balance() - Balance::from(env::storage_usage()) * env::storage_byte_cost();
        log!("Donating {} NEAR to Skyward", unused_near_balance);
        Promise::new(self.skyward_account_id.clone()).transfer(unused_near_balance)
    }

    fn internal_get_accounts(
        &self,
        account_id: &AccountId,
        lockup_index: Option<u32>,
    ) -> Option<Account> {
        self.accounts.get(account_id).or_else(|| {
            if env::block_timestamp() < to_nano(self.claim_expiration_timestamp) {
                if let Some(lockup_index) = lockup_index {
                    return Some(parse_lockup_account(account_id, lockup_index as usize));
                }
            }
            None
        })
    }
}

fn parse_lockup_account(expected_account_id: &AccountId, lockup_index: usize) -> Account {
    assert!(lockup_index < NUM_LOCKUP_ACCOUNTS, "Invalid lockup index");
    let FixedSizeAccount {
        account_len,
        account_id,
        start_timestamp,
        cliff_timestamp,
        end_timestamp,
        balance,
    } = FixedSizeAccount::try_from_slice(
        &LOCKUP_DATA[(lockup_index * SIZE_OF_FIXED_SIZE_ACCOUNT)
            ..((lockup_index + 1) * SIZE_OF_FIXED_SIZE_ACCOUNT)],
    )
    .unwrap();
    let account_id = AccountId::from_utf8(account_id[..account_len as usize].to_vec()).unwrap();
    assert_eq!(expected_account_id, &account_id);
    Account {
        start_timestamp,
        cliff_timestamp,
        end_timestamp,
        balance: balance.into(),
        claimed_balance: 0.into(),
    }
}

fn to_nano(timestamp: TimestampSec) -> Timestamp {
    Timestamp::from(timestamp) * 10u64.pow(9)
}
