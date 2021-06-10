use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupSet;
use near_sdk::json_types::ValidAccountId;
use near_sdk::{env, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault};

near_sdk::setup_alloc!();

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub approved_accounts: LookupSet<AccountId>,

    pub owner_id: AccountId,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: ValidAccountId) -> Self {
        Self {
            approved_accounts: LookupSet::new(StorageKey::Accounts),
            owner_id: owner_id.into(),
        }
    }

    pub fn is_permissions_contract(&self) -> bool {
        true
    }

    #[allow(unused_variables)]
    pub fn is_approved(&self, account_id: ValidAccountId, sale_id: u64) -> bool {
        self.approved_accounts.contains(account_id.as_ref())
    }

    pub fn approve(&mut self, account_id: ValidAccountId) {
        self.assert_called_by_owner();
        self.approved_accounts.insert(account_id.as_ref());
    }

    pub fn reject(&mut self, account_id: ValidAccountId) {
        self.assert_called_by_owner();
        self.approved_accounts.remove(account_id.as_ref());
    }
}

impl Contract {
    fn assert_called_by_owner(&self) {
        assert_eq!(&self.owner_id, &env::predecessor_account_id());
    }
}
