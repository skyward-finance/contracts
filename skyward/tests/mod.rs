use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FT_METADATA_SPEC};
use near_sdk::env::STORAGE_PRICE_PER_BYTE;
use near_sdk::json_types::{ValidAccountId, WrappedBalance, U128};
use near_sdk::serde_json::json;
use near_sdk::test_utils::accounts;
use near_sdk::{env, AccountId, Balance, Gas};
use near_sdk_sim::runtime::GenesisConfig;
use near_sdk_sim::{deploy, init_simulator, to_yocto, ContractAccount, UserAccount};
use skyward::{
    ContractContract as SkywardContract, SaleInput, SaleInputOutToken, SaleOutput,
    SaleOutputOutToken, SubscriptionOutput,
};
use std::convert::TryInto;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    SKYWARD_WASM_BYTES => "res/skyward.wasm",

    FUNGIBLE_TOKEN_WASM_BYTES => "../common/fungible_token.wasm",
    W_NEAR_WASM_BYTES => "../common/w_near.wasm",
}

const NEAR: &str = "near";
const SKYWARD_ID: &str = "skyward.near";
const WRAP_NEAR_ID: &str = "wrap.near";
const SKYWARD_TOKEN_ID: &str = "token.skyward.near";
const SKYWARD_DAO_ID: &str = "skyward-dao.near";

const TOKEN1_ID: &str = "token1.near";

const BASE_GAS: Gas = 10_000_000_000_000;
const TON_OF_GAS: Gas = 50_000_000_000_000;
const SKYWARD_TOKEN_DECIMALS: u8 = 18;
const SKYWARD_TOKEN_BASE: Balance = 10u128.pow(SKYWARD_TOKEN_DECIMALS as u32);
const SKYWARD_TOTAL_SUPPLY: Balance = 1_000_000 * SKYWARD_TOKEN_BASE;
const ONE_NEAR: Balance = 10u128.pow(24);
const LISTING_FEE_NEAR: Balance = 10 * ONE_NEAR;
const DEFAULT_TOTAL_SUPPLY: Balance = 1_000_000_000 * ONE_NEAR;

const BLOCK_DURATION: u64 = 1_000_000_000;

pub struct Env {
    pub root: UserAccount,
    pub near: UserAccount,
    pub skyward_dao: UserAccount,
    pub skyward: ContractAccount<SkywardContract>,
    pub skyward_token: UserAccount,
    pub w_near: UserAccount,

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
        let w_near = near.deploy_and_init(
            &W_NEAR_WASM_BYTES,
            WRAP_NEAR_ID.to_string(),
            "new",
            b"{}",
            to_yocto("10"),
            BASE_GAS,
        );
        let skyward = deploy!(
            contract: SkywardContract,
            contract_id: SKYWARD_ID.to_string(),
            bytes: &SKYWARD_WASM_BYTES,
            signer_account: near,
            deposit: to_yocto("20"),
            gas: BASE_GAS,
            init_method: new(SKYWARD_TOKEN_ID.to_string().try_into().unwrap(), SKYWARD_TOTAL_SUPPLY.into(), LISTING_FEE_NEAR.into())
        );
        let skyward_token = skyward.user_account.deploy_and_init(
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
        // Registering tokens
        storage_deposit(&skyward_dao, WRAP_NEAR_ID, SKYWARD_ID);
        storage_deposit(&skyward_dao, SKYWARD_TOKEN_ID, SKYWARD_ID);
        let mut this = Self {
            root,
            near,
            skyward_dao,
            skyward,
            skyward_token,
            w_near,
            users: vec![],
        };
        this.init_users(num_users);
        this
    }

    pub fn deploy_ft(&self, owner_id: &str, token_account_id: &str) -> UserAccount {
        let token = self.near.deploy_and_init(
            &FUNGIBLE_TOKEN_WASM_BYTES,
            token_account_id.to_string(),
            "new_default_meta",
            &json!({
                "owner_id": owner_id.to_string(),
                "total_supply": U128::from(DEFAULT_TOTAL_SUPPLY)
            })
            .to_string()
            .into_bytes(),
            to_yocto("10"),
            BASE_GAS,
        );
        storage_deposit(&self.near, token_account_id, SKYWARD_ID);
        token
    }

    pub fn wrap_near(&self, user: &UserAccount, amount: Balance) {
        user.call(
            self.w_near.account_id.clone(),
            "near_deposit",
            &json!({
                "account_id": user.valid_account_id()
            })
            .to_string()
            .into_bytes(),
            BASE_GAS,
            amount,
        )
        .assert_success();
    }

    pub fn register_skyward_token(&self, user: &UserAccount) {
        user.function_call(
            self.skyward
                .contract
                .register_token(None, self.skyward_token.valid_account_id()),
            BASE_GAS,
            to_yocto("0.01"),
        )
        .assert_success();
    }

