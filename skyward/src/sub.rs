use crate::*;
use near_sdk::json_types::WrappedBalance;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Subscription {
    pub shares: Balance,
    pub last_in_balance: Balance,
    pub spent_in_balance_without_shares: Balance,
    pub last_out_token_per_share: Vec<InnerU256>,
    pub claimed_out_balance: Vec<Balance>,
    pub referral_id: Option<AccountId>,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub enum VSubscription {
    Current(Subscription),
}

impl From<Subscription> for VSubscription {
    fn from(subscription: Subscription) -> Self {
        Self::Current(subscription)
    }
}

impl From<VSubscription> for Subscription {
    fn from(v_subscription: VSubscription) -> Self {
        match v_subscription {
            VSubscription::Current(subscription) => subscription,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct SubscriptionOutput {
    pub remaining_in_balance: WrappedBalance,
    pub spent_in_balance: WrappedBalance,
    pub unclaimed_out_balances: Vec<WrappedBalance>,
    pub claimed_out_balance: Vec<WrappedBalance>,
    pub shares: WrappedBalance,
    pub referral_id: Option<AccountId>,
}

impl Subscription {
    pub fn touch(&mut self, sale: &Sale) -> Vec<Balance> {
        let shares = U256::from(self.shares);
        let multiplier = U256::from(MULTIPLIER);
        self.last_out_token_per_share
            .iter_mut()
            .zip(sale.out_tokens.iter())
            .map(|(last_out_token_per_share, out_token)| {
                let out_token_per_share = U256(out_token.per_share.clone());
                let u256_last_out_token_per_share = U256(last_out_token_per_share.clone());
                let out_token_amount = if out_token_per_share == U256::zero() {
                    0
                } else {
                    let diff = out_token_per_share - u256_last_out_token_per_share;
                    (diff * shares / multiplier).as_u128()
                };
                *last_out_token_per_share = out_token_per_share.0;
                out_token_amount
            })
            .collect()
    }

    pub fn new(sale: &Sale, referral_id: Option<AccountId>) -> Self {
        Self {
            shares: 0,
            spent_in_balance_without_shares: 0,
            last_in_balance: 0,
            last_out_token_per_share: sale
                .out_tokens
                .iter()
                .map(|out_token| out_token.per_share.clone())
                .collect(),
            claimed_out_balance: vec![0; sale.out_tokens.len()],
            referral_id,
        }
    }
}

impl Contract {
    pub fn internal_withdraw_shares(
        &mut self,
        sale_id: u64,
        account_id: &AccountId,
        shares: Option<Balance>,
    ) {
        let mut sale = self.internal_unwrap_sale(sale_id);
        self.internal_distribute_unclaimed_tokens(&mut sale);
        let mut account = self.internal_unwrap_account(account_id);
        let mut subscription =
            self.internal_update_subscription(&mut account, sale_id, &mut sale, None, false);
        let shares = shares.unwrap_or(subscription.shares);
        assert!(shares > 0, "{}", errors::ZERO_SHARES);
        assert!(
            shares <= subscription.shares,
            "{}",
            errors::NOT_ENOUGH_SHARES
        );
        let remaining_in_balance = sale.shares_to_in_balance(subscription.shares);
        subscription.spent_in_balance_without_shares +=
            subscription.last_in_balance - remaining_in_balance;
        subscription.shares -= shares;
        let in_token_amount = sale.shares_to_in_balance(shares);
        if in_token_amount > 0 {
            account.internal_token_deposit(&sale.in_token_account_id, in_token_amount);
        }
        sale.total_shares -= shares;
        sale.in_token_remaining -= in_token_amount;

        subscription.last_in_balance = sale.shares_to_in_balance(subscription.shares);

        account.internal_save_subscription(sale_id, &sale, subscription);
        self.accounts.insert(&account_id, &account.into());
        self.sales.insert(&sale_id, &sale.into());
    }

    pub fn internal_withdraw_in_token_exact(
        &mut self,
        sale_id: u64,
        account_id: &AccountId,
        in_amount: Balance,
    ) {
        let mut sale = self.internal_unwrap_sale(sale_id);
        self.internal_distribute_unclaimed_tokens(&mut sale);
        let mut account = self.internal_unwrap_account(account_id);
        let mut subscription =
            self.internal_update_subscription(&mut account, sale_id, &mut sale, None, false);
        assert!(in_amount > 0, "{}", errors::ZERO_IN_AMOUNT);
        let remaining_in_balance = sale.shares_to_in_balance(subscription.shares);
        assert!(
            in_amount <= remaining_in_balance,
            "{}",
            errors::NOT_ENOUGH_BALANCE
        );
        let shares = sale.in_amount_to_shares(in_amount, true);
        subscription.spent_in_balance_without_shares +=
            subscription.last_in_balance - remaining_in_balance;
        subscription.shares -= shares;
        account.internal_token_deposit(&sale.in_token_account_id, in_amount);
        sale.total_shares -= shares;
        sale.in_token_remaining -= in_amount;

        subscription.last_in_balance = sale.shares_to_in_balance(subscription.shares);

        account.internal_save_subscription(sale_id, &sale, subscription);
        self.accounts.insert(&account_id, &account.into());
        self.sales.insert(&sale_id, &sale.into());
    }

    pub fn internal_deposit_in_amount(
        &mut self,
        sale_id: u64,
        account_id: &AccountId,
        in_amount: Balance,
        referral_id: Option<&AccountId>,
        passed_permission_check: bool,
    ) -> Option<AccountId> {
        assert_ne!(referral_id, Some(account_id), "{}", errors::SELF_REFERRAL);
        assert!(in_amount > 0, "{}", errors::ZERO_IN_AMOUNT);
        let mut sale = self.internal_unwrap_sale(sale_id);
        self.internal_distribute_unclaimed_tokens(&mut sale);
        let mut account = self.internal_unwrap_account(account_id);
        if !passed_permission_check {
            if let Some(permissions_contract_id) = &sale.permissions_contract_id {
                if account.subs.get(&sale_id).is_none() {
                    // Need to check permissions first
                    return Some(permissions_contract_id.clone());
                }
            }
        }

        let mut subscription = self.internal_update_subscription(
            &mut account,
            sale_id,
            &mut sale,
            referral_id,
            passed_permission_check,
        );

        account.internal_token_withdraw(&sale.in_token_account_id, in_amount);
        for out_token in &sale.out_tokens {
            self.internal_maybe_register_token(&mut account, &out_token.token_account_id);
        }
        let remaining_in_balance = sale.shares_to_in_balance(subscription.shares);
        subscription.spent_in_balance_without_shares +=
            subscription.last_in_balance - remaining_in_balance;
        let shares = sale.in_amount_to_shares(in_amount, false);
        subscription.shares += shares;
        sale.total_shares += shares;
        sale.in_token_remaining += in_amount;

        subscription.last_in_balance = sale.shares_to_in_balance(subscription.shares);

        account.internal_save_subscription(sale_id, &sale, subscription);
        self.accounts.insert(&account_id, &account.into());
        self.sales.insert(&sale_id, &sale.into());
        None
    }
}
