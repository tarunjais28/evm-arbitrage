use crate::{
    enums::*,
    structs::*,
    IUniswapV2Pool::{Burn, Mint, Swap, Sync},
};
use alloy::{
    primitives::{Address, TxHash, Uint, U256},
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::Filter,
    rpc::types::Log,
    sol,
};
use colored::Colorize;
use futures_util::stream::StreamExt;
use std::fmt::Display;
use utils::EnvParser;
type U112 = Uint<112, 2>;

mod enums;
mod structs;

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV2Pool,
    "../../resources/uniswapv2_pool_abi.json"
);

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load environment variables from .env file
    let env_parser = EnvParser::new()?;

    // Set up the WS transport and connect.
    let ws = WsConnect::new(env_parser.ws_address);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    // Create a filter for the events.
    let filter = provider
        .subscribe_logs(&Filter::new().address(env_parser.pools_addrs))
        .await?;
    let mut stream = filter.into_stream();

    println!("Waiting for events...");

    // Process events from the stream.
    while let Some(log) = stream.next().await {
        let mut scanner = ScanData::new(&log);

        if let Ok(decoded) = log.log_decode() {
            let swap: Swap = decoded.inner.data;
            scanner.update_swap(swap, decoded.inner.address);
        } else if let Ok(decoded) = log.log_decode() {
            let sync: Sync = decoded.inner.data;
            scanner.update_sync(sync, decoded.inner.address);
        } else if let Ok(decoded) = log.log_decode() {
            let mint: Mint = decoded.inner.data;
            scanner.update_liquidity_events(mint, decoded.inner.address);
        } else if let Ok(decoded) = log.log_decode() {
            let burn: Burn = decoded.inner.data;
            scanner.update_liquidity_events(burn, decoded.inner.address);
        } else {
            continue;
        }
        scanner.show();
    }

    Ok(())
}
