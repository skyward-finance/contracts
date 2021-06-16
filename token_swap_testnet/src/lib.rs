use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::{ValidAccountId, WrappedBalance, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, near_bindgen, serde_json, AccountId, Balance, BorshStorageKey, PanicOnDefault,
    PromiseOrValue,
};

near_sdk::setup_alloc!();

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub accounts: UnorderedMap<AccountId, Balance>,

    pub token_account_id: AccountId,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(token_account_id: ValidAccountId) -> Self {
        Self {
            accounts: UnorderedMap::new(StorageKey::Accounts),
            token_account_id: token_account_id.into(),
        }
    }

    pub fn get_balance(&self, account_id: ValidAccountId) -> WrappedBalance {
        self.accounts.get(account_id.as_ref()).unwrap_or(0).into()
    }

    pub fn get_accounts(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<(AccountId, WrappedBalance)> {
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(u64::MAX);
        let keys = self.accounts.keys_as_vector();
        let values = self.accounts.values_as_vector();
        (from_index..std::cmp::min(from_index.saturating_add(limit), keys.len()))
            .map(|index| (keys.get(index).unwrap(), values.get(index).unwrap().into()))
            .collect()
    }
}

impl Contract {
    fn internal_token_deposit(&mut self, account_id: &AccountId, amount: Balance) {
        let current_balance = self.accounts.get(&account_id).unwrap_or(0);
        self.accounts
            .insert(&account_id, &(current_balance + amount));
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum FtOnTransferArgs {
    LinkMainnetAccount { account_id: ValidAccountId },
}

const ERR_FAILED_TO_PARSE_FT_ON_TRANSFER_MSG: &str = "ERR_FAILED_TO_PARSE_FT_ON_TRANSFER_MSG";
const ERR_INVALID_FT_ACCOUNT_ID: &str = "ERR_INVALID_FT_ACCOUNT_ID";

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    #[allow(unused_variables)]
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let args: FtOnTransferArgs =
            serde_json::from_str(&msg).expect(ERR_FAILED_TO_PARSE_FT_ON_TRANSFER_MSG);
        let token_account_id = env::predecessor_account_id();
        assert_eq!(
            &self.token_account_id, &token_account_id,
            "{}",
            ERR_INVALID_FT_ACCOUNT_ID
        );
        match args {
            FtOnTransferArgs::LinkMainnetAccount { account_id } => {
                self.internal_token_deposit(account_id.as_ref(), amount.0);
            }
        }
        PromiseOrValue::Value(0.into())
    }
}
