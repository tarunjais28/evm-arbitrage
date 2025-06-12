use crate::{errors::*, helper::*, parser::*, structs::*};
use dotenv::dotenv;
use futures::StreamExt;
use hex::FromHexError;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};
use std::env::{self, VarError};
use std::{collections::HashMap, str::FromStr};
use thiserror::Error;
use web3::{
    ethabi::{Address, Event, Int, Log},
    transports::WebSocket,
    types::{H160, H256, U64},
    Web3,
};

mod errors;
mod helper;
mod parser;
mod structs;
#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();

    let env_parser = EnvParser::new()?;

    let web3 = web3::Web3::new(web3::transports::ws::WebSocket::new(&env_parser.ws_address).await?);
    let contract_address =
        web3::types::H160::from_slice(&hex::decode(env_parser.contract_address)?[..]);
    let contract = web3::contract::Contract::from_json(
        web3.eth(),
        contract_address,
        include_bytes!("contracts/uniswap_pool_abi.json"),
    )?;
    let swap_event = contract
        .abi()
        .events_by_name("Swap")?
        .first()
        .ok_or(CustomError::EventNameError("Swap"))?;
    let swap_event_signature = swap_event.signature();

    let mut block_stream = web3.eth_subscribe().subscribe_new_heads().await?;
    let mut data: HashMap<U64, BlockData> = HashMap::new();

    while let Some(Ok(block)) = block_stream.next().await {
        let block_number = block.number.ok_or(CustomError::NotFound("block number"))?;
        let block_hash = block.hash.ok_or(CustomError::NotFound("block hash"))?;

        println!("current block: {}", block_number);

        read_and_add_logs(
            web3.clone(),
            block_hash,
            contract_address,
            swap_event_signature,
            &mut data,
            block_number,
        )
        .await?;

        // Showing blocks N - 5 to protect against reorganisation
        show(
            web3.clone(),
            contract_address,
            swap_event,
            swap_event_signature,
            &mut data,
            block_number - 5,
        )
        .await?;

        check_for_reorganization(
            &mut data,
            block_number,
            web3.clone(),
            contract_address,
            swap_event_signature,
        )
        .await?;
    }

    Ok(())
}