    pub fn register_and_deposit(&self, user: &UserAccount, token: &UserAccount, amount: Balance) {
        user.function_call(
            self.skyward
                .contract
                .register_token(None, token.valid_account_id()),
            BASE_GAS,
            to_yocto("0.01"),
        )
        .assert_success();

        user.call(
            token.account_id.clone(),
            "ft_transfer_call",
            &json!({
                "receiver_id": self.skyward.user_account.valid_account_id(),
                "amount": U128::from(amount),
                "msg": "\"AccountDeposit\""
            })
            .to_string()
            .into_bytes(),
            TON_OF_GAS,
            1,
        )
        .assert_success();
    }

    pub fn init_users(&mut self, num_users: usize) {
        for i in 0..num_users {
            let user = self.root.create_user(accounts(i).into(), to_yocto("100"));
            self.wrap_near(&user, to_yocto("20"));
            self.register_and_deposit(&user, &self.w_near, to_yocto("10"));
            self.users.push(user);
        }
    }

    pub fn sale_create(
        &self,
        user: &UserAccount,
        tokens: &[(&UserAccount, Balance)],
    ) -> SaleOutput {
        let current_time = user.borrow_runtime().current_block().block_timestamp;
        let start_time = current_time + BLOCK_DURATION * 15;
        let sale_duration = BLOCK_DURATION * 60;

        let initial_balance = user.account().unwrap().amount;

        let res = user.function_call(
            self.skyward.contract.sale_create(SaleInput {
                out_tokens: tokens
                    .iter()
                    .map(|(token, balance)| SaleInputOutToken {
                        token_account_id: token.valid_account_id(),
                        balance: (*balance).into(),
                    })
                    .collect(),
                in_token_account_id: self.w_near.valid_account_id(),
                start_time: start_time.into(),
                duration: sale_duration.into(),
            }),
            BASE_GAS,
            to_yocto("1") + LISTING_FEE_NEAR,
        );
        res.assert_success();

        let balance_spent = initial_balance - user.account().unwrap().amount;
        // Should be listing fee plus some for storage. The rest should be refunded.
        assert!(
            LISTING_FEE_NEAR < balance_spent && balance_spent < LISTING_FEE_NEAR + to_yocto("0.02")
        );

        let sale_id: u64 = res.unwrap_json();
        self.get_sale(sale_id, None)
    }

    pub fn get_sale(&self, sale_id: u64, account_id: Option<ValidAccountId>) -> SaleOutput {
        let sale: Option<SaleOutput> = self
            .near
            .view_method_call(self.skyward.contract.get_sale(sale_id, account_id))
            .unwrap_json();
        sale.unwrap()
    }

    pub fn balances_of(&self, user: &UserAccount) -> Vec<(AccountId, Balance)> {
        let res: Vec<(AccountId, WrappedBalance)> = user
            .view_method_call(self.skyward.contract.balances_of(
                user.valid_account_id(),
                None,
                None,
            ))
            .unwrap_json();
        res.into_iter().map(|(a, b)| (a, b.0)).collect()
    }

    pub fn get_treasury_balances(&self) -> Vec<(AccountId, Balance)> {
        let res: Vec<(AccountId, WrappedBalance)> = self
            .near
            .view_method_call(self.skyward.contract.get_treasury_balances(None, None))
            .unwrap_json();
        res.into_iter().map(|(a, b)| (a, b.0)).collect()
    }

    pub fn skyward_total_supply(&self) -> Balance {
        let res: WrappedBalance = self
            .near
            .view_method_call(self.skyward.contract.get_skyward_total_supply())
            .unwrap_json();
        res.into()
    }

    pub fn get_token_balance(&self, token: &UserAccount, user: &UserAccount) -> Balance {
        let balance: Option<WrappedBalance> = self
            .near
            .view(
                token.account_id.clone(),
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
}

#[test]
fn test_init() {
    Env::init(0);
}

#[test]
fn test_account_deposit() {
    let e = Env::init(1);
    let alice = e.users.get(0).unwrap();

    assert_eq!(
        e.balances_of(alice),
        vec![(e.w_near.account_id.clone(), to_yocto("10"))]
    );
}

#[test]
fn test_create_sale() {
    let e = Env::init(1);
    let alice = e.users.get(0).unwrap();

    let token1 = e.deploy_ft(&alice.account_id, TOKEN1_ID);
    e.register_and_deposit(&alice, &token1, to_yocto("10000"));

    let sale = e.sale_create(alice, &[(&token1, to_yocto("4000"))]);

    assert_eq!(
        sale,
        SaleOutput {
            sale_id: 0,
            owner_id: alice.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: token1.account_id.clone(),
                remaining: to_yocto("3600").into(),
                distributed: 0.into(),
                treasury_unclaimed: Some(0.into())
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: U128(0),
            in_token_paid_unclaimed: U128(0),
            in_token_paid: U128(0),
            total_shares: U128(0),
            start_time: sale.start_time,
            duration: sale.duration.clone(),
            remaining_duration: sale.duration.clone(),
            subscription: None,
            associated_sale_id: Some(1),
        }
    );

    let in_skyward_sale = e.get_sale(1, None);
    assert_eq!(
        in_skyward_sale,
        SaleOutput {
            sale_id: 1,
            owner_id: alice.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: token1.account_id.clone(),
                remaining: to_yocto("400").into(),
                distributed: 0.into(),
                treasury_unclaimed: Some(0.into()),
            }],
            in_token_account_id: e.skyward_token.account_id.clone(),
            in_token_remaining: U128(0),
            in_token_paid_unclaimed: U128(0),
            in_token_paid: U128(0),
            total_shares: U128(0),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: sale.duration.clone(),
            associated_sale_id: Some(0),
            subscription: None,
        }
    );

