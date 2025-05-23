use reqwest::header::HeaderMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time;
use xerror::XwError;

const TIMEOUT_SEC: u64 = 10;
#[derive(Serialize, Deserialize, Debug)]
pub struct WalletBlock {
    #[serde(default, rename = "time")]
    time: String,

    #[serde(default, rename = "timestamp")]
    timestamp: String,

    #[serde(default, rename = "flags")]
    flags: Option<serde_json::Value>,

    #[serde(default, rename = "state")]
    state: String,

    #[serde(default, rename = "file_pos")]
    file_pos: String,

    #[serde(default, rename = "file")]
    file: String,

    #[serde(default, rename = "hash")]
    hash: Option<serde_json::Value>,

    #[serde(default, rename = "remark")]
    remark: Option<serde_json::Value>,

    #[serde(default, rename = "difficulty")]
    difficulty: Option<serde_json::Value>,

    #[serde(default, rename = "balance_address")]
    pub balance_address: String,

    #[serde(default, rename = "balance")]
    pub balance: String,

    #[serde(default, rename = "ui_notifications")]
    ui_notifications: Vec<Option<serde_json::Value>>,

    #[serde(default, rename = "block_as_transaction")]
    block_as_transaction: Vec<BlockAsTransaction>,

    #[serde(default, rename = "block_as_address")]
    pub block_as_address: Vec<BlockAsAddress>,

    #[serde(default, rename = "balances_last_week")]
    balances_last_week: Option<serde_json::Value>,

    #[serde(default, rename = "earnings_last_week")]
    earnings_last_week: Option<serde_json::Value>,

    #[serde(default, rename = "spendings_last_week")]
    spendings_last_week: Option<serde_json::Value>,

    #[serde(default, rename = "balance_change_last_24_hours")]
    balance_change_last_24_hours: String,

    #[serde(default, rename = "earnings_change_last_24_hours")]
    earnings_change_last_24_hours: String,

    #[serde(default, rename = "spendings_change_last_24_hours")]
    spendings_change_last_24_hours: String,

    #[serde(default, rename = "total_earnings")]
    total_earnings: String,

    #[serde(default, rename = "total_spendings")]
    total_spendings: String,

    #[serde(default, rename = "page_earnings_sum")]
    page_earnings_sum: String,

    #[serde(default, rename = "page_spendings_sum")]
    page_spendings_sum: String,

    #[serde(default, rename = "filtered_earnings_sum")]
    filtered_earnings_sum: String,

    #[serde(default, rename = "filtered_spendings_sum")]
    filtered_spendings_sum: String,

    #[serde(default, rename = "kind")]
    kind: String,

    #[serde(default, rename = "transactions_pagination")]
    transactions_pagination: SPagination,

