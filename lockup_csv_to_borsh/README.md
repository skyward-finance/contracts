# Tool for lockups

Converts a given csv file into a lockup binary input for efficient lockup deployments.

## Run

The tool takes up to 3 arguments:
- Input csv filename
- Output filename
- Optional balance multiplier. It's convenient to put balances in the CSV file to something that is human-readable.By default `1`.

```bash
cargo run -- example.csv example_out.borsh 1000000000000000
```

## CSV format

The CSV contains 5 fields:
- Account ID for the lockup. Has to be a valid account ID.
- The start date in the following format `YYYY-MM-DD`.
- Optional cliff date in the following format `YYYY-MM-DD`.
- The end date in the following format `YYYY-MM-DD`.
- The integer balance. It'll be multiplied by the balance multiplier argument. 

E.g. `example.csv`
```csv
account_id,start_date,cliff_date,end_date,balance
alice.near,2021-06-01,,2021-07-01,10
bob.near,2021-06-01,2021-07-01,2021-10-01,10000
```

Assuming:
- The given balance multiplier is `15` decimals (`1000000000000000`).
- And the FT has `18` decimals or (`1000000000000000000`).

The lockup setup has 2 accounts:
- `alice.near` has linear vesting of `0.01` tokens for 1 month (starting from `June 1, 2021` to `July 1, 2021` in UTC time).
- `bob.near` has linear vesting of `10` tokens for 4 months and 1 month cliff (starting from `June 1, 2021`, with a cliff at `July 1, 2021` and ends in `October 1, 2021` in UTC time).

The output info:
```console
Total number of accounts 2
Total balance: 10010
Total multiplied balance: 10010000000000000000
Minimum start timestamp: 1622505600
Maximum end timestamp: 1633046400
```

And produces the binary `example_out.borsh`
