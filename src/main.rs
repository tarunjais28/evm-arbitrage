use crate::{errors::*, parser::*, process::*, structs::*, util::*};
use colored::Colorize;
use dotenv::dotenv;
use futures::future::join_all;
use futures::StreamExt;
use hex::FromHexError;
use num_bigint::ParseBigIntError;
use std::{
    env::{self, VarError},
    fmt::Display,
    sync::Arc,
};
use thiserror::Error;
use web3::{
    ethabi::{Address, Event, Log},
    transports::WebSocket,
    types::{H160, H256},
    Web3,
};

mod errors;
mod parser;
mod process;
mod structs;
mod util;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();

    let env_parser = EnvParser::new()?;

    // Shared WebSocket connection
    let web3 = Arc::new(web3::Web3::new(
        web3::transports::ws::WebSocket::new(&env_parser.ws_address).await?,
    ));

    // Shared block stream
    let mut block_stream = web3.eth_subscribe().subscribe_new_heads().await?;

    while let Some(Ok(block)) = block_stream.next().await {
        let block_number = block.number.ok_or(CustomError::NotFound("block number"))?;
        let block_hash = block.hash.ok_or(CustomError::NotFound("block hash"))?;

        println!("{}", format!("current block: {}", block_number).blue());

        // Spawn a task for each contract address
        let mut tasks = vec![];
        for address in &env_parser.contract_addresses {
            let web3 = web3.clone();
            let address = *address;

            let task = tokio::spawn(async move {
                let contract = web3::contract::Contract::from_json(
                    web3.eth(),
                    address,
                    include_bytes!("contracts/uniswap_pool_abi.json"),
                )?;

                let swap_event = contract
                    .abi()
                    .events_by_name("Swap")?
                    .first()
                    .ok_or(CustomError::EventNameError("Swap"))?;
                let swap_event_signature = swap_event.signature();

                let sync_event = contract
                    .abi()
                    .events_by_name("Sync")?
                    .first()
                    .ok_or(CustomError::EventNameError("Sync"))?;
                let sync_event_signature = sync_event.signature();

                show(
                    web3.clone(),
                    address,
                    &[swap_event.clone(), sync_event.clone()],
                    vec![swap_event_signature, sync_event_signature],
                    block_hash,
                )
                .await
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        let results = join_all(tasks).await;
        for res in results {
            if let Err(e) = res {
                eprintln!("{}", format!("Error in contract task: {:?}", e).red());
            }
        }
    }

    Ok(())
}
