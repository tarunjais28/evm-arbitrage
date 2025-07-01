use alloy::{
    primitives::{address, Address},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
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
    "../../resources/uniswapv2_factory.json"
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
    address: Address,
}

impl Pools {
    fn new(token_a: Address, token_b: Address, address: Address) -> Self {
        Self {
            token_a,
            token_b,
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
    exchanges: Exchanges, 
) -> Result<(), CustomError<'a>> {
    let n = tokens.len();
    let mut pools = Vec::with_capacity(n);

    use Exchanges::*;
    let (factory, mut file) = match exchanges {
        Uniswap => {
            (address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"), File::create("resources/sushiv2_tokens_to_pool.json")?)
        }
        Sushi =>{
            (address!("0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac"), File::create("resources/uniswapv2_tokens_to_pool.json")?)

        }
    };

    for i in 0..n - 1 {
        for j in (i + 1)..n {
            if let Some(pool) = get_pair_address(provider.clone(), factory, tokens[i], tokens[j]).await? {
                pools.push(pool);
            }
        }
    }

    file.write_all(serde_json::to_string_pretty(&pools)?.as_bytes())?;

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
    get_addresses_v2(provider.clone(), tokens.clone(), Exchanges::Uniswap).await?;
    get_addresses_v2(provider, tokens, Exchanges::Sushi).await?;

    Ok(())
}
