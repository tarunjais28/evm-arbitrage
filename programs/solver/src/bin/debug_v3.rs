//! Example demonstrating pool creation with tick data provider and swap simulation
//!
//! # Prerequisites
//! - Environment variable MAINNET_RPC_URL must be set
//! - Requires the "extensions" feature
//!
//! # Note
//! This example uses mainnet block 17000000 for consistent results

use std::{fs::File, io::BufReader};

use alloy::{
    primitives::{address, U160, U256},
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::TransactionRequest,
    sol_types::SolCall,
};

use alloy_primitives::aliases::I24;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use uniswap_sdk_core::{prelude::*, token};
use uniswap_v3_sdk::prelude::{tick_sync::TickSync, *};
use utils::EnvParser;

/// Computes price = (sqrtPriceX96)^2 / 2^192
/// Returns price as an f64 for readability.
///
/// sqrtPriceX96 is the Q96 fixed-point square root price.
pub fn price_from_sqrt_price_x96(sqrt_price_x96: U256) -> (U256, U256) {
    // Numerator: (sqrtPriceX96)^2
    let numerator = sqrt_price_x96 * sqrt_price_x96;

    // Denominator: 2^192 = 1 << 192
    let denominator = U256::ONE << 192;

    let price = numerator.checked_div(denominator).unwrap_or_default();
    let rec = U256::ONE.checked_div(price).unwrap_or_default();
    (price, rec)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TickDetails {
    block: u64,
    pool: Address,
    current_tick: I24,
    sqrt_price_x96: U160,
    liquidity: u128,
    ticks: Vec<TickSync>,
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    let env_parser = EnvParser::new().unwrap();

    // Set up the WS transport and connect.
    let ws = WsConnect::new(env_parser.ws_address);
    let provider = ProviderBuilder::new().connect_ws(ws).await.unwrap();

    const CHAIN_ID: u64 = 1;
    let usdc = token!(
        CHAIN_ID,
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
        6,
        "LINK",
        "ChainLink Token"
    );

    let usdt = token!(
        CHAIN_ID,
        "0xdac17f958d2ee523a2206206994597c13d831ec7",
        6,
        "LINK",
        "ChainLink Token"
    );

    let weth = WETH9::on_chain(CHAIN_ID).unwrap();

    // let pool = Pool::<EphemeralTickMapDataProvider>::from_pool_key_with_tick_data_provider(
    //     1,
    //     FACTORY_ADDRESS,
    //     usdc.address,
    //     weth.address,
    //     FeeAmount::LOWEST,
    //     provider,
    //     None,
    // )
    // .await
    // .unwrap();

    let amount_in = CurrencyAmount::from_raw_amount(usdc.clone(), 1000000000000000000u128).unwrap();
    // let zero_for_one = amount_in.currency.equals(&pool.token0);

    // let (tick_next, initialized) = pool
    //     .tick_data_provider
    //     .next_initialized_tick_within_one_word(pool.tick_current, zero_for_one, pool.tick_spacing())
    //     .await
    //     .unwrap();

    // println!("{}, {tick_next}", pool.tick_current); // 193934

    // let amount_out = pool.get_output_amount(&amount_in, None).await.unwrap();
    // println!("amount_out: {}", amount_out.quotient(),);

    // let amount_in = CurrencyAmount::from_raw_amount(weth.clone(), 1000000000000000000u128).unwrap();
    // let amount_out = pool.get_output_amount(&amount_in, None).await.unwrap();
    // println!("{}", pool.tick_spacing());
    // println!("{}", amount_out.quotient());
    // println!("{}", pool.address(None, None));

    let file = File::open("resources/ticks.json").unwrap();
    let reader = BufReader::new(file);
    let tick_details: Vec<TickDetails> = from_reader(reader).unwrap();

    let pool_addr = address!("0xe0554a476a092703abdb3ef35c80e0d76d32939f");

    let td = tick_details.iter().find(|t| t.pool == pool_addr).unwrap();

    let pool_1: Pool = Pool::new(
        usdc.clone(),
        weth.clone(),
        FeeAmount::LOWEST,
        td.sqrt_price_x96,
        td.liquidity,
    )
    .unwrap();
    println!("{}", pool_1.sqrt_ratio_x96);

    let amount_out = pool_1
        .get_output_amount_sync(&amount_in, None, td.current_tick, &td.ticks)
        .unwrap();
    println!(
        "amount_out: {} / {}",
        amount_out.numerator(),
        amount_out.denominator()
    );

    // let amount_out = pool.get_output_amount(&amount_in, None).await.unwrap();
    // println!(
    //     "amount_out: {} / {}",
    //     amount_out.numerator(),
    //     amount_out.denominator()
    // );

    // let amount_out =
    //     CurrencyAmount::from_raw_amount(weth.clone(), 1000000000000000000u128).unwrap();

    // let amount_in = pool.get_input_amount(&amount_out, None).await.unwrap();
    // println!(
    //     "amount_in: {} / {}",
    //     amount_in.numerator(),
    //     amount_in.denominator()
    // );

    // println!("{}", "=".repeat(80));

    // let amount_in = pool_1
    //     .get_input_amount_sync(&amount_out, None, &ticks_initialised, &ticks)
    //     .unwrap();
    // println!(
    //     "amount_in: {} / {}",
    //     amount_in.numerator(),
    //     amount_in.denominator()
    // );
}
