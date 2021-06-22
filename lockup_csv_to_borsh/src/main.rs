use chrono::NaiveDate;
use near_sdk::borsh::{self, BorshSerialize};
use near_sdk::json_types::ValidAccountId;
use near_sdk::serde::Deserialize;
use near_sdk::{AccountId, CryptoHash};
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::Write;

#[derive(Debug, Deserialize)]
#[serde(crate = "near_sdk::serde")]
struct Record {
    pub account_id: ValidAccountId,
    pub start_date: String,
    pub cliff_date: String,
    pub end_date: String,
    pub balance: u128,
}

#[derive(BorshSerialize)]
pub struct FixedSizeAccount {
    pub account_hash: CryptoHash,
    pub start_timestamp: u32,
    pub cliff_timestamp: u32,
    pub end_timestamp: u32,
    pub balance: u128,
}

fn parse_date(s: &str) -> Option<u32> {
    let dt = NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()?;
    Some(dt.and_hms(0, 0, 0).timestamp() as u32)
}

fn hash_account(account_id: &AccountId) -> CryptoHash {
    use sha2::Digest;

    let value_hash = sha2::Sha256::digest(account_id.as_bytes());
    let mut res = CryptoHash::default();
    res.copy_from_slice(&value_hash);

    res
}

const MAX_PER_FILE: usize = 10000;

pub fn main() {
    let file_path = env::args_os()
        .nth(1)
        .expect("Missing input csv file name argument");
    let output_file_path = env::args_os()
        .nth(2)
        .expect("Missing output borsh file name argument")
        .into_string()
        .unwrap();
    let balance_multiplier: u128 = env::args_os()
        .nth(3)
        .map(|s| {
            s.into_string()
                .expect("Failed to parse balance multiplier")
                .parse()
                .unwrap()
        })
        .unwrap_or(1);
    assert!(
        balance_multiplier > 0,
        "Balance multiplier should be positive"
    );
    let file = File::open(file_path).unwrap();
    let mut rdr = csv::Reader::from_reader(file);
    let mut total_accounts: usize = 0;
    let mut total_balance: u128 = 0;
    let mut min_start_timestamp = u32::MAX;
    let mut max_end_timestamp = 0;
    let mut accounts = BTreeMap::new();
    for result in rdr.deserialize() {
        let Record {
            account_id,
            start_date,
            cliff_date,
            end_date,
            balance,
        } = result.unwrap();
        let account_id_str: AccountId = account_id.into();
        let start_timestamp = parse_date(&start_date).unwrap();
        let cliff_timestamp = parse_date(&cliff_date).unwrap_or(start_timestamp);
        let end_timestamp = parse_date(&end_date).unwrap();
        assert!(start_timestamp <= cliff_timestamp);
        assert!(cliff_timestamp <= end_timestamp);
        assert!(start_timestamp < end_timestamp);
        assert!(balance > 0);
        min_start_timestamp = std::cmp::min(min_start_timestamp, start_timestamp);
        max_end_timestamp = std::cmp::max(max_end_timestamp, end_timestamp);
        let account_hash = hash_account(&account_id_str);
        let balance = balance
            .checked_mul(balance_multiplier)
            .expect("Balance multiplication overflow");
        total_accounts += 1;
        total_balance = total_balance
            .checked_add(balance)
            .expect("Total balance overflow");
        println!(
            "{:30} -> {} {} {} -> {}",
            balance, start_timestamp, cliff_timestamp, end_timestamp, account_id_str
        );
        let account = FixedSizeAccount {
            account_hash,
            start_timestamp,
            cliff_timestamp,
            end_timestamp,
            balance,
        };
        assert!(accounts.insert(account_hash, account).is_none());
    }
    println!("Total number of accounts {}\nTotal balance: {}\nTotal multiplied balance: {}\nMinimum start timestamp: {}\nMaximum end timestamp: {}",
        total_accounts,
        total_balance / balance_multiplier,
        total_balance,
        min_start_timestamp,
        max_end_timestamp,
    );

    let mut index = 0;
    let values: Vec<_> = accounts.values().collect();
    for chunk in values.chunks(MAX_PER_FILE) {
        let output_file = format!("{}{}.borsh", output_file_path, index);
        let mut total_balance = 0;
        let mut data = vec![];
        for account in chunk {
            total_balance += account.balance;
            data.extend(account.try_to_vec().unwrap());
        }
        println!("File {}: balance {}", output_file, total_balance);
        let mut file = File::create(output_file).expect("Failed to create the output file");
        file.write_all(&data).expect("Failed to write data");
        index += 1;
    }
}
