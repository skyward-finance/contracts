use crate::*;
use near_sdk::{Gas, Timestamp};

pub(crate) const NO_DEPOSIT: Balance = 0;
pub(crate) const ONE_YOCTO: Balance = 1;
pub(crate) const ONE_NEAR: Balance = 10u128.pow(24);

const BASE_GAS: Gas = 5_000_000_000_000;
pub(crate) const FT_TRANSFER_GAS: Gas = BASE_GAS;
pub(crate) const AFTER_FT_TRANSFER_GAS: Gas = BASE_GAS;

pub(crate) const STORAGE_DEPOSIT: Balance = 125 * env::STORAGE_PRICE_PER_BYTE;
pub(crate) const STORAGE_DEPOSIT_GAS: Gas = BASE_GAS * 2;
pub(crate) const NEAR_DEPOSIT_GAS: Gas = BASE_GAS;
// 1 NEAR
pub(crate) const EXTRA_NEAR_FOR_STORAGE: Balance = 1000 * env::STORAGE_PRICE_PER_BYTE;
pub(crate) const EXTRA_NEAR: Balance = EXTRA_NEAR_FOR_STORAGE + STORAGE_DEPOSIT;
pub(crate) const MIN_EXTRA_NEAR: Balance = EXTRA_NEAR + ONE_NEAR;
pub(crate) const AFTER_NEAR_DEPOSIT_GAS: Gas = BASE_GAS;

pub type TimestampSec = u32;

uint::construct_uint! {
    pub struct U256(4);
}

pub(crate) type InnerU256 = [u64; 4];
pub(crate) type TokenAccountId = AccountId;

uint::construct_uint! {
    pub struct U384(6);
}

pub(crate) fn refund_extra_storage_deposit(storage_used: StorageUsage, used_balance: Balance) {
    let required_cost = env::storage_byte_cost() * Balance::from(storage_used);
    let attached_deposit = env::attached_deposit()
        .checked_sub(used_balance)
        .expect(errors::NOT_ENOUGH_ATTACHED_BALANCE);

    assert!(
        required_cost <= attached_deposit,
        "{} {}",
        errors::NOT_ENOUGH_ATTACHED_BALANCE,
        required_cost,
    );

    let refund = attached_deposit - required_cost;
    if refund > 1 {
        Promise::new(env::predecessor_account_id()).transfer(refund);
    }
}

pub(crate) fn refund_released_storage(account_id: &AccountId, storage_released: StorageUsage) {
    if storage_released > 0 {
        let refund =
            env::storage_byte_cost() * Balance::from(storage_released) + env::attached_deposit();
        Promise::new(account_id.clone()).transfer(refund);
    }
}

pub(crate) fn assert_at_least_one_yocto() {
    assert!(
        env::attached_deposit() >= ONE_YOCTO,
        "{}",
        errors::NEED_AT_LEAST_ONE_YOCTO
    )
}

pub(crate) fn to_nano(timestamp: TimestampSec) -> Timestamp {
    Timestamp::from(timestamp) * 10u64.pow(9)
}
