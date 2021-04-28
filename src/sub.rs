use crate::*;
use near_sdk::json_types::WrappedBalance;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Subscription {
    pub shares: Balance,
    pub last_out_token_per_share: Vec<InnerU256>,
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
pub struct SubscriptionOutput {
    pub remaining_in_balance: WrappedBalance,
    pub unclaimed_out_balances: Vec<WrappedBalance>,
    pub shares: WrappedBalance,
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
}

impl Contract {
    pub fn internal_withdraw_shares(
        &mut self,
        sale_id: u64,
        account_id: &AccountId,
        shares: Option<Balance>,
    ) {
        let mut sale = self.internal_unwrap_sale(sale_id);
        let mut account = self.internal_unwrap_account(account_id);
        let mut subscription =
            self.internal_update_subscription(&mut account, sale_id, &mut sale, None);
        let shares = shares.unwrap_or(subscription.shares);
        assert!(shares > 0, "{}", errors::ZERO_SHARES);
        assert!(
            shares <= subscription.shares,
            "{}",
            errors::NOT_ENOUGH_SHARES
        );
        subscription.shares -= shares;
        let in_token_amount = sale.shares_to_in_balance(shares);
        if in_token_amount > 0 {
            account.internal_token_deposit(&sale.in_token_account_id, in_token_amount);
        }
        account.internal_save_subscription(sale_id, subscription);
        self.accounts.insert(&account_id, &account.into());

        sale.total_shares -= shares;
        sale.in_token_remaining -= in_token_amount;
        self.sales.insert(&sale_id, &sale.into());
    }

    pub fn internal_deposit_in_amount(
        &mut self,
        sale_id: u64,
        account_id: &AccountId,
        in_amount: Balance,
        referral_id: Option<&AccountId>,
    ) {
        assert_ne!(referral_id, Some(account_id), "{}", errors::SELF_REFERRAL);
        let mut sale = self.internal_unwrap_sale(sale_id);
        let mut account = self.internal_unwrap_account(account_id);
        let mut subscription =
            self.internal_update_subscription(&mut account, sale_id, &mut sale, referral_id);
        assert!(in_amount > 0, "{}", errors::ZERO_IN_AMOUNT);

        account.internal_token_withdraw(&sale.in_token_account_id, in_amount);
        for out_token in &sale.out_tokens {
            self.internal_maybe_register_token(&mut account, &out_token.token_account_id);
        }
        let shares = sale.in_amount_to_shares(in_amount);
        subscription.shares += shares;
        account.internal_save_subscription(sale_id, subscription);
        self.accounts.insert(&account_id, &account.into());

        sale.total_shares += shares;
        sale.in_token_remaining += in_amount;
        self.sales.insert(&sale_id, &sale.into());
    }
}
