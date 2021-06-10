use crate::*;
use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_sdk::is_promise_success;
use near_sdk::json_types::WrappedBalance;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum FtOnTransferArgs {
    AccountDeposit,
    DonateToTreasury,
}

#[ext_contract(ext_permission_contract)]
trait ExtPermissionContract {
    fn is_approved(&mut self, account_id: AccountId, sale_id: u64);
}

#[ext_contract(ext_self)]
trait SelfCallbacks {
    fn after_ft_transfer(
        &mut self,
        account_id: AccountId,
        token_account_id: AccountId,
        amount: WrappedBalance,
    ) -> bool;

    fn after_near_deposit(&mut self, amount: WrappedBalance) -> bool;

    fn after_is_approved(
        &mut self,
        sale_id: u64,
        account_id: AccountId,
        in_amount: WrappedBalance,
        referral_id: Option<AccountId>,
        attached_deposit: WrappedBalance,
    );

    fn maybe_refund_deposit(
        &mut self,
        account_id: AccountId,
        attached_deposit: WrappedBalance,
    ) -> bool;
}

trait SelfCallbacks {
    fn after_ft_transfer(
        &mut self,
        account_id: AccountId,
        token_account_id: AccountId,
        amount: WrappedBalance,
    ) -> bool;

    fn after_near_deposit(&mut self, amount: WrappedBalance) -> bool;

    fn after_is_approved(
        &mut self,
        is_approved: bool,
        sale_id: u64,
        account_id: AccountId,
        in_amount: WrappedBalance,
        referral_id: Option<AccountId>,
        attached_deposit: WrappedBalance,
    );

    fn maybe_refund_deposit(
        &mut self,
        account_id: AccountId,
        attached_deposit: WrappedBalance,
    ) -> bool;
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
    ) -> bool {
        let promise_success = is_promise_success();
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
        promise_success
    }

    #[private]
    fn after_near_deposit(&mut self, amount: WrappedBalance) -> bool {
        let promise_success = is_promise_success();
        if promise_success {
            log!(
                "Successfully wrapped {} NEAR tokens into Treasury",
                amount.0,
            );
            let w_near_token_id = self.treasury.w_near_token_id.clone();
            self.treasury.internal_deposit(&w_near_token_id, amount.0);
        }
        promise_success
    }

    #[private]
    fn after_is_approved(
        &mut self,
        #[callback] is_approved: bool,
        sale_id: u64,
        account_id: AccountId,
        in_amount: WrappedBalance,
        referral_id: Option<AccountId>,
        attached_deposit: WrappedBalance,
    ) {
        assert!(is_approved, "{}", errors::NOT_APPROVED);
        let initial_storage_usage = env::storage_usage();

        assert!(self
            .internal_deposit_in_amount(
                sale_id,
                &account_id,
                in_amount.0,
                referral_id.as_ref(),
                true,
            )
            .is_none());

        let attached_deposit = attached_deposit.0;
        let required_cost =
            env::storage_byte_cost() * Balance::from(env::storage_usage() - initial_storage_usage);
        assert!(
            required_cost <= attached_deposit,
            "{} {}",
            errors::NOT_ENOUGH_ATTACHED_BALANCE,
            required_cost,
        );

        let refund = attached_deposit - required_cost;
        if refund > 1 {
            Promise::new(account_id).transfer(refund);
        }
        self.treasury.locked_attached_deposits -= attached_deposit;
    }

    #[private]
    fn maybe_refund_deposit(
        &mut self,
        account_id: AccountId,
        attached_deposit: WrappedBalance,
    ) -> bool {
        let promise_success = is_promise_success();
        if !promise_success {
            self.treasury.locked_attached_deposits -= attached_deposit.0;
            Promise::new(account_id).transfer(attached_deposit.0);
        }
        promise_success
    }
}
