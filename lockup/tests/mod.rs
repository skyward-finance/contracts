use lockup::{AccountOutput, ContractContract as LockupContract, Stats, TimestampSec};
use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FT_METADATA_SPEC};
use near_sdk::env::STORAGE_PRICE_PER_BYTE;
use near_sdk::json_types::{ValidAccountId, WrappedBalance, U128};
use near_sdk::serde::Serialize;
use near_sdk::serde_json::json;
use near_sdk::{env, Balance, Gas, Timestamp};
use near_sdk_sim::runtime::GenesisConfig;
use near_sdk_sim::{
    deploy, init_simulator, to_yocto, ContractAccount, ExecutionResult, UserAccount,
};
use std::convert::TryInto;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    LOCKUP_WASM_BYTES => "res/lockup0.wasm",
    SKYWARD_WASM_BYTES => "../skyward/res/skyward.wasm",

    FUNGIBLE_TOKEN_WASM_BYTES => "../common/fungible_token.wasm",
}

pub fn accounts(id: usize) -> ValidAccountId {
    ["alice.near", "bob.near", "charlie.near"][id]
        .to_string()
        .try_into()
        .unwrap()
}

fn to_nano(timestamp: TimestampSec) -> Timestamp {
    Timestamp::from(timestamp) * 10u64.pow(9)
}

const NEAR: &str = "near";
const SKYWARD_ID: &str = "skyward.near";
const SKYWARD_CLAIM_ID: &str = "claim.skyward.near";
const SKYWARD_TOKEN_ID: &str = "token.skyward.near";
const WRAP_NEAR_ID: &str = "wrap_near.skyward.near";
const SKYWARD_DAO_ID: &str = "skyward-dao.near";

const BASE_GAS: Gas = 10_000_000_000_000;
const CLAIM_GAS: Gas = 60_000_000_000_000;
const DONATE_GAS: Gas = 80_000_000_000_000;
const SKYWARD_TOKEN_DECIMALS: u8 = 18;
const SKYWARD_TOKEN_BASE: Balance = 10u128.pow(SKYWARD_TOKEN_DECIMALS as u32);
const SKYWARD_TOTAL_SUPPLY: Balance = 1_000_000 * SKYWARD_TOKEN_BASE;
const ONE_NEAR: Balance = 10u128.pow(24);
const LISTING_FEE_NEAR: Balance = 10 * ONE_NEAR;

// From example.csv
const CLAIM_EXPIRATION_TIMESTAMP: TimestampSec = 1640000000;
const TOTAL_LOCKUP_BALANCE: Balance = 10010000000000000000;
const TIMESTAMP_1: TimestampSec = 1622505600;
const TIMESTAMP_2: TimestampSec = 1625097600;
const TIMESTAMP_3: TimestampSec = 1633046400;
const BALANCE_1: Balance = 10u128.pow(16);
const BALANCE_2: Balance = 10u128.pow(19);

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
struct VestingIntervalInput {
    pub start_timestamp: TimestampSec,
    pub end_timestamp: TimestampSec,
    pub amount: WrappedBalance,
}

pub struct Env {
    pub root: UserAccount,
    pub near: UserAccount,
    pub skyward_claim: ContractAccount<LockupContract>,
    pub skyward_dao: UserAccount,
    pub skyward: UserAccount,
    pub skyward_token: UserAccount,

    pub users: Vec<UserAccount>,
}

fn storage_deposit(user: &UserAccount, token_account_id: &str, account_id: &str) {
    user.call(
        token_account_id.to_string(),
        "storage_deposit",
        &json!({
            "account_id": account_id.to_string()
        })
        .to_string()
        .into_bytes(),
        BASE_GAS,
        125 * env::STORAGE_PRICE_PER_BYTE, // attached deposit
    )
    .assert_success();
}