    assert_eq!(
        e.balances_of(alice),
        vec![
            (e.w_near.account_id.clone(), to_yocto("10")),
            (token1.account_id.clone(), to_yocto("6000")),
            (e.skyward_token.account_id.clone(), 0)
        ]
    );
}

#[test]
fn test_join_sale() {
    let e = Env::init(2);
    let alice = e.users.get(0).unwrap();
    let bob = e.users.get(1).unwrap();

    let token1 = e.deploy_ft(&alice.account_id, TOKEN1_ID);
    e.register_and_deposit(&alice, &token1, to_yocto("10000"));

    let sale = e.sale_create(alice, &[(&token1, to_yocto("4000"))]);

    bob.function_call(
        e.skyward
            .contract
            .sale_deposit_in_token(sale.sale_id, to_yocto("4").into(), None),
        BASE_GAS,
        to_yocto("0.01"),
    )
    .assert_success();

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(bobs_sale.in_token_remaining.0, to_yocto("4"));
    assert_eq!(bobs_sale.total_shares.0, to_yocto("4"));
    assert_eq!(
        bobs_sale.subscription,
        Some(SubscriptionOutput {
            remaining_in_balance: to_yocto("4").into(),
            unclaimed_out_balances: vec![U128(0)],
            shares: to_yocto("4").into(),
        })
    );

    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("6")),
            (token1.account_id.clone(), 0),
        ]
    );

    {
        let mut runtime = e.near.borrow_runtime_mut();
        let current_time = runtime.current_block().block_timestamp;
        runtime
            .produce_blocks(
                (sale.start_time.0 + sale.duration.0 / 2 - current_time) / BLOCK_DURATION,
            )
            .expect("Producing blocks");
    }

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: alice.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: token1.account_id.clone(),
                remaining: to_yocto("1800").into(),
                distributed: to_yocto("1800").into(),
                treasury_unclaimed: Some(to_yocto("18").into()),
            },],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("2").into(),
            in_token_paid_unclaimed: to_yocto("2").into(),
            in_token_paid: to_yocto("2").into(),
            total_shares: to_yocto("4").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: (sale.duration.0 / 2).into(),
            associated_sale_id: Some(1),
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: to_yocto("2").into(),
                unclaimed_out_balances: vec![to_yocto("1782").into()],
                shares: to_yocto("4").into(),
            }),
        }
    );

    e.near
        .borrow_runtime_mut()
        .produce_blocks(bobs_sale.remaining_duration.0 / BLOCK_DURATION)
        .expect("Producing blocks");

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: alice.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: token1.account_id.clone(),
                remaining: 0.into(),
                distributed: to_yocto("3600").into(),
                treasury_unclaimed: Some(to_yocto("36").into()),
            },],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("0").into(),
            in_token_paid_unclaimed: to_yocto("4").into(),
            in_token_paid: to_yocto("4").into(),
            total_shares: to_yocto("4").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: Some(1),
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: to_yocto("0").into(),
                unclaimed_out_balances: vec![to_yocto("3564").into()],
                shares: to_yocto("4").into(),
            }),
        }
    );

    assert_eq!(
        e.get_treasury_balances(),
        vec![
            (e.w_near.account_id.clone(), 0),
            (token1.account_id.clone(), 0),
        ]
    );

    alice
        .function_call(
            e.skyward.contract.sale_distribute_unclaimed_tokens(0),
            BASE_GAS,
            0,
        )
        .assert_success();

    assert_eq!(
        e.balances_of(alice),
        vec![
            (e.w_near.account_id.clone(), to_yocto("13.96")),
            (token1.account_id.clone(), to_yocto("6000")),
            (e.skyward_token.account_id.clone(), 0),
        ]
    );
    assert_eq!(
        e.get_treasury_balances(),
        vec![
            (e.w_near.account_id.clone(), to_yocto("0.04")),
            (token1.account_id.clone(), to_yocto("36")),
        ]
    );
    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("6")),
            (token1.account_id.clone(), 0),
        ]
    );

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: alice.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: token1.account_id.clone(),
                remaining: 0.into(),
                distributed: to_yocto("3600").into(),
                treasury_unclaimed: Some(0.into()),
            },],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("0").into(),
            in_token_paid_unclaimed: to_yocto("0").into(),
            in_token_paid: to_yocto("4").into(),
            total_shares: to_yocto("4").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: Some(1),
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: to_yocto("0").into(),
                unclaimed_out_balances: vec![to_yocto("3564").into()],
                shares: to_yocto("4").into(),
            }),
        }
    );

    bob.function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
        .assert_success();

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: alice.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: token1.account_id.clone(),
                remaining: 0.into(),
                distributed: to_yocto("3600").into(),
                treasury_unclaimed: Some(0.into()),
            },],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("0").into(),
            in_token_paid_unclaimed: to_yocto("0").into(),
            in_token_paid: to_yocto("4").into(),
            total_shares: to_yocto("0").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: Some(1),
            subscription: None,
        }
    );

    assert_eq!(
        e.get_treasury_balances(),
        vec![
            (e.w_near.account_id.clone(), to_yocto("0.04")),
            (token1.account_id.clone(), to_yocto("36")),
        ]
    );
    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("6")),
            (token1.account_id.clone(), to_yocto("3564")),
        ]
    );
}

