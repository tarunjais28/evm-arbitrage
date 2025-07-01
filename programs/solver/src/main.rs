use crate::{
    dijkstra::*,
    enums::*,
    fetch::*,
    pools::*,
    scanner::*,
    slippage::*,
    structs::*,
    IUniswapV2Pool::{Burn, Mint, Swap, Sync},
};
use alloy::{
    primitives::{address, Address, TxHash, Uint, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    rpc::types::{Filter, Log},
    sol,
};
use colored::Colorize;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    fmt::Display,
    fs::File,
    io::BufReader,
};
use utils::{CustomError, EnvParser};
type U112 = Uint<112, 2>;

mod dijkstra;
mod enums;
mod fetch;
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
    // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    env_logger::init();

    log::info!("Logger initialized");

    // Load environment variables from .env file
    let env_parser = debug_time!("env_parser", { EnvParser::new()? });

    // Set up the WS transport and connect.
    let ws = WsConnect::new(env_parser.ws_address);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    // // Scanning the ethereum blockchain for events
    // debug_time!("Calling scanner()", {
    //     scan(provider.clone(), env_parser.pools_addrs).await?
    // });

    let file = File::open("resources/tokens_to_pool.json")?;
    let reader = BufReader::new(file);
    let pools: Vec<Pools> = from_reader(reader)?;
    let graph = debug_time!("slippage::calc_slippage", {
        calc_slippage(provider, pools).await?
    });

    let best_path = debug_time!("best_path()", {
        best_path(
            &graph,
            &address!("0x614f611300d8fb0108fa2a860dbca1ff8fc62624"),
            &address!("0xdac17f958d2ee523a2206206994597c13d831ec7"),
        )
    });
    println!("{:#?}", best_path);

    Ok(())
}
