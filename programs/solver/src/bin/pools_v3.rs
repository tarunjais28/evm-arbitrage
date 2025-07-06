use alloy::{
    primitives::{address, aliases::U24, Address},
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

pub async fn get_pair_address_v3<'a>(
    provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pools: Arc<Mutex<Vec<PoolV3>>>,
    factory_address: Address,
    token_a: Address,
    token_b: Address,
) -> Result<(), CustomError<'a>> {
    let contract = IUniswapV3Factory::new(factory_address, provider);

    for fee in [100, 500, 3000, 10000] {
        let fee_u24 = U24::from(fee);
        let pool: Address = contract.getPool(token_a, token_b, fee_u24).call().await?;
        if !pool.is_zero() {
            pools
                .lock()
                .await
                .push(PoolV3::new(token_a, token_b, fee, pool));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolV3 {
    token_a: Address,
    token_b: Address,
    fee: u16,
    address: Address,
}

impl PoolV3 {
    fn new(token_a: Address, token_b: Address, fee: u16, address: Address) -> Self {
        Self {
            token_a,
            token_b,
            fee,
            address,
        }
    }
}

enum Exchanges {
    Sushi,
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
    tokens: Vec<Address>,
    pools: Arc<Mutex<Vec<PoolV3>>>,
    exchanges: Exchanges,
) -> Result<(), CustomError<'a>> {
    let n = tokens.len();

    use Exchanges::*;
    let factory = match exchanges {
        Uniswap => address!("0x1F98431c8aD98523631AE4a59f267346ea31F984"),
        Sushi => address!(),
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
    // Load environment variables from .env file
    let env_parser = EnvParser::new()?;

    // Set up the WS transport and connect.
    let ws = WsConnect::new(env_parser.ws_address);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    let file = File::open("resources/tokens.json")?;
    let reader = BufReader::new(file);

    // Parse and decode addresses
    let tokens: Vec<Address> = from_reader(reader)?;
    let pools = Arc::new(Mutex::new(Vec::with_capacity(tokens.len() * 2)));
    get_addresses_v3(
        provider.clone(),
        tokens.clone(),
        Arc::clone(&pools),
        Exchanges::Uniswap,
    )
    .await?;

    let pools_guard = pools.lock().await;
    let pool_addresses: Vec<_> = pools_guard.iter().map(|p| p.address).collect();

    let mut file = File::create("resources/uniswapv3_tokens_to_pool.json")?;
    file.write_all(serde_json::to_string_pretty(&*pools_guard)?.as_bytes())?;

    let mut file = File::create("resources/pools_v3.json")?;
    file.write_all(serde_json::to_string_pretty(&pool_addresses)?.as_bytes())?;

    Ok(())
}