#[test]
fn test_join_in_skyward_sale() {
    let e = Env::init(2);
    let alice = e.users.get(0).unwrap();
    let bob = e.users.get(1).unwrap();

    let token1 = e.deploy_ft(&alice.account_id, TOKEN1_ID);
    assert_eq!(e.get_token_balance(&token1, alice), DEFAULT_TOTAL_SUPPLY);
    e.register_and_deposit(&alice, &token1, to_yocto("10000"));
    assert_eq!(
        e.get_token_balance(&token1, alice),
        DEFAULT_TOTAL_SUPPLY - to_yocto("10000")
    );

    // Bob's deposit
    let bobs_amount = 100 * SKYWARD_TOKEN_BASE;
    storage_deposit(bob, &e.skyward_token.account_id, &bob.account_id);
    assert_eq!(e.get_token_balance(&e.skyward_token, bob), 0);
    e.skyward_dao
        .call(
            e.skyward_token.account_id.clone(),
            "ft_transfer",
            &json!({
                "receiver_id": bob.valid_account_id(),
                "amount": U128::from(bobs_amount),
            })
            .to_string()
            .into_bytes(),
            BASE_GAS,
            1,
        )
        .assert_success();
    assert_eq!(e.get_token_balance(&e.skyward_token, bob), bobs_amount);
    e.register_and_deposit(&bob, &e.skyward_token, bobs_amount);
    assert_eq!(e.get_token_balance(&e.skyward_token, bob), 0);

    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("10")),
            (e.skyward_token.account_id.clone(), bobs_amount),
        ]
    );

    let sale = e.sale_create(alice, &[(&token1, to_yocto("4000"))]);

    bob.function_call(
        e.skyward
            .contract
            .sale_deposit_in_token(1, bobs_amount.into(), None),
        BASE_GAS,
        to_yocto("0.01"),
    )
    .assert_success();

    let bobs_sale = e.get_sale(1, Some(bob.valid_account_id()));
    assert_eq!(bobs_sale.in_token_remaining.0, bobs_amount);
    assert_eq!(bobs_sale.total_shares.0, bobs_amount);
    assert_eq!(
        bobs_sale.subscription,
        Some(SubscriptionOutput {
            remaining_in_balance: bobs_amount.into(),
            unclaimed_out_balances: vec![U128(0)],
            shares: bobs_amount.into(),
        })
    );

    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("10")),
            (e.skyward_token.account_id.clone(), 0),
            (token1.account_id.clone(), 0),
        ]
    );

    {
        let mut runtime = e.near.borrow_runtime_mut();
        let current_time = runtime.current_block().block_timestamp;
        runtime
            .produce_blocks(
                (sale.start_time.0 + sale.duration.0 / 2 - current_time) / BLOCK_DURATION,
            )
            .expect("Producing blocks");
    }

    let bobs_sale = e.get_sale(1, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 1,
            owner_id: alice.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: token1.account_id.clone(),
                remaining: to_yocto("200").into(),
                distributed: to_yocto("200").into(),
                treasury_unclaimed: Some(to_yocto("2").into()),
            },],
            in_token_account_id: e.skyward_token.account_id.clone(),
            in_token_remaining: (bobs_amount / 2).into(),
            in_token_paid_unclaimed: (bobs_amount / 2).into(),
            in_token_paid: (bobs_amount / 2).into(),
            total_shares: bobs_amount.into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: (sale.duration.0 / 2).into(),
            associated_sale_id: Some(0),
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: (bobs_amount / 2).into(),
                unclaimed_out_balances: vec![to_yocto("198").into()],
                shares: bobs_amount.into(),
            }),
        }
    );

    e.near
        .borrow_runtime_mut()
        .produce_blocks(bobs_sale.remaining_duration.0 / BLOCK_DURATION)
        .expect("Producing blocks");

    let bobs_sale = e.get_sale(1, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 1,
            owner_id: alice.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: token1.account_id.clone(),
                remaining: 0.into(),
                distributed: to_yocto("400").into(),
                treasury_unclaimed: Some(to_yocto("4").into()),
            },],
            in_token_account_id: e.skyward_token.account_id.clone(),
            in_token_remaining: 0.into(),
            in_token_paid_unclaimed: bobs_amount.into(),
            in_token_paid: bobs_amount.into(),
            total_shares: bobs_amount.into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: Some(0),
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: 0.into(),
                unclaimed_out_balances: vec![to_yocto("396").into()],
                shares: bobs_amount.into(),
            }),
        }
    );

    assert_eq!(
        e.get_treasury_balances(),
        vec![
            (e.w_near.account_id.clone(), 0),
            (token1.account_id.clone(), 0),
        ]
    );

    alice
        .function_call(
            e.skyward.contract.sale_distribute_unclaimed_tokens(1),
            BASE_GAS,
            0,
        )
        .assert_success();

    assert_eq!(
        e.balances_of(alice),
        vec![
            (e.w_near.account_id.clone(), to_yocto("10")),
            (token1.account_id.clone(), to_yocto("6000")),
            (e.skyward_token.account_id.clone(), bobs_amount),
        ]
    );
    assert_eq!(
        e.get_treasury_balances(),
        vec![
            (e.w_near.account_id.clone(), 0),
            (token1.account_id.clone(), to_yocto("4")),
        ]
    );

    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("10")),
            (e.skyward_token.account_id.clone(), 0),
            (token1.account_id.clone(), 0),
        ]
    );

    let bobs_sale = e.get_sale(1, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 1,
            owner_id: alice.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: token1.account_id.clone(),
                remaining: 0.into(),
                distributed: to_yocto("400").into(),
                treasury_unclaimed: Some(0.into()),
            },],
            in_token_account_id: e.skyward_token.account_id.clone(),
            in_token_remaining: 0.into(),
            in_token_paid_unclaimed: 0.into(),
            in_token_paid: bobs_amount.into(),
            total_shares: bobs_amount.into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: Some(0),
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: 0.into(),
                unclaimed_out_balances: vec![to_yocto("396").into()],
                shares: bobs_amount.into(),
            }),
        }
    );

    bob.function_call(e.skyward.contract.sale_claim_out_tokens(1), BASE_GAS, 0)
        .assert_success();

    let bobs_sale = e.get_sale(1, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 1,
            owner_id: alice.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: token1.account_id.clone(),
                remaining: 0.into(),
                distributed: to_yocto("400").into(),
                treasury_unclaimed: Some(0.into()),
            },],
            in_token_account_id: e.skyward_token.account_id.clone(),
            in_token_remaining: 0.into(),
            in_token_paid_unclaimed: 0.into(),
            in_token_paid: bobs_amount.into(),
            total_shares: 0.into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: Some(0),
            subscription: None,
        }
    );

    assert_eq!(
        e.get_treasury_balances(),
        vec![
            (e.w_near.account_id.clone(), 0),
            (token1.account_id.clone(), to_yocto("4")),
        ]
    );
    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("10")),
            (e.skyward_token.account_id.clone(), 0),
            (token1.account_id.clone(), to_yocto("396")),
        ]
    );

    storage_deposit(bob, &token1.account_id, &bob.account_id);
    assert_eq!(e.get_token_balance(&token1, bob), 0);

    bob.function_call(
        e.skyward
            .contract
            .withdraw_token(token1.valid_account_id(), to_yocto("100").into()),
        TON_OF_GAS,
        1,
    )
    .assert_success();
    assert_eq!(e.get_token_balance(&token1, bob), to_yocto("100"));

    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("10")),
            (e.skyward_token.account_id.clone(), 0),
            (token1.account_id.clone(), to_yocto("296")),
        ]
    );
}

