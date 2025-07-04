use crate::{
    dijkstra::*,
    enums::*,
    fetch::*,
    parser::*,
    pools::*,
    scanner::*,
    slippage::*,
    structs::*,
    IUniswapV2Pool::{Burn, Mint, Swap, Sync},
};
use alloy::{
    primitives::{Address, TxHash, Uint, U256},
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

type U112 = Uint<112, 2>;

mod dijkstra;
mod enums;
mod fetch;
mod parser;
#[macro_use]
mod logger;
mod pools;
mod scanner;
mod slippage;
mod structs;

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV2Pool,
    "../../resources/uniswapv2_pool_abi.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV2Pair,
    "../../resources/uniswapv2_pair.json"
);

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
            File::open("resources/uniswapv2_tokens_to_pool.json")?
        });
        let reader = debug_time!("reader()", { BufReader::new(file) });
        let pools: Vec<Pools> = debug_time!("pools_serialize()", { from_reader(reader)? });
        let pool_data: PoolData = debug_time!("update_reserves", {
            update_reserves(provider.clone(), pools).await?
        });

        // Scanning the ethereum blockchain for events
        debug_time!("Calling scanner()", {
            scan(provider.clone(), env_parser.pools_addrs, pool_data).await?
        });
    });

    Ok(())
}
