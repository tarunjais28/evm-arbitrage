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
    tick_spacing: i32,
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
    let link = token!(
        CHAIN_ID,
        "0x6b175474e89094c44da98b954eedeac495271d0f",
        18,
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

    let pool = Pool::<EphemeralTickMapDataProvider>::from_pool_key_with_tick_data_provider(
        1,
        FACTORY_ADDRESS,
        weth.address(),
        usdt.address(),
        FeeAmount::HIGH,
        provider,
        None,
    )
    .await
    .unwrap();

    let amount_in = CurrencyAmount::from_raw_amount(weth.clone(), 1000000000000000000u128).unwrap();
    let amount_out = pool.get_output_amount(&amount_in, None).await.unwrap();
    println!("{}", pool.tick_spacing());
    println!("{}", amount_out.quotient());
    println!("{}", pool.address(None, None));

    let file = File::open("resources/ticks.json").unwrap();
    let reader = BufReader::new(file);
    let tick_details: Vec<TickDetails> = from_reader(reader).unwrap();

    let pool_addr = address!("0x48da0965ab2d2cbf1c17c09cfb5cbe67ad5b1406");

    let td = tick_details.iter().find(|t| t.pool == pool_addr).unwrap();

    let pool_1: Pool = Pool::new(
        link.clone(),
        usdt.clone(),
        FeeAmount::LOWEST,
        td.sqrt_price_x96,
        td.liquidity,
    )
    .unwrap();
    println!("{}", pool_1.sqrt_ratio_x96);

    let amount_in = CurrencyAmount::from_raw_amount(link.clone(), 1000000000000000000u128).unwrap();

    let amount_out = pool_1
        .get_output_amount_sync(&amount_in, None, td.current_tick, &td.ticks )
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

// expected: 335609224399286 / 1, 2980871771211812930840 / 1
// found: 335609224399286 / 1, 999999999999997765 / 1
// 0x6B175474E89094C44Da98b954EedeAC495271d0F, 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2, 0xC2e9F25Be6257c210d7Adf0D4Cd6E3E881ba25f8

// expected: 336837542675735 / 1, 2970677075917217228468 / 1
// found: 336837542675735 / 1, 999999999999998894 / 1
// 0x6B175474E89094C44Da98b954EedeAC495271d0F, 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2, 0x60594a405d53811d3BC4766596EFD80fd545A270
