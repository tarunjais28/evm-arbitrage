use crate::{
    dijkstra::*,
    enums::*,
    fetch::*,
    pools::*,
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
mod pools;
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

    // // Scanning the ethereum blockchain for events
    // scan(provider, env_parser.pools_addrs).await?;

    let file = File::open("resources/tokens_to_pool.json")?;
    let reader = BufReader::new(file);
    let pools: Vec<Pools> = from_reader(reader)?;

    let mut pool_data: HashMap<TokenPair, TokenData> = HashMap::with_capacity(pools.len());
    // let mut graph: SwapGraph = HashMap::with_capacity(pools.len());

    pools.iter().for_each(|pool| {
        let (pair, data) = pool.to_key_value();
        pool_data.insert(pair, data);
    });

    for (_pair, data) in pool_data.iter_mut() {
        let reserves = get_reserves(provider.clone(), data.address).await?;
        data.update_reserves(reserves);
        data.calc_slippage();
    }

    println!("{:#?}", pool_data);

    Ok(())
}
