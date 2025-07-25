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
    IUniswapV3Factory,
    "../../resources/contracts/uniswapv3_factory.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    ERC20,
    "../../resources/contracts/erc20_abi.json"
);

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TokenData {
    address: Address,
    name: String,
    symbol: String,
    decimals: u8,
}

async fn get_token_metadata<'a>(
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
            let name = contract.name().call().await?;
            let symbol = contract.symbol().call().await?;
            let decimals = contract.decimals().call().await?;
            Ok(TokenData {
                address,
                name,
                symbol,
                decimals,
            })
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
            token_data.push(token.clone());
        }
    });

    Ok(token_data)
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

    let file = File::open("resources/curve_tokens.json")?;
    let reader = BufReader::new(file);

    // Parse and decode addresses
    let tokens: Vec<Address> = from_reader(reader)?;
    let token_data = get_token_metadata(&provider, &tokens).await?;

    let mut file = File::create("resources/curve_token_metadata.json")?;
    file.write_all(serde_json::to_string_pretty(&token_data)?.as_bytes())?;

    log::info!("{} tokens proessed!", token_data.len());

    Ok(())
}
