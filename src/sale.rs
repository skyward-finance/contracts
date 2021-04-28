use crate::*;
use near_sdk::json_types::{WrappedBalance, WrappedDuration, WrappedTimestamp};
use near_sdk::{assert_one_yocto, Duration, Timestamp};

const MAX_DURATION_TO_START: Duration = 365 * 24 * 60 * 60 * 1_000_000_000;
const MAX_DURATION: Duration = 4 * 366 * 24 * 60 * 60 * 1_000_000_000;
/// Minimum duration. Use 1 nanosecond to run a simple auction.
const MIN_DURATION: Duration = 1;

pub(crate) const MULTIPLIER: u128 = u128::MAX;
pub(crate) const TREASURY_FEE_DENOMINATOR: Balance = 100;
pub(crate) const MAX_NUM_OUT_TOKENS: usize = 4;

#[derive(BorshSerialize, BorshDeserialize)]
#[borsh_init(touch)]
pub struct Sale {
    pub owner_id: AccountId,

    pub out_tokens: Vec<SaleOutToken>,

    pub in_token_account_id: AccountId,
    pub in_token_remaining: Balance,
    pub in_token_paid_unclaimed: Balance,
    pub in_token_paid: Balance,

    pub start_time: Timestamp,
    pub duration: Duration,

