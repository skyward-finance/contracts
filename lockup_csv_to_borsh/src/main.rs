use chrono::NaiveDate;
use near_sdk::borsh::{self, BorshSerialize};
use near_sdk::json_types::ValidAccountId;
use near_sdk::serde::Deserialize;
use near_sdk::AccountId;
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
    pub account_len: u8,
    pub account_id: [u8; 64],
    pub start_timestamp: u32,
    pub cliff_timestamp: u32,
    pub end_timestamp: u32,
    pub balance: u128,
}

fn parse_date(s: &str) -> Option<u32> {
    let dt = NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()?;
    Some(dt.and_hms(0, 0, 0).timestamp() as u32)
}

pub fn main() {
    let file_path = env::args_os()
        .nth(1)
        .expect("Missing input csv file name argument");
    let output_file_path = env::args_os()
        .nth(2)
        .expect("Missing output borsh file name argument");
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
    let mut data = vec![];
    let mut min_start_timestamp = u32::MAX;
    let mut max_end_timestamp = 0;
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
        let account_len = account_id_str.len() as u8;
        let mut account_id = [0u8; 64];
        account_id[..account_id_str.len()].copy_from_slice(account_id_str.as_bytes());
        let balance = balance
            .checked_mul(balance_multiplier)
            .expect("Balance multiplication overflow");
        total_accounts += 1;
        total_balance = total_balance
            .checked_add(balance)
            .expect("Total balance overflow");
        let account = FixedSizeAccount {
            account_len,
            account_id,
            start_timestamp,
            cliff_timestamp,
            end_timestamp,
            balance,
        };
        data.extend(account.try_to_vec().unwrap());
    }
    println!("Total number of accounts {}\nTotal balance: {}\nTotal multiplied balance: {}\nMinimum start timestamp: {}\nMaximum end timestamp: {}",
        total_accounts,
        total_balance / balance_multiplier,
        total_balance,
        min_start_timestamp,
        max_end_timestamp,
    );
    let mut file = File::create(output_file_path).expect("Failed to create the output file");
    file.write_all(&data).expect("Failed to write data");
}