impl Env {
    pub fn init(num_users: usize) -> Self {
        let mut genesis_config = GenesisConfig::default();
        genesis_config.runtime_config.storage_amount_per_byte = STORAGE_PRICE_PER_BYTE;
        let root = init_simulator(Some(genesis_config));
        let near = root.create_user(NEAR.to_string(), to_yocto("1000"));
        let skyward_dao = near.create_user(SKYWARD_DAO_ID.to_string(), to_yocto("100"));
        let skyward = near.deploy_and_init(
            &SKYWARD_WASM_BYTES,
            SKYWARD_ID.to_string(),
            "new",
            &json!({
                "skyward_token_id": SKYWARD_TOKEN_ID.to_string(),
                "skyward_vesting_schedule": vec![
                    VestingIntervalInput {
                        start_timestamp: 0,
                        end_timestamp: 1,
                        amount: U128(SKYWARD_TOTAL_SUPPLY),
                    },
                ],
                "listing_fee_near": U128::from(LISTING_FEE_NEAR),
                "w_near_token_id": WRAP_NEAR_ID.to_string(),
            })
            .to_string()
            .into_bytes(),
            to_yocto("30"),
            BASE_GAS,
        );
        let skyward_token = skyward.deploy_and_init(
            &FUNGIBLE_TOKEN_WASM_BYTES,
            SKYWARD_TOKEN_ID.to_string(),
            "new",
            &json!({
                "owner_id": skyward_dao.valid_account_id(),
                "total_supply": U128::from(SKYWARD_TOTAL_SUPPLY),
                "metadata": FungibleTokenMetadata {
                    spec: FT_METADATA_SPEC.to_string(),
                    name: "Skyward Finance Token".to_string(),
                    symbol: "SKYWARD".to_string(),
                    icon: None,
                    reference: None,
                    reference_hash: None,
                    decimals: SKYWARD_TOKEN_DECIMALS,
                }
            })
            .to_string()
            .into_bytes(),
            to_yocto("10"),
            BASE_GAS,
        );
        let skyward_claim = deploy!(
            contract: LockupContract,
            contract_id: SKYWARD_CLAIM_ID.to_string(),
            bytes: &LOCKUP_WASM_BYTES,
            signer_account: skyward,
            deposit: to_yocto("10"),
            gas: BASE_GAS,
            init_method: new(skyward_token.valid_account_id(), skyward.valid_account_id(), CLAIM_EXPIRATION_TIMESTAMP)
        );
        // Registering tokens
        storage_deposit(&skyward_dao, SKYWARD_TOKEN_ID, SKYWARD_ID);
        storage_deposit(&skyward_dao, SKYWARD_TOKEN_ID, SKYWARD_CLAIM_ID);

        // Give total lockup balance to claim.
        skyward_dao
            .call(
                SKYWARD_TOKEN_ID.to_string(),
                "ft_transfer",
                &json!({
                    "receiver_id": SKYWARD_CLAIM_ID.to_string(),
                    "amount": U128::from(TOTAL_LOCKUP_BALANCE),
                })
                .to_string()
                .into_bytes(),
                BASE_GAS,
                1,
            )
            .assert_success();

        let mut this = Self {
            root,
            near,
            skyward_claim,
            skyward_dao,
            skyward,
            skyward_token,
            users: vec![],
        };
        this.init_users(num_users);
        this
    }

    pub fn init_users(&mut self, num_users: usize) {
        for i in 0..num_users {
            let user = self.near.create_user(accounts(i).into(), to_yocto("100"));
            storage_deposit(&user, SKYWARD_TOKEN_ID, &user.account_id);
            self.users.push(user);
        }
    }

    pub fn get_account(&self, user: &UserAccount) -> Option<AccountOutput> {
        self.near
            .view_method_call(
                self.skyward_claim
                    .contract
                    .get_account(user.valid_account_id()),
            )
            .unwrap_json()
    }

    pub fn get_stats(&self) -> Stats {
        self.near
            .view_method_call(self.skyward_claim.contract.get_stats())
            .unwrap_json()
    }

    pub fn claim(&self, user: &UserAccount) -> ExecutionResult {
        user.function_call(self.skyward_claim.contract.claim(), CLAIM_GAS, 0)
    }

    pub fn get_skyward_token_balance(&self, user: &UserAccount) -> Balance {
        let balance: Option<WrappedBalance> = self
            .near
            .view(
                SKYWARD_TOKEN_ID.to_string(),
                "ft_balance_of",
                &json!({
                    "account_id": user.valid_account_id(),
                })
                .to_string()
                .into_bytes(),
            )
            .unwrap_json();
        balance.unwrap().0
    }