    pub total_shares: Balance,
    pub last_timestamp: Timestamp,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SaleOutToken {
    pub token_account_id: TokenAccountId,
    pub remaining: Balance,
    pub distributed: Balance,
    pub treasury_unclaimed: Option<Balance>,
    pub per_share: InnerU256,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub enum VSale {
    Current(Sale),
}

impl From<Sale> for VSale {
    fn from(sale: Sale) -> Self {
        Self::Current(sale)
    }
}

impl From<VSale> for Sale {
    fn from(v_sale: VSale) -> Self {
        match v_sale {
            VSale::Current(sale) => sale,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleInput {
    pub out_tokens: Vec<SaleInputOutToken>,

    pub in_token_account_id: ValidAccountId,

    pub start_time: WrappedTimestamp,
    pub duration: WrappedDuration,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleInputOutToken {
    token_account_id: ValidAccountId,
    balance: WrappedBalance,
}

impl SaleOutToken {
    pub fn from_input(token: SaleInputOutToken, skyward_token_id: &TokenAccountId) -> Self {
        let is_skyward_token = token.token_account_id.as_ref() == skyward_token_id;
        Self {
            token_account_id: token.token_account_id.into(),
            remaining: token.balance.into(),
            distributed: 0,
            treasury_unclaimed: if is_skyward_token { Some(0) } else { None },
            per_share: U256::zero().0,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleOutput {
    pub sale_id: u64,
    pub owner_id: AccountId,

    pub out_tokens: Vec<SaleOutputOutToken>,

    pub in_token_account_id: AccountId,
    pub in_token_remaining: WrappedBalance,
    pub in_token_paid_unclaimed: WrappedBalance,
    pub in_token_paid: WrappedBalance,

    pub total_shares: WrappedBalance,

    pub start_time: WrappedTimestamp,
    pub duration: WrappedDuration,
    pub remaining_duration: WrappedDuration,

    pub subscription: Option<SubscriptionOutput>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleOutputOutToken {
    pub token_account_id: TokenAccountId,
    pub remaining: WrappedBalance,
    pub distributed: WrappedBalance,
    pub treasury_unclaimed: Option<WrappedBalance>,
}

impl From<SaleOutToken> for SaleOutputOutToken {
    fn from(token: SaleOutToken) -> Self {
        Self {
            token_account_id: token.token_account_id,
            remaining: token.remaining.into(),
            distributed: token.distributed.into(),
            treasury_unclaimed: token.treasury_unclaimed.map(|b| b.into()),
        }
    }
}

impl Sale {
    pub fn touch(&mut self) {
        let end_time = self.start_time + self.duration;
        let timestamp = std::cmp::min(end_time, env::block_timestamp());
        if timestamp <= self.last_timestamp {
            // Sale haven't started or already updated.
            return;
        }
        if self.last_timestamp >= end_time {
            // Sale closed
            return;
        }
        if self.total_shares == 0 {
            return;
        }
        let time_diff = U256::from(timestamp - self.last_timestamp);
        let remaining_duration = U256::from(end_time - self.last_timestamp);

        for out_token in &mut self.out_tokens {
            let mut amount =
                (U256::from(out_token.remaining) * time_diff / remaining_duration).as_u128();
            if amount > 0 {
                out_token.distributed += amount;
                out_token.remaining -= amount;
                if let Some(treasury_unclaimed) = &mut out_token.treasury_unclaimed {
                    let treasury_fee = amount / TREASURY_FEE_DENOMINATOR;
                    *treasury_unclaimed += treasury_fee;
                    amount -= treasury_fee;
                }
                out_token.per_share = (U256(out_token.per_share)
                    + U256::from(amount) * U256::from(MULTIPLIER) / U256::from(self.total_shares))
                .0;
            }
        }

        let in_token_amount =
            (U256::from(self.in_token_remaining) * time_diff / remaining_duration).as_u128();
        self.in_token_paid_unclaimed += in_token_amount;
        self.in_token_paid += in_token_amount;
        self.in_token_remaining -= in_token_amount;

        self.last_timestamp = timestamp;
    }

    pub fn assert_valid_not_started(&self) {
        let timestamp = env::block_timestamp();
        assert!(self.start_time > timestamp, "{}", errors::ALREADY_STARTED);
        assert!(
            self.start_time < timestamp + MAX_DURATION_TO_START,
            "{}",
            errors::MAX_DURATION_TO_START
        );
        assert!(self.duration <= MAX_DURATION, "{}", errors::MAX_DURATION);
        assert!(self.duration >= MIN_DURATION, "{}", errors::MIN_DURATION);
        assert!(
            self.out_tokens.len() <= MAX_NUM_OUT_TOKENS,
            "{}",
            errors::MAX_NUM_OUT_TOKENS
        );
        let mut unique_tokens = Vec::with_capacity(self.out_tokens.len());
        for out_token in &self.out_tokens {
            assert!(out_token.remaining > 0, "{}", errors::ZERO_OUT_AMOUNT);
            assert_ne!(
                self.in_token_account_id,
                out_token.token_account_id,
                "{}",
                errors::SAME_TOKENS
            );
            unique_tokens.push(out_token.token_account_id.clone());
        }
        unique_tokens.sort();
        unique_tokens.dedup();
        assert_eq!(
            unique_tokens.len(),
            self.out_tokens.len(),
            "{}",
            errors::NON_UNIQUE_OUT_TOKENS
        );
    }

    pub fn from_input(
        sale: SaleInput,
        owner_id: AccountId,
        skyward_token_id: &TokenAccountId,
    ) -> Self {
        let start_time = sale.start_time.into();
        Sale {
            owner_id,
            out_tokens: sale
                .out_tokens
                .into_iter()
                .map(|o| SaleOutToken::from_input(o, skyward_token_id))
                .collect(),
            in_token_account_id: sale.in_token_account_id.into(),
            in_token_remaining: 0,
            in_token_paid_unclaimed: 0,
            in_token_paid: 0,
            total_shares: 0,
            start_time,
            duration: sale.duration.into(),
            last_timestamp: start_time,
        }
    }

    pub fn into_output(self, sale_id: u64, account: Option<&Account>) -> SaleOutput {
        let remaining_duration = self.start_time + self.duration - self.last_timestamp;
        let subscription =
            account.and_then(|account| account.internal_subscription_output(sale_id, &self));
        SaleOutput {
            sale_id,
            owner_id: self.owner_id,
            out_tokens: self.out_tokens.into_iter().map(|o| o.into()).collect(),
            in_token_account_id: self.in_token_account_id,
            in_token_remaining: self.in_token_remaining.into(),
            in_token_paid_unclaimed: self.in_token_paid_unclaimed.into(),
            in_token_paid: self.in_token_paid.into(),
            total_shares: self.total_shares.into(),
            start_time: self.start_time.into(),
            duration: self.duration.into(),
            remaining_duration: remaining_duration.into(),
            subscription,
        }
    }

    /// Returns remaining in_balance
    pub fn shares_to_in_balance(&self, shares: Balance) -> Balance {
        if shares == 0 {
            return 0;
        }
        return (U256::from(self.in_token_remaining) * U256::from(shares)
            / U256::from(self.total_shares))
        .as_u128();
    }

    pub fn in_amount_to_shares(&self, in_amount: Balance) -> Balance {
        if self.total_shares == 0 {
            return in_amount;
        }
        assert!(
            self.in_token_remaining > 0 && !self.has_ended(),
            "{}",
            errors::SALE_ENDED
        );
        let num_shares = U256::from(in_amount) * U256::from(self.total_shares)
            / U256::from(self.in_token_remaining);
        assert!(
            num_shares + U256::from(self.total_shares) < U256::from(u128::MAX),
            "{}",
            errors::SHARES_OVERFLOW
        );
        num_shares.as_u128()
    }

    pub fn has_ended(&self) -> bool {
        self.last_timestamp >= self.start_time + self.duration
    }

    pub fn ensure_skyward_token_present(&mut self, skyward_token_id: &TokenAccountId) {
        if &self.in_token_account_id != skyward_token_id
            && self
                .out_tokens
                .iter()
                .find(|o| &o.token_account_id == skyward_token_id)
                .is_none()
        {
            self.out_tokens.push(SaleOutToken {
                token_account_id: skyward_token_id.clone(),
                remaining: 0,
                distributed: 0,
                treasury_unclaimed: None,
                per_share: U256::zero().0,
            })
        }
    }
}

impl Contract {
    pub fn internal_unwrap_sale(&self, sale_id: u64) -> Sale {
        self.sales
            .get(&sale_id)
            .expect(errors::SALE_NOT_FOUND)
            .into()
    }

    pub fn internal_get_sale(&self, sale_id: u64, account: Option<&Account>) -> Option<SaleOutput> {
        self.sales.get(&sale_id).map(|v_sale| {
            let sale: Sale = v_sale.into();
            sale.into_output(sale_id, account)
        })
    }

    pub fn internal_distribute_unclaimed_tokens(&mut self, sale: &mut Sale) {
        let mut account = self.internal_unwrap_account(&sale.owner_id);
        if sale.in_token_paid_unclaimed > 0 {
            if &sale.in_token_account_id != &self.treasury.skyward_token_id {
                let treasury_fee = sale.in_token_paid_unclaimed / TREASURY_FEE_DENOMINATOR;
                self.treasury
                    .internal_deposit(&sale.in_token_account_id, treasury_fee);
                sale.in_token_paid_unclaimed -= treasury_fee;
            }
            account.internal_token_deposit(&sale.in_token_account_id, sale.in_token_paid_unclaimed);
            self.accounts.insert(&sale.owner_id, &account.into());

            sale.in_token_paid_unclaimed = 0;
        }
        for out_token in &mut sale.out_tokens {
            if let Some(treasury_unclaimed) = &mut out_token.treasury_unclaimed {
                self.treasury
                    .internal_deposit(&out_token.token_account_id, *treasury_unclaimed);
                *treasury_unclaimed = 0;
            }
        }
    }
}

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn sale_create(&mut self, sale: SaleInput) -> SaleOutput {
        let initial_storage_usage = env::storage_usage();
        let sale_id = self.num_sales;
        let mut sale = Sale::from_input(
            sale,
            env::predecessor_account_id(),
            &self.treasury.skyward_token_id,
        );
        sale.assert_valid_not_started();
        sale.ensure_skyward_token_present(&self.treasury.skyward_token_id);

        let mut account = self.internal_unwrap_account(&sale.owner_id);
        for out_token in &sale.out_tokens {
            if out_token.remaining > 0 {
                account.internal_token_withdraw(&out_token.token_account_id, out_token.remaining);
            }
        }
        self.internal_maybe_register_token(&mut account, &sale.in_token_account_id);
        account.sales.insert(&sale_id);
        self.accounts.insert(&sale.owner_id, &account.into());
        self.sales.insert(&sale_id, &sale.into());
        self.num_sales += 1;
        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage);
        self.get_sale(sale_id, None).unwrap()
    }

    #[payable]
    pub fn sale_deposit_out_token(
        &mut self,
        sale_id: u64,
        token_account_id: ValidAccountId,
        amount: WrappedBalance,
    ) {
        assert_at_least_one_yocto();
        let amount: Balance = amount.into();
        assert!(amount > 0, "{}", errors::ZERO_AMOUNT);
        let initial_storage_usage = env::storage_usage();
        let mut sale = self.internal_unwrap_sale(sale_id);
        assert!(!sale.has_ended(), "{}", errors::SALE_ENDED);
        let account_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&account_id);
        account.internal_token_withdraw(token_account_id.as_ref(), amount);
        self.accounts.insert(&account_id, &account.into());
        {
            let out_token = sale
                .out_tokens
                .iter_mut()
                .find(|out_token| &out_token.token_account_id == token_account_id.as_ref())
                .expect(errors::TOKEN_NOT_IN_SALE);
            out_token.remaining = out_token
                .remaining
                .checked_add(amount)
                .expect(errors::BALANCE_OVERFLOW);
        }
        self.sales.insert(&sale_id, &sale.into());
        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage);
    }

    pub fn get_sale(&self, sale_id: u64, account_id: Option<ValidAccountId>) -> Option<SaleOutput> {
        let account: Option<Account> = account_id
            .and_then(|account_id| self.accounts.get(account_id.as_ref()).map(|a| a.into()));
        self.internal_get_sale(sale_id, account.as_ref())
    }

    pub fn get_sales(
        &self,
        account_id: Option<ValidAccountId>,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<SaleOutput> {
        let account: Option<Account> = account_id
            .and_then(|account_id| self.accounts.get(account_id.as_ref()).map(|a| a.into()));
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(self.num_sales);
        (from_index..std::cmp::min(from_index + limit, self.num_sales))
            .filter_map(|sale_id| self.internal_get_sale(sale_id, account.as_ref()))
            .collect()
    }

    #[payable]
    pub fn sale_deposit_in_token(&mut self, sale_id: u64, amount: WrappedBalance) {
        assert_at_least_one_yocto();
        let initial_storage_usage = env::storage_usage();
        let account_id = env::predecessor_account_id();
        self.internal_deposit_in_amount(sale_id, &account_id, amount.0);
        refund_extra_storage_deposit(env::storage_usage() - initial_storage_usage);
    }

    #[payable]
    pub fn sale_withdraw_in_token(&mut self, sale_id: u64, shares: Option<WrappedBalance>) {
        assert_one_yocto();
        let initial_storage_usage = env::storage_usage();
        let account_id = env::predecessor_account_id();
        self.internal_withdraw_shares(sale_id, &account_id, shares.map(|s| s.0));
        refund_released_storage(&account_id, initial_storage_usage - env::storage_usage());
    }

    /// This method can be called by anyone in order to move in tokens to treasury
    pub fn sale_distribute_unclaimed_tokens(&mut self, sale_id: u64) {
        let mut sale = self.internal_unwrap_sale(sale_id);
        self.internal_distribute_unclaimed_tokens(&mut sale);
        self.sales.insert(&sale_id, &sale.into());
    }

    pub fn sale_claim_out_tokens(&mut self, sale_id: u64) {
        let account_id = env::predecessor_account_id();
        let initial_storage_usage = env::storage_usage();
        let mut sale = self.internal_unwrap_sale(sale_id);
        self.internal_distribute_unclaimed_tokens(&mut sale);
        let mut account = self.internal_unwrap_account(&account_id);
        let subscription = account.internal_update_subscription(sale_id, &mut sale);

        account.internal_save_subscription(sale_id, subscription);

        self.accounts.insert(&account_id, &account.into());
        self.sales.insert(&sale_id, &sale.into());
        refund_released_storage(&account_id, initial_storage_usage - env::storage_usage());
    }
}
