use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{
    fs::File,
    io::{BufReader, Write},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolAddress {
    pub v2: Vec<Address>,
    pub v3: Vec<Address>,
    pub curve: Vec<Address>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut pool: Vec<Vec<Address>> = vec![vec![], vec![], vec![]];
    for (i, fp) in [
        "resources/pools_v2.json",
        "resources/pools_v3.json",
        "resources/curve_pools.json",
    ]
    .iter()
    .enumerate()
    {
        let file = File::open(fp)?;
        let reader = BufReader::new(file);

        pool[i] = from_reader(reader)?;
    }

    let pools = PoolAddress {
        v2: pool[0].clone(),
        v3: pool[1].clone(),
        curve: pool[2].clone(),
    };

    let mut file = File::create("resources/pools_combined.json")?;
    file.write_all(serde_json::to_string_pretty(&pools)?.as_bytes())?;

    Ok(())
}
