//! Example demonstrating pool creation with tick data provider and swap simulation
//!
//! # Prerequisites
//! - Environment variable MAINNET_RPC_URL must be set
//! - Requires the "extensions" feature
//!
//! # Note
//! This example uses mainnet block 17000000 for consistent results

use alloy::{
    primitives::U256,
    providers::{ProviderBuilder, WsConnect},
};

use uniswap_sdk_core::{prelude::*, token};
use uniswap_v3_sdk::prelude::*;
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
        "0x6b175474e89094c44da98b954eedeac495271d0f",
        18,
        "USDC"
    );

    let weth = WETH9::on_chain(CHAIN_ID).unwrap();

    // Create a pool with a tick map data provider
    let pool = Pool::<EphemeralTickMapDataProvider>::from_pool_key_with_tick_data_provider(
        CHAIN_ID,
        FACTORY_ADDRESS,
        usdc.address(),
        weth.address(),
        FeeAmount::LOW,
        provider.clone(),
        None,
    )
    .await
    .unwrap();

    let amount_in = CurrencyAmount::from_raw_amount(usdc.clone(), 10000000000000u128).unwrap();
    let zero_for_one = amount_in.currency.equals(&pool.token0);

    let mut ticks_initialised = Vec::new();
    let mut ticks = Vec::new();
    let mut current_state = pool.tick_current;
    let mut count = 0;
    loop {
        if let Ok((tick_next, initialized)) = pool
            .tick_data_provider
            .next_initialized_tick_within_one_word(current_state, zero_for_one, pool.tick_spacing())
            .await
        {
            ticks_initialised.push((tick_next, initialized));
            if let Ok(tick) = pool.tick_data_provider.get_tick(tick_next).await {
                ticks.push(tick);
            } else {
                ticks_initialised.pop();
                break;
            }
            current_state = if zero_for_one {
                tick_next - pool.tick_spacing()
            } else {
                tick_next
            };
        } else {
            break;
        };
        count += 1;
    }
    println!("{count}, {}, {}", ticks.len(), ticks_initialised.len());

    let pool_1 = Pool::new(
        usdc.clone(),
        weth.clone(),
        FeeAmount::LOW,
        pool.sqrt_ratio_x96,
        pool.liquidity,
    )
    .unwrap();

    // let amount_out = pool_1
    //     .get_output_amount_sync(&amount_in, None, &ticks_initialised, &ticks)
    //     .unwrap();
    // println!(
    //     "amount_out: {} / {}",
    //     amount_out.numerator(),
    //     amount_out.denominator()
    // );
    let amount_out = pool.get_output_amount(&amount_in, None).await.unwrap();
    println!(
        "amount_out: {} / {}",
        amount_out.numerator(),
        amount_out.denominator()
    );

    let amount_out = CurrencyAmount::from_raw_amount(weth.clone(), 10000000000u128).unwrap();
    let amount_in = pool.get_input_amount(&amount_out, None).await.unwrap();
    println!(
        "amount_in: {} / {}",
        amount_in.numerator(),
        amount_in.denominator()
    );
    println!("{}", "=".repeat(80));
    // let amount_in = pool_1
    //     .get_input_amount_sync(&amount_out, None, &ticks_initialised, &ticks)
    //     .unwrap();
    // println!(
    //     "amount_in: {} / {}",
    //     amount_in.numerator(),
    //     amount_in.denominator()
    // );
}
