use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, Write},
};

use alloy::{
    primitives::{Address, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use uniswap_sdk_core::prelude::*;
use utils::{debug_time, EnvParser};

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    CurvePool,
    "../../resources/contracts/curve_pool.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    ERC20,
    "../../resources/contracts/erc20_abi.json"
);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pools {
    tokens: Vec<Address>,
    fee: U256,
    a: U256,
    address: Address,
}

pub async fn get_pool_data<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pools: Vec<Address>,
) -> (Vec<Pools>, HashSet<Address>) {
    let mut curve_pools = Vec::with_capacity(pools.len());
    let mut unique_tokens = HashSet::new();
    for pool in pools {
        log::info!("Processing {pool}...........");
        let contract = CurvePool::new(pool, provider);
        let a = match contract.A().call().await {
            Ok(a) => a,
            Err(err) => {
                log::error!("pool: {pool}, A(): {err}");
                U256::default()
            }
        };

        let fee = contract.fee().call().await.unwrap_or_default();

        let mut count = U256::ZERO;
        let mut tokens: Vec<Address> = Vec::new();
        loop {
            if let Ok(addr) = contract.coins(count).call().await {
                unique_tokens.insert(addr);
                tokens.push(addr);
                count += U256::ONE;
            } else {
                break;
            }
        }

        curve_pools.push(Pools {
            tokens,
            fee,
            a,
            address: pool,
        });
    }

    (curve_pools, unique_tokens)
}

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Logger initialized");

    // Load environment variables from .env file
    let env_parser = EnvParser::new().unwrap();

    // Set up the WS transport and connect.
    let ws = WsConnect::new(env_parser.ws_address);
    let provider = ProviderBuilder::new().connect_ws(ws).await.unwrap();

    let file = File::open("resources/curve_pools.json").unwrap();
    let reader = BufReader::new(file);
    let pools: Vec<Address> = from_reader(reader).unwrap();

    let (curve_pools, tokens) =
        debug_time!("get_pool_data()", { get_pool_data(&provider, pools).await });

    let mut file = File::create("resources/curve_tokens_to_pool.json").unwrap();
    file.write_all(
        serde_json::to_string_pretty(&curve_pools)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();

    let mut file = File::create("resources/curve_tokens.json").unwrap();
    file.write_all(serde_json::to_string_pretty(&tokens).unwrap().as_bytes())
        .unwrap();
}