    pub fn get_treasury_circulating_supply(&self) -> Balance {
        let balance: WrappedBalance = self
            .near
            .view(
                SKYWARD_ID.to_string(),
                "get_skyward_circulating_supply",
                b"{}",
            )
            .unwrap_json();
        balance.0
    }
}

#[test]
fn test_init() {
    Env::init(0);
}

#[test]
fn test_initial_get_account() {
    let e = Env::init(3);
    assert_eq!(
        e.get_account(&e.users[0]),
        Some(AccountOutput {
            start_timestamp: TIMESTAMP_1,
            cliff_timestamp: TIMESTAMP_1,
            end_timestamp: TIMESTAMP_2,
            balance: U128(BALANCE_1),
            claimed_balance: U128(0)
        })
    );
    assert_eq!(
        e.get_account(&e.users[1]),
        Some(AccountOutput {
            start_timestamp: TIMESTAMP_1,
            cliff_timestamp: TIMESTAMP_2,
            end_timestamp: TIMESTAMP_3,
            balance: U128(BALANCE_2),
            claimed_balance: U128(0)
        })
    );
    assert!(e.get_account(&e.users[2]).is_none());
}

#[test]
fn test_claim() {
    let e = Env::init(3);
    e.root.borrow_runtime_mut().genesis.block_prod_time = 0;
    e.root.borrow_runtime_mut().cur_block.block_timestamp = to_nano(TIMESTAMP_1 - 100);
    assert_eq!(e.get_skyward_token_balance(&e.users[0]), 0);
    assert_eq!(
        e.get_stats(),
        Stats {
            token_account_id: SKYWARD_TOKEN_ID.to_string(),
            skyward_account_id: SKYWARD_ID.to_string(),
            claim_expiration_timestamp: CLAIM_EXPIRATION_TIMESTAMP,
            total_balance: U128(TOTAL_LOCKUP_BALANCE),
            untouched_balance: U128(TOTAL_LOCKUP_BALANCE),
            total_claimed: U128(0)
        }
    );
    e.claim(&e.users[0]).assert_success();
    assert_eq!(e.get_skyward_token_balance(&e.users[0]), 0);
    assert_eq!(e.get_account(&e.users[0]).unwrap().claimed_balance.0, 0);
    assert_eq!(
        e.get_stats(),
        Stats {
            token_account_id: SKYWARD_TOKEN_ID.to_string(),
            skyward_account_id: SKYWARD_ID.to_string(),
            claim_expiration_timestamp: CLAIM_EXPIRATION_TIMESTAMP,
            total_balance: U128(TOTAL_LOCKUP_BALANCE),
            untouched_balance: U128(TOTAL_LOCKUP_BALANCE - BALANCE_1),
            total_claimed: U128(0)
        }
    );

    assert_eq!(e.get_skyward_token_balance(&e.users[1]), 0);
    e.claim(&e.users[1]).assert_success();
    assert_eq!(e.get_account(&e.users[1]).unwrap().claimed_balance.0, 0);
    assert_eq!(
        e.get_stats(),
        Stats {
            token_account_id: SKYWARD_TOKEN_ID.to_string(),
            skyward_account_id: SKYWARD_ID.to_string(),
            claim_expiration_timestamp: CLAIM_EXPIRATION_TIMESTAMP,
            total_balance: U128(TOTAL_LOCKUP_BALANCE),
            untouched_balance: U128(0),
            total_claimed: U128(0)
        }
    );
    assert_eq!(e.get_skyward_token_balance(&e.users[1]), 0);

    e.claim(&e.users[1]).assert_success();
    assert!(!e.claim(&e.users[2]).is_ok());

    e.root.borrow_runtime_mut().cur_block.block_timestamp =
        to_nano((TIMESTAMP_2 - TIMESTAMP_1) / 2 + TIMESTAMP_1);
    e.claim(&e.users[0]).assert_success();
    assert_eq!(
        e.get_account(&e.users[0]).unwrap().claimed_balance.0,
        BALANCE_1 / 2
    );
    assert_eq!(e.get_stats().total_claimed.0, BALANCE_1 / 2);
    assert_eq!(e.get_skyward_token_balance(&e.users[0]), BALANCE_1 / 2);

    e.claim(&e.users[1]).assert_success();
    assert_eq!(e.get_account(&e.users[1]).unwrap().claimed_balance.0, 0);
    assert_eq!(e.get_stats().total_claimed.0, BALANCE_1 / 2);
    assert_eq!(e.get_skyward_token_balance(&e.users[1]), 0);

    e.root.borrow_runtime_mut().cur_block.block_timestamp =
        to_nano((TIMESTAMP_3 - TIMESTAMP_1) / 2 + TIMESTAMP_1);
    e.claim(&e.users[0]).assert_success();
    assert_eq!(
        e.get_account(&e.users[0]).unwrap().claimed_balance.0,
        BALANCE_1
    );
    assert_eq!(e.get_stats().total_claimed.0, BALANCE_1);
    assert_eq!(e.get_skyward_token_balance(&e.users[0]), BALANCE_1);

    e.claim(&e.users[1]).assert_success();
    assert_eq!(
        e.get_account(&e.users[1]).unwrap().claimed_balance.0,
        BALANCE_2 / 2
    );
    assert_eq!(e.get_stats().total_claimed.0, BALANCE_1 + BALANCE_2 / 2);
    assert_eq!(e.get_skyward_token_balance(&e.users[1]), BALANCE_2 / 2);

    e.root.borrow_runtime_mut().cur_block.block_timestamp = to_nano(TIMESTAMP_3 + 100);
    e.claim(&e.users[0]).assert_success();
    assert_eq!(
        e.get_account(&e.users[0]).unwrap().claimed_balance.0,
        BALANCE_1
    );
    assert_eq!(e.get_stats().total_claimed.0, BALANCE_1 + BALANCE_2 / 2);
    assert_eq!(e.get_skyward_token_balance(&e.users[0]), BALANCE_1);

    e.claim(&e.users[1]).assert_success();
    assert_eq!(
        e.get_account(&e.users[1]).unwrap().claimed_balance.0,
        BALANCE_2
    );
    assert_eq!(e.get_stats().total_claimed.0, TOTAL_LOCKUP_BALANCE);
    assert_eq!(e.get_skyward_token_balance(&e.users[1]), BALANCE_2);
}