    #[serde(default, rename = "addresses_pagination")]
    addresses_pagination: SPagination,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranxBlock {
    #[serde(default)]
    time: String,
    #[serde(default)]
    timestamp: String,
    #[serde(default)]
    flags: String,
    #[serde(default)]
    pub state: String,
    #[serde(default, rename = "file_pos")]
    file_pos: String,
    #[serde(default)]
    file: String,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    remark: Option<String>,
    #[serde(default)]
    difficulty: String,
    #[serde(default, rename = "balance_address")]
    pub balance_address: String,
    balance: String,
    #[serde(default, rename = "ui_notifications")]
    ui_notifications: Vec<Option<serde_json::Value>>,
    #[serde(default, rename = "block_as_transaction")]
    pub block_as_transaction: Vec<BlockAsTransaction>,
    #[serde(default, rename = "block_as_address")]
    block_as_address: Vec<BlockAsAddress>,
    #[serde(default, rename = "total_fee")]
    pub total_fee: String,
    #[serde(default, rename = "total_inputs")]
    total_inputs: String,
    #[serde(default, rename = "total_outputs")]
    total_outputs: String,
    #[serde(default, rename = "page_fee_sum")]
    page_fee_sum: String,
    #[serde(default, rename = "page_inputs_sum")]
    page_inputs_sum: String,
    #[serde(default, rename = "page_outputs_sum")]
    page_outputs_sum: String,
    #[serde(default, rename = "filtered_fee_sum")]
    filtered_fee_sum: String,
    #[serde(default, rename = "filtered_inputs_sum")]
    filtered_inputs_sum: String,
    #[serde(default, rename = "filtered_outputs_sum")]
    filtered_outputs_sum: String,
    #[serde(default)]
    kind: String,
    #[serde(default, rename = "transactions_pagination")]
    transactions_pagination: SPagination,
    #[serde(default, rename = "addresses_pagination")]
    addresses_pagination: SPagination,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct SPagination {
    #[serde(rename = "current_page")]
    current_page: i64,

    #[serde(rename = "last_page")]
    last_page: i64,

    #[serde(rename = "total")]
    total: i64,

    #[serde(rename = "per_page")]
    per_page: i64,

    #[serde(rename = "links")]
    links: Links,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Links {
    #[serde(rename = "prev")]
    prev: Option<serde_json::Value>,

    #[serde(rename = "next")]
    next: Option<String>,

    #[serde(rename = "first")]
    first: String,

    #[serde(rename = "last")]
    last: String,
}

// #[derive(Serialize, Deserialize, Default, Debug)]
// pub struct SLastWeek {
//     #[serde(rename = "2024-05-29")]
//     the_20240529: String,

//     #[serde(rename = "2024-05-30")]
//     the_20240530: String,

//     #[serde(rename = "2024-05-31")]
//     the_20240531: String,

//     #[serde(rename = "2024-06-01")]
//     the_20240601: String,

//     #[serde(rename = "2024-06-02")]
//     the_20240602: String,

//     #[serde(rename = "2024-06-03")]
//     the_20240603: String,

//     #[serde(rename = "2024-06-04")]
//     the_20240604: String,
// }

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockAsAddress {
    #[serde(rename = "direction")]
    pub direction: Direction,

    #[serde(rename = "address")]
    pub address: String,

    #[serde(rename = "amount")]
    pub amount: String,

    #[serde(rename = "time")]
    pub time: String,

    #[serde(rename = "remark")]
    pub remark: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockAsTransaction {
    pub direction: String,
    pub address: String,
    pub amount: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Direction {
    #[serde(rename = "input")]
    Input,

    #[serde(rename = "output")]
    Output,

    #[serde(rename = "snapshot")]
    Snapshot,
}

pub async fn get_history<T: DeserializeOwned>(
    url: &str,
    address: &str,
    page_no: u32,
) -> Result<T, XwError> {
    let client = reqwest::Client::new();
    let mut _headers = HeaderMap::new();
    _headers.insert(
        "Accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
            .parse()
            .unwrap(),
    );
    _headers.insert("Accept-Encoding", "gzip, deflate".parse().unwrap());
    _headers.insert(
        "Accept-Language",
        "zh-cn,zh;q=0.8,en-us;q=0.5,en;q=0.3".parse().unwrap(),
    );
    _headers.insert("Connection", "keep-alive".parse().unwrap());
    _headers.insert("User-Agent", "Apache-HttpClient/4.3.1".parse().unwrap());

    let uri: String = format!("{}/{}", url, address);

    let items: T = client
        .get(uri)
        .query(&[("addresses_per_page", 50), ("addresses_page", page_no)])
        .headers(_headers)
        .timeout(time::Duration::from_secs(TIMEOUT_SEC))
        .send()
        .await?
        .json()
        .await?;

    Ok(items)
}

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    async fn test_get_history() {
        let result = get_history::<WalletBlock>(
            "https://explorer.xdag.io/api/block",
            "4RfUFL7XwLi3gaprob6uPUQ41dHbnjcD6",
            2,
        )
        .await;

        if let Err(e) = result {
            println!("Error fetching address history: {:?}", e)
        } else {
            let res = result.unwrap();
            println!("{:?}", res);
            for i in res.block_as_address.into_iter() {
                println!("{:?}", i);
            }
        }
    }

    #[tokio::test]
    async fn test_get_tranx() {
        let result = get_history::<TranxBlock>(
            "https://testexplorer.xdag.io/api/block",
            "uMz89D4AG+0yHDDHXXuIwo4LZGN4Bq1U",
            1,
        )
        .await;
        if let Err(e) = result {
            println!("Error fetching transaction history:{:?}", e);
        } else {
            let res = result.unwrap();
            println!("{:?}", res);
            for i in res.block_as_transaction.into_iter() {
                println!("{:?}", i);
            }
        }
    }
}
