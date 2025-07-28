use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{
    fs::File,
    io::{BufReader, Write},
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, PartialOrd)]
pub struct TokenMetadata {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let mut token_metadata: Vec<TokenMetadata> = Vec::default();
    for fp in [
        "resources/token_metadata.json",
        "resources/curve_token_metadata.json",
    ]
    {
        let file = File::open(fp)?;
        let reader = BufReader::new(file);

        let mut tm: Vec<TokenMetadata> = from_reader(reader)?;
        token_metadata.append(&mut tm);
    }

    println!("Before: {}", token_metadata.len());
    token_metadata.sort_by_key(|t| t.address);
    token_metadata.dedup();
    println!("After: {}", token_metadata.len());

    let mut file = File::create("resources/token_metadata_combined.json")?;
    file.write_all(serde_json::to_string_pretty(&token_metadata)?.as_bytes())?;

    Ok(())
}
