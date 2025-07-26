use alloy::{
    primitives::{address, aliases::U24, Address},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use futures::{future::join_all, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{
    fs::File,
    io::{BufReader, Write},
    sync::Arc,
};
use tokio::sync::Mutex;
use utils::{CustomError, EnvParser};

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV3Factory,
    "../../resources/contracts/uniswapv3_factory.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    ERC20,
    "../../resources/contracts/erc20_abi.json"
);

// sushi = https://etherscan.io/address/0xc0aee478e3658e2610c5f7a4a2e1777ce9e4f2ac#readContract

#[derive(Debug, Clone, Copy)]
struct TokenData {
    address: Address,
    decimals: u8,
}

async fn get_decimal<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    tokens: &[Address],
) -> Result<Vec<TokenData>, CustomError<'a>> {
    let mut futures = Vec::with_capacity(tokens.len());
    for &address in tokens {
        let provider_clone = provider.clone();
        let fut = async move {
            let contract = ERC20::new(address, provider_clone);
            let decimals = contract.decimals().call().await?;
            Ok(TokenData { address, decimals })
        };
        futures.push(fut);
    }

    let results: Vec<Result<TokenData, CustomError<'a>>> = futures::stream::iter(futures)
        .buffer_unordered(10)
        .collect::<Vec<_>>()
        .await;

    let mut token_data = Vec::with_capacity(tokens.len());
    results.iter().for_each(|res| {
        if let Ok(token) = res {
            token_data.push(*token);
        }
    });

    Ok(token_data)
}

async fn get_pair_address_v3<'a>(
    provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pools: Arc<Mutex<Vec<Pools>>>,
    factory_address: Address,
    token_a: TokenData,
    token_b: TokenData,
) -> Result<(), CustomError<'a>> {
    let contract = IUniswapV3Factory::new(factory_address, provider);

    for fee in [100, 500, 3000, 10000] {
        let fee_u24 = U24::from(fee);
        let pool: Address = contract
            .getPool(token_a.address, token_b.address, fee_u24)
            .call()
            .await?;
        if !pool.is_zero() {
            pools
                .lock()
                .await
                .push(Pools::new(token_a, token_b, fee, pool));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pools {
    token_a: Address,
    token_b: Address,
    decimals0: u8,
    decimals1: u8,
    fee: u16,
    address: Address,
}

impl Pools {
    fn new(token_a: TokenData, token_b: TokenData, fee: u16, address: Address) -> Self {
        Self {
            token_a: token_a.address,
            token_b: token_b.address,
            fee,
            address,
            decimals1: token_a.decimals,
            decimals0: token_b.decimals,
        }
    }
}
enum Exchanges {
    Uniswap,
}

async fn get_addresses_v3<'a>(
    provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    tokens: Vec<TokenData>,
    pools: Arc<Mutex<Vec<Pools>>>,
    exchanges: Exchanges,
) -> Result<(), CustomError<'a>> {
    let n = tokens.len();

    use Exchanges::*;
    let factory = match exchanges {
        Uniswap => address!("0x1F98431c8aD98523631AE4a59f267346ea31F984"),
    };

    let mut handles = Vec::with_capacity((3 * n * (n - 1)) / 2);
    for i in 0..n - 1 {
        for j in (i + 1)..n {
            let provider_clone = provider.clone();
            let pools_clone = Arc::clone(&pools);
            let factory_clone = factory;
            let token_a = tokens[i];
            let token_b = tokens[j];

            handles.push(tokio::spawn(async move {
                get_pair_address_v3(provider_clone, pools_clone, factory_clone, token_a, token_b)
                    .await
            }));
        }
    }

    join_all(handles).await;

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
    let token_data = get_decimal(&provider, &tokens).await?;
    let pools = Arc::new(Mutex::new(Vec::with_capacity(tokens.len() * 2)));
    get_addresses_v3(
        provider.clone(),
        token_data.clone(),
        Arc::clone(&pools),
        Exchanges::Uniswap,
    )
    .await?;

    let uniswap_pools = pools.lock().await.len();
    log::info!("UniswapV3 Pools: {uniswap_pools}");

    let pools_guard = pools.lock().await;
    let pool_addresses: Vec<_> = pools_guard.iter().map(|p| p.address).collect();

    let mut file = File::create("resources/uniswapv3_tokens_to_pool.json")?;
    file.write_all(serde_json::to_string_pretty(&*pools_guard)?.as_bytes())?;

    let mut file = File::create("resources/pools_v3.json")?;
    file.write_all(serde_json::to_string_pretty(&pool_addresses)?.as_bytes())?;

    Ok(())
}
