# Fungible Token Lockup contract

Make sure you update `data/accounts.borsh` before building the contract.

## 查询接口
```rust
pub struct AccountOutput {
    // 开始时间s，中间时间c，结束时间e，时间戳(秒)
    // s到c之间为封闭期，c到e之间为线性释放期
    pub start_timestamp: TimestampSec,
    pub cliff_timestamp: TimestampSec,
    pub end_timestamp: TimestampSec,
    // 总金额，固定不动
    pub balance: WrappedBalance,
    // 已提取金额，每次claim之后增加
    pub claimed_balance: WrappedBalance,
}

// 指定账户查询锁仓情况
pub fn get_account(&self, account_id: ValidAccountId) -> Option<AccountOutput>;


pub struct Stats {
    // token地址
    pub token_account_id: TokenAccountId,
    // 财库地址
    pub skyward_account_id: AccountId,
    // 超期期限，这之后所有未动过的账户中的token会被捐献给财库
    pub claim_expiration_timestamp: TimestampSec,
    // 初始总锁仓量
    pub total_balance: WrappedBalance,
    // 所有未碰过的账户中的金额汇总
    pub untouched_balance: WrappedBalance,
    // 总的已提取量
    pub total_claimed: WrappedBalance,
}
// 查询合约状态
pub fn get_stats(&self) -> Stats；
```

## 操作接口
```rust
// 提取签名者的所有已释放未提取的token
pub fn claim(&mut self) -> PromiseOrValue<bool>;

// 超期后，将所有未动过的账户中的token和本合约剩余的near捐献给财库
pub fn donate_to_treasury(&mut self) -> Promise;
```