#[test]
fn test_claim_unregistered() {
    let e = Env::init(0);
    e.root.borrow_runtime_mut().genesis.block_prod_time = 0;
    e.root.borrow_runtime_mut().cur_block.block_timestamp = to_nano(TIMESTAMP_1 - 100);

    let user = e.near.create_user(accounts(0).into(), to_yocto("100"));

    assert_eq!(e.get_skyward_token_balance(&user), 0);
    assert_eq!(
        e.get_stats(),
        Stats {
            token_account_id: SKYWARD_TOKEN_ID.to_string(),
            skyward_account_id: SKYWARD_ID.to_string(),
            claim_expiration_timestamp: CLAIM_EXPIRATION_TIMESTAMP,
            total_balance: U128(TOTAL_LOCKUP_BALANCE),
            untouched_balance: U128(TOTAL_LOCKUP_BALANCE),
            total_claimed: U128(0)
        }
    );
    let res: bool = e.claim(&user).unwrap_json();
    assert!(res);
    assert_eq!(e.get_skyward_token_balance(&user), 0);
    assert_eq!(e.get_account(&user).unwrap().claimed_balance.0, 0);
    assert_eq!(
        e.get_stats(),
        Stats {
            token_account_id: SKYWARD_TOKEN_ID.to_string(),
            skyward_account_id: SKYWARD_ID.to_string(),
            claim_expiration_timestamp: CLAIM_EXPIRATION_TIMESTAMP,
            total_balance: U128(TOTAL_LOCKUP_BALANCE),
            untouched_balance: U128(TOTAL_LOCKUP_BALANCE - BALANCE_1),
            total_claimed: U128(0)
        }
    );

    e.root.borrow_runtime_mut().cur_block.block_timestamp =
        to_nano((TIMESTAMP_2 - TIMESTAMP_1) / 2 + TIMESTAMP_1);

    // Not registered for storage on token
    let res: bool = e.claim(&user).unwrap_json();
    assert!(!res);

    assert_eq!(e.get_skyward_token_balance(&user), 0);
    assert_eq!(e.get_account(&user).unwrap().claimed_balance.0, 0);

    assert_eq!(
        e.get_stats(),
        Stats {
            token_account_id: SKYWARD_TOKEN_ID.to_string(),
            skyward_account_id: SKYWARD_ID.to_string(),
            claim_expiration_timestamp: CLAIM_EXPIRATION_TIMESTAMP,
            total_balance: U128(TOTAL_LOCKUP_BALANCE),
            untouched_balance: U128(TOTAL_LOCKUP_BALANCE - BALANCE_1),
            total_claimed: U128(0)
        }
    );

    // Registering for storage
    storage_deposit(&user, SKYWARD_TOKEN_ID, &user.account_id);

    let res: bool = e.claim(&user).unwrap_json();
    assert!(res);

    assert_eq!(
        e.get_account(&user).unwrap().claimed_balance.0,
        BALANCE_1 / 2
    );
    assert_eq!(e.get_stats().total_claimed.0, BALANCE_1 / 2);
    assert_eq!(e.get_skyward_token_balance(&user), BALANCE_1 / 2);
}

