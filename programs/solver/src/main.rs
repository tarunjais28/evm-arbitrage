use crate::{
    contracts::*, dijkstra::*, enums::*, fetch::*, helper::*, parser::*, pools::*, scanner::*,
    slippage::*, structs::*,
};
use alloy::{
    primitives::{Address, TxHash, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    rpc::types::{Filter, Log},
    sol,
};
use colored::Colorize;
use dotenv::dotenv;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    env,
    fmt::Display,
    fs::File,
    io::BufReader,
    sync::Arc,
};

use tokio::sync::{mpsc, Mutex};
use utils::CustomError;

mod dijkstra;
mod enums;
mod fetch;
mod parser;
#[macro_use]
mod logger;
mod contracts;
mod helper;
mod pools;
mod scanner;
mod slippage;
mod structs;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize the logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Logger initialized");

    info_time!("main()", {
        // Load environment variables from .env file
        let env_parser = info_time!("env_parser", { EnvParser::new()? });

        // Set up the WS transport and connect.
        let ws = WsConnect::new(env_parser.ws_address);
        let provider = ProviderBuilder::new().connect_ws(ws).await?;

        let file = debug_time!("file_open()", {
            File::open("resources/tokens_to_pool.json")?
        });
        let reader = debug_time!("reader()", { BufReader::new(file) });
        let pools: Vec<Pools> = debug_time!("pools_serialize()", { from_reader(reader)? });
        let pool_data: PoolData = debug_time!("update_reserves", {
            update_reserves(provider.clone(), pools, &env_parser.pool_address).await?
        });

        // Scanning the ethereum blockchain for events
        debug_time!("Calling scanner()", {
            scan(
                provider.clone(),
                env_parser.pool_address.single(),
                pool_data,
            )
            .await?
        });
    });

    Ok(())
}