#[test]
fn test_join_sale_with_referral() {
    let e = Env::init(2);
    let alice = e.users.get(0).unwrap();
    let bob = e.users.get(1).unwrap();

    let sale_amount = 10000 * SKYWARD_TOKEN_BASE;
    e.register_and_deposit(&e.skyward_dao, &e.skyward_token, sale_amount * 2);

    e.register_skyward_token(alice);

    let sale = e.sale_create(&e.skyward_dao, &[(&e.skyward_token, sale_amount)]);

    assert_eq!(
        e.balances_of(&e.skyward_dao),
        vec![
            (e.skyward_token.account_id.clone(), sale_amount),
            (e.w_near.account_id.clone(), to_yocto("0")),
        ]
    );

    bob.function_call(
        e.skyward.contract.sale_deposit_in_token(
            sale.sale_id,
            to_yocto("4").into(),
            Some(alice.valid_account_id()),
        ),
        BASE_GAS,
        to_yocto("0.01"),
    )
    .assert_success();

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: e.skyward_dao.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: e.skyward_token.account_id.clone(),
                remaining: sale_amount.into(),
                distributed: 0.into(),
                treasury_unclaimed: None,
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("4").into(),
            in_token_paid_unclaimed: to_yocto("0").into(),
            in_token_paid: to_yocto("0").into(),
            total_shares: to_yocto("4").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: sale.duration.clone(),
            associated_sale_id: None,
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: to_yocto("4").into(),
                unclaimed_out_balances: vec![U128(0)],
                shares: to_yocto("4").into(),
            }),
        }
    );

    assert_eq!(
        e.balances_of(alice),
        vec![
            (e.w_near.account_id.clone(), to_yocto("10")),
            (e.skyward_token.account_id.clone(), 0),
        ]
    );
    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("6")),
            (e.skyward_token.account_id.clone(), 0),
        ]
    );

    {
        let mut runtime = e.near.borrow_runtime_mut();
        let current_time = runtime.current_block().block_timestamp;
        runtime
            .produce_blocks(
                (sale.start_time.0 + sale.duration.0 / 2 - current_time) / BLOCK_DURATION,
            )
            .expect("Producing blocks");
    }

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: e.skyward_dao.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: e.skyward_token.account_id.clone(),
                remaining: (sale_amount / 2).into(),
                distributed: (sale_amount / 2).into(),
                treasury_unclaimed: None,
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("2").into(),
            in_token_paid_unclaimed: to_yocto("2").into(),
            in_token_paid: to_yocto("2").into(),
            total_shares: to_yocto("4").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: (sale.duration.0 / 2).into(),
            associated_sale_id: None,
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: to_yocto("2").into(),
                unclaimed_out_balances: vec![(sale_amount / 2).into()],
                shares: to_yocto("4").into(),
            }),
        }
    );

    e.near
        .borrow_runtime_mut()
        .produce_blocks(bobs_sale.remaining_duration.0 / BLOCK_DURATION)
        .expect("Producing blocks");

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: e.skyward_dao.account_id.clone(),

            out_tokens: vec![SaleOutputOutToken {
                token_account_id: e.skyward_token.account_id.clone(),
                remaining: 0.into(),
                distributed: sale_amount.into(),
                treasury_unclaimed: None,
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("0").into(),
            in_token_paid_unclaimed: to_yocto("4").into(),
            in_token_paid: to_yocto("4").into(),
            total_shares: to_yocto("4").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: None,
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: to_yocto("0").into(),
                unclaimed_out_balances: vec![sale_amount.into()],
                shares: to_yocto("4").into(),
            }),
        }
    );

    assert_eq!(
        e.get_treasury_balances(),
        vec![(e.w_near.account_id.clone(), 0),]
    );

    alice
        .function_call(
            e.skyward.contract.sale_distribute_unclaimed_tokens(0),
            BASE_GAS,
            0,
        )
        .assert_success();

    assert_eq!(
        e.balances_of(&e.skyward_dao),
        vec![
            (e.skyward_token.account_id.clone(), sale_amount),
            (e.w_near.account_id.clone(), to_yocto("3.96")),
        ]
    );

    assert_eq!(
        e.balances_of(alice),
        vec![
            (e.w_near.account_id.clone(), to_yocto("10")),
            (e.skyward_token.account_id.clone(), 0),
        ]
    );
    assert_eq!(
        e.get_treasury_balances(),
        vec![(e.w_near.account_id.clone(), to_yocto("0.04")),]
    );
    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("6")),
            (e.skyward_token.account_id.clone(), 0),
        ]
    );

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: e.skyward_dao.account_id.clone(),

            out_tokens: vec![SaleOutputOutToken {
                token_account_id: e.skyward_token.account_id.clone(),
                remaining: 0.into(),
                distributed: sale_amount.into(),
                treasury_unclaimed: None,
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("0").into(),
            in_token_paid_unclaimed: to_yocto("0").into(),
            in_token_paid: to_yocto("4").into(),
            total_shares: to_yocto("4").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: None,
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: to_yocto("0").into(),
                unclaimed_out_balances: vec![sale_amount.into()],
                shares: to_yocto("4").into(),
            }),
        }
    );

    bob.function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
        .assert_success();

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: e.skyward_dao.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: e.skyward_token.account_id.clone(),
                remaining: 0.into(),
                distributed: sale_amount.into(),
                treasury_unclaimed: None,
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("0").into(),
            in_token_paid_unclaimed: to_yocto("0").into(),
            in_token_paid: to_yocto("4").into(),
            total_shares: to_yocto("0").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: None,
            subscription: None,
        }
    );

    assert_eq!(
        e.get_treasury_balances(),
        vec![(e.w_near.account_id.clone(), to_yocto("0.04")),]
    );
    assert_eq!(
        e.balances_of(alice),
        vec![
            (e.w_near.account_id.clone(), to_yocto("10")),
            (e.skyward_token.account_id.clone(), sale_amount / 100),
        ]
    );
    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("6")),
            (e.skyward_token.account_id.clone(), sale_amount / 100 * 99),
        ]
    );
    assert_eq!(e.skyward_total_supply(), SKYWARD_TOTAL_SUPPLY);
}

