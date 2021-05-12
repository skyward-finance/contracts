pub mod account;
pub(crate) mod errors;
mod internal;
pub mod sale;
pub mod sub;
pub mod treasury;
pub(crate) mod utils;

pub use crate::account::*;
pub use crate::internal::*;
pub use crate::sale::*;
pub use crate::sub::*;
pub use crate::treasury::*;
pub(crate) use crate::utils::*;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::{ValidAccountId, WrappedBalance};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, log, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault,
    Promise, StorageUsage,
};

near_sdk::setup_alloc!();

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
    AccountTokens { account_id: AccountId },
    AccountSubs { account_id: AccountId },
    AccountSales { account_id: AccountId },
    Sales,
    TreasuryBalances,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub accounts: LookupMap<AccountId, VAccount>,

    pub sales: LookupMap<u64, VSale>,

    pub num_sales: u64,

    pub treasury: Treasury,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(
        skyward_token_id: ValidAccountId,
        skyward_total_supply: WrappedBalance,
        listing_fee_near: WrappedBalance,
    ) -> Self {
        Self {
            accounts: LookupMap::new(StorageKey::Accounts),
            sales: LookupMap::new(StorageKey::Sales),
            num_sales: 0,
            treasury: Treasury::new(
                skyward_token_id.into(),
                skyward_total_supply.0,
                listing_fee_near.0,
            ),
        }
    }
}
