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
use std::{fs::File, io::BufReader};
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
    token_a: Address,
    token_b: Address,
) -> Result<Option<Pools>, CustomError<'a>> {
    let factory_address = address!("0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");
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

async fn get_addresses_v2<'a>(
    provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    tokens: Vec<Address>,
) -> Result<Vec<Pools>, CustomError<'a>> {
    let n = tokens.len();
    let mut pools = Vec::with_capacity(n);
    for i in 0..n - 1 {
        for j in (i + 1)..n {
            if let Some(pool) = get_pair_address(provider.clone(), tokens[i], tokens[j]).await? {
                pools.push(pool);
            }
        }
    }

    Ok(pools)
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
    let pools = get_addresses_v2(provider, tokens).await?;
    println!("{}", serde_json::to_string_pretty(&pools)?);

    Ok(())
}
