use alloy::{
    primitives::Address,
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{
    fs::File,
    io::{BufReader, Write},
};
use utils::{CustomError, EnvParser};

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV3Pool,
    "../../resources/contracts/uniswapv3_pool_abi.json"
);

async fn get_serialised_pool_data<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pools: &[Address],
) -> Result<Vec<Pools>, CustomError<'a>> {
    let mut futures = Vec::with_capacity(pools.len());
    for &address in pools {
        let provider_clone = provider.clone();
        let fut = async move {
            let contract = IUniswapV3Pool::new(address, provider_clone);
            let token0 = contract.token0().call().await?;
            let token1 = contract.token1().call().await?;
            let fee = contract.fee().call().await?;
            Ok(Pools {
                token0,
                token1,
                fee: fee.to_string().parse::<u16>().unwrap(),
                address,
            })
        };
        futures.push(fut);
    }

    let results: Vec<Result<Pools, CustomError<'a>>> = futures::stream::iter(futures)
        .buffer_unordered(10)
        .collect::<Vec<_>>()
        .await;

    let mut pool_data = Vec::with_capacity(pools.len());
    results.iter().for_each(|res| {
        if let Ok(pool) = res {
            pool_data.push(pool.clone());
        }
    });

    Ok(pool_data)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pools {
    token0: Address,
    token1: Address,
    fee: u16,
    address: Address,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize the logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Logger initialized");

    // Load environment variables from .env file
    let env_parser = EnvParser::new()?;

    // Set up the WS transport and connect.
    let ws = WsConnect::new(env_parser.ws_address);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    let file = File::open("resources/pools_v3.json")?;
    let reader = BufReader::new(file);

    // Parse and decode addresses
    let tokens: Vec<Address> = from_reader(reader)?;
    let pool_data = get_serialised_pool_data(&provider, &tokens).await?;

    let mut file = File::create("resources/serialised_v3_pools.json")?;
    file.write_all(serde_json::to_string_pretty(&pool_data)?.as_bytes())?;

    log::info!("{} pools serialised!", pool_data.len());

    Ok(())
}