#[test]
fn test_join_sale_with_referral_and_alice() {
    let e = Env::init(2);
    let alice = e.users.get(0).unwrap();
    let bob = e.users.get(1).unwrap();

    let sale_amount = 10000 * SKYWARD_TOKEN_BASE;
    e.register_and_deposit(&e.skyward_dao, &e.skyward_token, sale_amount * 2);

    e.register_skyward_token(alice);
    assert_eq!(e.skyward_total_supply(), SKYWARD_TOTAL_SUPPLY);

    let sale = e.sale_create(&e.skyward_dao, &[(&e.skyward_token, sale_amount)]);
    assert_eq!(e.skyward_total_supply(), SKYWARD_TOTAL_SUPPLY);

    assert_eq!(
        e.balances_of(&e.skyward_dao),
        vec![
            (e.skyward_token.account_id.clone(), sale_amount),
            (e.w_near.account_id.clone(), to_yocto("0")),
        ]
    );

    bob.function_call(
        e.skyward.contract.sale_deposit_in_token(
            sale.sale_id,
            to_yocto("4").into(),
            Some(alice.valid_account_id()),
        ),
        BASE_GAS,
        to_yocto("0.01"),
    )
    .assert_success();

    alice
        .function_call(
            e.skyward
                .contract
                .sale_deposit_in_token(sale.sale_id, to_yocto("1").into(), None),
            BASE_GAS,
            to_yocto("0.01"),
        )
        .assert_success();

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: e.skyward_dao.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: e.skyward_token.account_id.clone(),
                remaining: sale_amount.into(),
                distributed: 0.into(),
                treasury_unclaimed: None,
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("5").into(),
            in_token_paid_unclaimed: to_yocto("0").into(),
            in_token_paid: to_yocto("0").into(),
            total_shares: to_yocto("5").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: sale.duration.clone(),
            associated_sale_id: None,
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: to_yocto("4").into(),
                unclaimed_out_balances: vec![U128(0)],
                shares: to_yocto("4").into(),
            }),
        }
    );

    assert_eq!(
        e.get_sale(0, Some(alice.valid_account_id())).subscription,
        Some(SubscriptionOutput {
            remaining_in_balance: to_yocto("1").into(),
            unclaimed_out_balances: vec![U128(0)],
            shares: to_yocto("1").into(),
        }),
    );

    assert_eq!(
        e.balances_of(alice),
        vec![
            (e.w_near.account_id.clone(), to_yocto("9")),
            (e.skyward_token.account_id.clone(), 0),
        ]
    );
    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("6")),
            (e.skyward_token.account_id.clone(), 0),
        ]
    );

    {
        let mut runtime = e.near.borrow_runtime_mut();
        let current_time = runtime.current_block().block_timestamp;
        runtime
            .produce_blocks(
                (sale.start_time.0 + sale.duration.0 / 2 - current_time) / BLOCK_DURATION,
            )
            .expect("Producing blocks");
    }

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: e.skyward_dao.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: e.skyward_token.account_id.clone(),
                remaining: (sale_amount / 2).into(),
                distributed: (sale_amount / 2).into(),
                treasury_unclaimed: None,
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("2.5").into(),
            in_token_paid_unclaimed: to_yocto("2.5").into(),
            in_token_paid: to_yocto("2.5").into(),
            total_shares: to_yocto("5").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: (sale.duration.0 / 2).into(),
            associated_sale_id: None,
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: to_yocto("2").into(),
                unclaimed_out_balances: vec![(sale_amount / 5 * 4 / 2).into()],
                shares: to_yocto("4").into(),
            }),
        }
    );
    assert_eq!(
        e.get_sale(0, Some(alice.valid_account_id())).subscription,
        Some(SubscriptionOutput {
            remaining_in_balance: to_yocto("0.5").into(),
            unclaimed_out_balances: vec![(sale_amount / 5 * 1 / 2).into()],
            shares: to_yocto("1").into(),
        }),
    );

    e.near
        .borrow_runtime_mut()
        .produce_blocks(bobs_sale.remaining_duration.0 / BLOCK_DURATION)
        .expect("Producing blocks");

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: e.skyward_dao.account_id.clone(),

            out_tokens: vec![SaleOutputOutToken {
                token_account_id: e.skyward_token.account_id.clone(),
                remaining: 0.into(),
                distributed: sale_amount.into(),
                treasury_unclaimed: None,
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("0").into(),
            in_token_paid_unclaimed: to_yocto("5").into(),
            in_token_paid: to_yocto("5").into(),
            total_shares: to_yocto("5").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: None,
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: 0.into(),
                unclaimed_out_balances: vec![(sale_amount / 5 * 4).into()],
                shares: to_yocto("4").into(),
            }),
        }
    );
    assert_eq!(
        e.get_sale(0, Some(alice.valid_account_id())).subscription,
        Some(SubscriptionOutput {
            remaining_in_balance: 0.into(),
            unclaimed_out_balances: vec![(sale_amount / 5).into()],
            shares: to_yocto("1").into(),
        }),
    );

    assert_eq!(
        e.get_treasury_balances(),
        vec![(e.w_near.account_id.clone(), 0),]
    );
    assert_eq!(e.skyward_total_supply(), SKYWARD_TOTAL_SUPPLY);

    alice
        .function_call(
            e.skyward.contract.sale_distribute_unclaimed_tokens(0),
            BASE_GAS,
            0,
        )
        .assert_success();

    assert_eq!(
        e.balances_of(&e.skyward_dao),
        vec![
            (e.skyward_token.account_id.clone(), sale_amount),
            (e.w_near.account_id.clone(), to_yocto("4.95")),
        ]
    );

    assert_eq!(
        e.balances_of(alice),
        vec![
            (e.w_near.account_id.clone(), to_yocto("9")),
            (e.skyward_token.account_id.clone(), 0),
        ]
    );
    assert_eq!(
        e.get_treasury_balances(),
        vec![(e.w_near.account_id.clone(), to_yocto("0.05")),]
    );
    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("6")),
            (e.skyward_token.account_id.clone(), 0),
        ]
    );

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: e.skyward_dao.account_id.clone(),

            out_tokens: vec![SaleOutputOutToken {
                token_account_id: e.skyward_token.account_id.clone(),
                remaining: 0.into(),
                distributed: sale_amount.into(),
                treasury_unclaimed: None,
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("0").into(),
            in_token_paid_unclaimed: to_yocto("0").into(),
            in_token_paid: to_yocto("5").into(),
            total_shares: to_yocto("5").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: None,
            subscription: Some(SubscriptionOutput {
                remaining_in_balance: to_yocto("0").into(),
                unclaimed_out_balances: vec![(sale_amount / 5 * 4).into()],
                shares: to_yocto("4").into(),
            }),
        }
    );
    assert_eq!(
        e.get_sale(0, Some(alice.valid_account_id())).subscription,
        Some(SubscriptionOutput {
            remaining_in_balance: 0.into(),
            unclaimed_out_balances: vec![(sale_amount / 5).into()],
            shares: to_yocto("1").into(),
        }),
    );

    assert_eq!(e.skyward_total_supply(), SKYWARD_TOTAL_SUPPLY);

    bob.function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
        .assert_success();

    let bobs_sale = e.get_sale(0, Some(bob.valid_account_id()));
    assert_eq!(
        bobs_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: e.skyward_dao.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: e.skyward_token.account_id.clone(),
                remaining: 0.into(),
                distributed: sale_amount.into(),
                treasury_unclaimed: None,
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("0").into(),
            in_token_paid_unclaimed: to_yocto("0").into(),
            in_token_paid: to_yocto("5").into(),
            total_shares: to_yocto("1").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: None,
            subscription: None,
        }
    );
    assert_eq!(
        e.get_sale(0, Some(alice.valid_account_id())).subscription,
        Some(SubscriptionOutput {
            remaining_in_balance: 0.into(),
            unclaimed_out_balances: vec![(sale_amount / 5).into()],
            shares: to_yocto("1").into(),
        }),
    );

    assert_eq!(e.skyward_total_supply(), SKYWARD_TOTAL_SUPPLY);
    assert_eq!(
        e.get_treasury_balances(),
        vec![(e.w_near.account_id.clone(), to_yocto("0.05")),]
    );
    assert_eq!(
        e.balances_of(alice),
        vec![
            (e.w_near.account_id.clone(), to_yocto("9")),
            (
                e.skyward_token.account_id.clone(),
                sale_amount / 5 * 4 / 100
            ),
        ]
    );
    assert_eq!(
        e.balances_of(bob),
        vec![
            (e.w_near.account_id.clone(), to_yocto("6")),
            (
                e.skyward_token.account_id.clone(),
                sale_amount / 5 * 4 / 100 * 99
            ),
        ]
    );

    alice
        .function_call(e.skyward.contract.sale_claim_out_tokens(0), BASE_GAS, 0)
        .assert_success();

    let alice_sale = e.get_sale(0, Some(alice.valid_account_id()));
    assert_eq!(
        alice_sale,
        SaleOutput {
            sale_id: 0,
            owner_id: e.skyward_dao.account_id.clone(),
            out_tokens: vec![SaleOutputOutToken {
                token_account_id: e.skyward_token.account_id.clone(),
                remaining: 0.into(),
                distributed: sale_amount.into(),
                treasury_unclaimed: None,
            }],
            in_token_account_id: e.w_near.account_id.clone(),
            in_token_remaining: to_yocto("0").into(),
            in_token_paid_unclaimed: to_yocto("0").into(),
            in_token_paid: to_yocto("5").into(),
            total_shares: to_yocto("0").into(),
            start_time: sale.start_time.clone(),
            duration: sale.duration.clone(),
            remaining_duration: 0.into(),
            associated_sale_id: None,
            subscription: None,
        }
    );

    assert_eq!(
        e.skyward_total_supply(),
        SKYWARD_TOTAL_SUPPLY - sale_amount / 5 / 100
    );

    assert_eq!(
        e.balances_of(alice),
        vec![
            (e.w_near.account_id.clone(), to_yocto("9")),
            (
                e.skyward_token.account_id.clone(),
                sale_amount / 5 * 99 / 100 + sale_amount / 5 * 4 / 100
            ),
        ]
    );
    assert_eq!(
        e.get_treasury_balances(),
        vec![(e.w_near.account_id.clone(), to_yocto("0.05")),]
    );
}
