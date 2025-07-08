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

use uniswap_sdk_core::{prelude::*, token};
use uniswap_v3_sdk::prelude::*;
use utils::EnvParser;

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

    // Get the output amount from the pool
    let amount_in = CurrencyAmount::from_raw_amount(usdc.clone(), 100000000).unwrap();
    let amount_in_b =
        CurrencyAmount::from_raw_amount(weth.clone(), 100000000000000000u128).unwrap();
    let local_amount_out = pool.get_output_amount(&amount_in, None).await.unwrap();
    let local_amount_out_b = pool.get_input_amount(&amount_in_b, None).await.unwrap();
    let local_amount_out = local_amount_out.quotient();
    println!("Local amount out: {}", local_amount_out);
    println!("Local amount in: {}", local_amount_out_b.quotient());

    // Get the output amount from the quoter
    let route = Route::new(vec![pool], usdc, weth);
    let params = quote_call_parameters(&route, &amount_in, TradeType::ExactInput, None);
    let tx = TransactionRequest::default()
        .to(*QUOTER_ADDRESSES.get(&CHAIN_ID).unwrap())
        .input(params.calldata.into());
    let res = provider.call(tx).await.unwrap();
    let amount_out =
        IQuoter::quoteExactInputSingleCall::abi_decode_returns_validate(res.as_ref()).unwrap();
    println!("Quoter amount out: {}", amount_out);

    // Compare local calculation with on-chain quoter to ensure accuracy
    assert_eq!(U256::from_big_int(local_amount_out), amount_out);
}
