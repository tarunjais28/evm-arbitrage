use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{collections::HashSet, fs::File, io::BufReader};
use uniswap_sdk_core::prelude::*;

#[derive(Debug, Deserialize, Serialize)]
struct Pools {
    meta: Vec<Address>,
    unspecified: Vec<Address>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PoolData {
    pool: Address,
    dy: i128,
}

#[derive(Debug, Deserialize, Serialize)]
struct Output {
    dx: u128,
    data: Vec<PoolData>,
}

#[tokio::main]
async fn main() {
    let file = File::open("resources/1.json").unwrap();
    let reader = BufReader::new(file);

    // Parse and decode addresses
    let v1: Pools = from_reader(reader).unwrap();

    let file = File::open("resources/2.json").unwrap();
    let reader = BufReader::new(file);

    // Parse and decode addresses
    let v2: Pools = from_reader(reader).unwrap();

    let s1: HashSet<_> = v1.meta.iter().cloned().collect();
    let s2: HashSet<_> = v2.meta.iter().cloned().collect();

    let non_common: Vec<_> = s1.symmetric_difference(&s2).cloned().collect();
    println!("{:#?}", non_common);
}
