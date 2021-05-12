use crate::*;
use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_sdk::is_promise_success;
use near_sdk::json_types::WrappedBalance;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum FtOnTransferArgs {
    AccountDeposit,
}

#[ext_contract(ext_self)]
trait SelfCallbacks {
    fn after_ft_transfer(
        &mut self,
        account_id: AccountId,
        token_account_id: AccountId,
        amount: WrappedBalance,
    );
}

trait SelfCallbacks {
    fn after_ft_transfer(
        &mut self,
        account_id: AccountId,
        token_account_id: AccountId,
        amount: WrappedBalance,
    );
}

impl Contract {
    pub fn internal_ft_transfer(
        &mut self,
        account_id: &AccountId,
        token_account_id: &AccountId,
        amount: Balance,
    ) -> Promise {
        ext_fungible_token::ft_transfer(
            account_id.clone(),
            amount.into(),
            None,
            &token_account_id,
            ONE_YOCTO,
            FT_TRANSFER_GAS,
        )
        .then(ext_self::after_ft_transfer(
            account_id.clone(),
            token_account_id.clone(),
            amount.into(),
            &env::current_account_id(),
            NO_DEPOSIT,
            AFTER_FT_TRANSFER_GAS,
        ))
    }
}

#[near_bindgen]
impl SelfCallbacks for Contract {
    #[private]
    fn after_ft_transfer(
        &mut self,
        account_id: AccountId,
        token_account_id: AccountId,
        amount: WrappedBalance,
    ) {
        if !is_promise_success() {
            log!(
                "{} by {} token {} amount {}",
                errors::TOKEN_WITHDRAW_FAILED,
                account_id,
                token_account_id,
                amount.0
            );
            let mut account = self.internal_unwrap_account(&account_id);
            account.internal_token_deposit(&token_account_id, amount.0);
        }
    }
}