#[test]
fn test_miss_claim() {
    let e = Env::init(3);
    e.root.borrow_runtime_mut().genesis.block_prod_time = 0;
    e.root.borrow_runtime_mut().cur_block.block_timestamp = to_nano(TIMESTAMP_1 - 100);
    assert_eq!(e.get_treasury_circulating_supply(), SKYWARD_TOTAL_SUPPLY);
    e.claim(&e.users[1]).assert_success();
    assert_eq!(e.get_account(&e.users[1]).unwrap().claimed_balance.0, 0);
    assert_eq!(
        e.get_stats(),
        Stats {
            token_account_id: SKYWARD_TOKEN_ID.to_string(),
            skyward_account_id: SKYWARD_ID.to_string(),
            claim_expiration_timestamp: CLAIM_EXPIRATION_TIMESTAMP,
            total_balance: U128(TOTAL_LOCKUP_BALANCE),
            untouched_balance: U128(BALANCE_1),
            total_claimed: U128(0)
        }
    );
    e.root.borrow_runtime_mut().cur_block.block_timestamp = to_nano(CLAIM_EXPIRATION_TIMESTAMP + 1);

    // Too late to claim
    assert!(!e.claim(&e.users[0]).is_ok());

    assert_eq!(
        e.get_stats(),
        Stats {
            token_account_id: SKYWARD_TOKEN_ID.to_string(),
            skyward_account_id: SKYWARD_ID.to_string(),
            claim_expiration_timestamp: CLAIM_EXPIRATION_TIMESTAMP,
            total_balance: U128(TOTAL_LOCKUP_BALANCE),
            untouched_balance: U128(BALANCE_1),
            total_claimed: U128(0)
        }
    );

    let skyward_initial_balance = e.skyward.account().unwrap().amount;
    let claim_initial_balance = e.skyward_claim.user_account.account().unwrap().amount;
    e.users[0]
        .function_call(e.skyward_claim.contract.donate_to_treasury(), DONATE_GAS, 0)
        .assert_success();
    let skyward_end_balance = e.skyward.account().unwrap().amount;
    let claim_end_balance = e.skyward_claim.user_account.account().unwrap().amount;
    assert!(claim_end_balance < claim_initial_balance);
    assert!(skyward_initial_balance < skyward_end_balance);
    let claim_balance_diff = claim_initial_balance - claim_end_balance;
    let skyward_balance_diff = skyward_end_balance - skyward_initial_balance;
    let gas_contract_reward_eps = to_yocto("0.001");
    assert!(claim_balance_diff > to_yocto("5"));
    assert!(claim_balance_diff + gas_contract_reward_eps > skyward_balance_diff);
    assert!(skyward_balance_diff + gas_contract_reward_eps > claim_balance_diff);

    assert_eq!(
        e.get_stats(),
        Stats {
            token_account_id: SKYWARD_TOKEN_ID.to_string(),
            skyward_account_id: SKYWARD_ID.to_string(),
            claim_expiration_timestamp: CLAIM_EXPIRATION_TIMESTAMP,
            total_balance: U128(BALANCE_2),
            untouched_balance: U128(0),
            total_claimed: U128(0)
        }
    );
    assert_eq!(
        e.get_treasury_circulating_supply(),
        SKYWARD_TOTAL_SUPPLY - BALANCE_1
    );
}
