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
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::TransactionRequest,
    sol_types::SolCall,
};

use alloy_primitives::aliases::I24;
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
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
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

    let tick_data_provider = EphemeralTickMapDataProvider::new(
        pool.address(None, Some(FACTORY_ADDRESS)),
        provider,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let amount_in = CurrencyAmount::from_raw_amount(usdc.clone(), 10000000000000u128).unwrap();
    let zero_for_one = amount_in.currency.equals(&pool.token0);

    // let (tick_next, initialized) = pool.tick_data_provider
    //         .next_initialized_tick_within_one_word(pool.tick_current, zero_for_one, pool.tick_spacing())
    //         .await.unwrap();

    // println!("{}", pool.tick_current);
    // println!("{tick_next}, {initialized}");

    // let (tick_next, initialized) = pool.tick_data_provider
    //         .next_initialized_tick_within_one_word(tick_next, zero_for_one, pool.tick_spacing())
    //         .await.unwrap();
    // println!("{tick_next}, {initialized}");

    let mut ticks_initialised = Vec::new();
    let mut ticks = Vec::new();
    let mut current_state = pool.tick_current;
    let mut count = 0;
    loop {
        if let Ok((tick_next, initialized)) = tick_data_provider
            .next_initialized_tick_within_one_word(current_state, zero_for_one, pool.tick_spacing())
            .await
        {
            ticks_initialised.push((tick_next, initialized));
            if let Ok(tick) = tick_data_provider.get_tick(tick_next).await {
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

    // println!("{:#?}", ticks);
    let pool_1 = Pool::new(
        usdc.clone(),
        weth.clone(),
        FeeAmount::LOW,
        pool.sqrt_ratio_x96,
        pool.liquidity,
    )
    .unwrap();

    let amount_out = pool_1
        .get_output_amount_sync(&amount_in, None, &ticks_initialised, &ticks)
        .unwrap();
    println!(
        "amount_out: {} / {}",
        amount_out.numerator(),
        amount_out.denominator()
    );
    let amount_out = pool.get_output_amount(&amount_in, None).await.unwrap();
    println!(
        "amount_out: {} / {}",
        amount_out.numerator(),
        amount_out.denominator()
    );
    // // Get the output amount from the pool
    // let amount_in = CurrencyAmount::from_raw_amount(usdc.clone(), 100000000).unwrap();
    // let amount_in_b =
    //     CurrencyAmount::from_raw_amount(weth.clone(), 100000000000000000u128).unwrap();
    // let local_amount_out = pool.get_output_amount(&amount_in, None).await.unwrap();
    // let local_amount_out_b = pool.get_input_amount(&amount_in_b, None).await.unwrap();

    // let price_usdc = pool.price_of(&usdc).unwrap().quotient();
    // let price_weth = pool.price_of(&weth).unwrap().quotient();
    // let sqrt = pool.sqrt_ratio_x96;
    // let local_amount_out = local_amount_out.quotient();
    // println!("Local amount out: {}", local_amount_out);
    // println!("Local amount in: {}", local_amount_out_b.quotient());
    // println!("Price USDC: {}", price_usdc);
    // println!("Price WETH: {}", price_weth);
    // println!("sqrt price: {}", sqrt);
    // println!("price: {:?}", price_from_sqrt_price_x96(U256::from(sqrt)));
    // println!(
    //     "price: {} {}",
    //     pool.token0_price().quotient(),
    //     pool.token1_price().quotient()
    // );
    // println!("address: {}", pool.address(None, None));
    // println!("address: {}", pool.address(None, None));
    // println!("liquidity: {}", pool.tick_spacing());

    // // Get the output amount from the quoter
    // let route = Route::new(vec![pool], usdc, weth.clone());
    // let params = quote_call_parameters(&route, &amount_in, TradeType::ExactInput, None);
    // let tx = TransactionRequest::default()
    //     .to(*QUOTER_ADDRESSES.get(&CHAIN_ID).unwrap())
    //     .input(params.calldata.into());
    // let res = provider.call(tx).await.unwrap();
    // let amount_out =
    //     IQuoter::quoteExactInputSingleCall::abi_decode_returns_validate(res.as_ref()).unwrap();
    // println!("Quoter amount out: {}", amount_out);

    // // Compare local calculation with on-chain quoter to ensure accuracy
    // assert_eq!(U256::from_big_int(local_amount_out), amount_out);
}
