use std::{
    fs::File,
    io::{BufReader, Write},
    sync::Arc,
};

use alloy::{
    primitives::{Address, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use colored::Colorize;
use futures::{stream::FuturesUnordered, StreamExt};
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
    balances: Vec<U256>,
    fee: U256,
    a: U256,
    address: Address,
}

pub async fn get_balances<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pools: &mut Vec<Pools>,
) {
    let provider = Arc::new(provider.clone());

    let mut tasks = FuturesUnordered::new();

    for pool in pools.iter_mut() {
        let provider = Arc::clone(&provider);
        let pool_ptr: *mut Pools = pool;

        tasks.push(async move {
            let pool = unsafe { &mut *pool_ptr }; // Safe here because each task has a unique pool
            let contract = CurvePool::new(pool.address, provider.as_ref().clone());
            let mut multicall = provider.multicall().dynamic();

            for i in 0..pool.tokens.len() {
                multicall = multicall.add_dynamic(contract.balances(U256::from(i)));
            }

            if let Ok(bals) = multicall.aggregate3().await {
                for (i, bals_res) in bals.iter().enumerate() {
                    if let Ok(bal) = bals_res {
                        pool.balances[i] = *bal;
                    } else {
                        eprintln!("{}", format!("pool: {}, i: {i}", pool.address).red());
                    }
                }
            } else {
                eprintln!("{}", format!("pool: {}", pool.address).red());
            }
        });
    }

    while tasks.next().await.is_some() {}
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

    let file = File::open("resources/curve_tokens_to_pool.json").unwrap();
    let reader = BufReader::new(file);
    let mut pools: Vec<Pools> = from_reader(reader).unwrap();

    debug_time!("get_balances()", {
        get_balances(&provider, &mut pools).await
    });

    let mut file = File::create("resources/curve_tokens_to_pool.json").unwrap();
    file.write_all(serde_json::to_string_pretty(&pools).unwrap().as_bytes())
        .unwrap();
}
