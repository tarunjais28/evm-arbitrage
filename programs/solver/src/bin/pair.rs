use alloy::{
    primitives::{address, Address},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use uniswap_sdk_core::{prelude::*, token};
use uniswap_v2_sdk::prelude::*;
use utils::{CustomError, EnvParser};

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV2Pair,
    "../../resources/contracts/uniswapv2_pair.json"
);

#[derive(Default, Debug, Clone, Copy)]
pub struct Reserves {
    pub reserve0: BigInt,
    pub reserve1: BigInt,
}

pub async fn get_reserves_v2<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pool: &Address,
) -> Result<Reserves, CustomError<'a>> {
    let contract = IUniswapV2Pair::new(*pool, provider);
    let reserves = contract.getReserves().call().await?;

    Ok(Reserves {
        reserve0: U256::from(reserves._reserve0).to_big_int(),
        reserve1: U256::from(reserves._reserve1).to_big_int(),
    })
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

    let pool_address = address!("0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc");
    let token0 = token!(1, "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48", 18, "USDC");
    let token1 = WETH9::on_chain(1).unwrap();

    let reserves = get_reserves_v2(&provider, &pool_address).await?;
    let token_amount_a = CurrencyAmount::from_raw_amount(token0, reserves.reserve0).unwrap();
    let token_amount_b = CurrencyAmount::from_raw_amount(token1, reserves.reserve1).unwrap();
    let pair = Pair::new(token_amount_a.clone(), token_amount_b.clone())?;

    println!("pair address: {}", pair.address());
    println!("token0 price: {}", pair.token0_price().quotient());
    println!("reserve0: {}", reserves.reserve0);
    println!("reserve0: {}", pair.reserve0().quotient());
    println!("token1 price: {}", pair.token1_price().quotient());
    println!("reserve1: {}", reserves.reserve1);
    println!("reserve1: {}", pair.reserve1().quotient());

    let res = get_reserves_v2(
        &provider,
        &address!("0x559eBE4E206e6B4D50e9bd3008cDA7ce640C52cb"),
    )
    .await?;

    println!("{:#?}", res);
    Ok(())
}
