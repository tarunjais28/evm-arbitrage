use crate::{
    dijkstra::*,
    enums::*,
    fetch::*,
    scanner::*,
    structs::*,
    IUniswapV2Pool::{Burn, Mint, Swap, Sync},
};
use alloy::{
    primitives::{Address, TxHash, Uint, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    rpc::types::Filter,
    rpc::types::Log,
    sol,
};
use colored::Colorize;
use futures_util::stream::StreamExt;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    fmt::Display,
};
use utils::{CustomError, EnvParser};
type U112 = Uint<112, 2>;

mod dijkstra;
mod enums;
mod fetch;
mod helper;
mod scanner;
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
    // Load environment variables from .env file
    let env_parser = EnvParser::new()?;

    // Set up the WS transport and connect.
    let ws = WsConnect::new(env_parser.ws_address);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    // Scanning the ethereum blockchain for events
    scan(provider, env_parser.pools_addrs).await?;

    Ok(())
}
