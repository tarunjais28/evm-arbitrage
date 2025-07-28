use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, Write},
    sync::{Arc, Mutex},
};

use alloy::{
    primitives::{Address, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
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
    CurvePool1,
    "../../resources/contracts/curve_pool_1.json"
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
    let unique_tokens = Arc::new(Mutex::new(HashSet::new()));

    let tasks = pools.into_iter().map(|pool| {
        let provider = provider.clone();
        let provider_1 = provider.clone();
        let unique_tokens = Arc::clone(&unique_tokens);

        tokio::spawn(async move {
            let contract = CurvePool::new(pool, provider);
            let contract_1 = CurvePool1::new(pool, provider_1);

            let a = match contract.A().call().await {
                Ok(val) => val,
                Err(err) => {
                    log::error!("pool: {pool}, A(): {err}");
                    U256::default()
                }
            };

            let fee = match contract.fee().call().await {
                Ok(val) => val,
                Err(err) => {
                    log::error!("pool: {pool}, fee(): {err}");
                    U256::default()
                }
            };

            let mut count = U256::ZERO;
            let mut count_1 = 0;
            let mut tokens = Vec::new();
            loop {
                if let Ok(token) = contract.coins(count).call().await {
                    {
                        let mut set = unique_tokens.lock().unwrap();
                        set.insert(token);
                    }
                    tokens.push(token);
                    count += U256::ONE;
                } else if let Ok(token) = contract_1.coins(count_1).call().await {
                    {
                        let mut set = unique_tokens.lock().unwrap();
                        set.insert(token);
                    }
                    tokens.push(token);
                    count_1 += 1;
                } else {
                    break;
                }
            }

            Pools {
                balances: vec![U256::ZERO; tokens.len()],
                tokens,
                fee,
                a,
                address: pool,
            }
        })
    });

    let mut curve_pools = Vec::new();
    let mut handles: FuturesUnordered<_> = tasks.collect();

    while let Some(result) = handles.next().await {
        if let Ok(pool) = result {
            curve_pools.push(pool);
        }
    }

    let tokens = Arc::try_unwrap(unique_tokens)
        .unwrap_or_default()
        .into_inner()
        .unwrap_or_default();

    (curve_pools, tokens)
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

    let (curve_pools, tokens) = debug_time!("get_pool_data()", {
        get_pool_data(&provider, pools).await
    });

    let mut file = File::create("resources/curve_tokens_to_pool.json").unwrap();
    file.write_all(
        serde_json::to_string_pretty(&curve_pools)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();

    // let mut file = File::create("resources/curve_tokens.json").unwrap();
    // file.write_all(serde_json::to_string_pretty(&tokens).unwrap().as_bytes())
    //     .unwrap();
}
