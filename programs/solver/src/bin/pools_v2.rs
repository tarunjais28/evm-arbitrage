use alloy::{
    primitives::{address, Address},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use futures::future::join_all;
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
    IUniswapV2Factory,
    "../../resources/contracts/uniswapv2_factory.json"
);

pub async fn get_pair_address<'a>(
    provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    factory_address: Address,
    token_a: Address,
    token_b: Address,
) -> Result<Option<Pools>, CustomError<'a>> {
    let contract = IUniswapV2Factory::new(factory_address, provider);
    let pair: Address = contract.getPair(token_a, token_b).call().await?;

    if pair.is_zero() {
        Ok(None)
    } else {
        Ok(Some(Pools::new(token_a, token_b, pair)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pools {
    token_a: Address,
    token_b: Address,
    fee: u16,
    address: Address,
}

impl Pools {
    fn new(token_a: Address, token_b: Address, address: Address) -> Self {
        Self {
            token_a,
            token_b,
            fee: 0,
            address,
        }
    }
}

enum Exchanges {
    Sushi,
    Uniswap,
}

async fn get_addresses_v2<'a>(
    provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    tokens: Vec<Address>,
    pools: &mut Vec<Pools>,
    exchanges: Exchanges,
) -> Result<(), CustomError<'a>> {
    let n = tokens.len();

    use Exchanges::*;
    let factory = match exchanges {
        Uniswap => address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"),
        Sushi => address!("0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac"),
    };

    let mut handles = Vec::with_capacity((n * (n - 1)) / 2);
    for i in 0..n - 1 {
        for j in (i + 1)..n {
            let provider_clone = provider.clone();
            let factory_clone = factory;
            let token_a = tokens[i];
            let token_b = tokens[j];

            handles.push(tokio::spawn(async move {
                get_pair_address(provider_clone, factory_clone, token_a, token_b).await
            }));
        }
    }

    let results = join_all(handles).await;
    for result in results {
        if let Ok(Ok(Some(pool))) = result {
            pools.push(pool);
        }
    }

    Ok(())
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

    let file = File::open("resources/tokens.json")?;
    let reader = BufReader::new(file);

    // Parse and decode addresses
    let tokens: Vec<Address> = from_reader(reader)?;
    let mut pools: Vec<Pools> = Vec::with_capacity(tokens.len() * 2);
    get_addresses_v2(
        provider.clone(),
        tokens.clone(),
        &mut pools,
        Exchanges::Uniswap,
    )
    .await?;
    let uniswap_pools = pools.len();
    log::info!("UniswapV2 Pools: {uniswap_pools}");

    get_addresses_v2(provider, tokens, &mut pools, Exchanges::Sushi).await?;
    let sushiswap_pools = pools.len() - uniswap_pools;
    log::info!("SushiswapV2 Pools: {sushiswap_pools}");

    let mut pool_addresses = Vec::with_capacity(pools.len());
    pools.iter().for_each(|p| pool_addresses.push(p.address));

    let mut file = File::create("resources/uniswapv2_tokens_to_pool.json")?;
    file.write_all(serde_json::to_string_pretty(&pools)?.as_bytes())?;

    let mut file = File::create("resources/pools_v2.json")?;
    file.write_all(serde_json::to_string_pretty(&pool_addresses)?.as_bytes())?;

    Ok(())
}
