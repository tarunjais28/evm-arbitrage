// use alloy_network::Ethereum;
use alloy::{
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::Filter,
    sol,
};
use futures_util::stream::StreamExt;
use utils::EnvParser;

sol! {
    event Swap(
        address indexed sender,
        uint amount0In,
        uint amount1In,
        uint amount0Out,
        uint amount1Out,
        address indexed to
    );

    event Sync(
        uint112 reserve0,
        uint112 reserve1,
    );
}

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
        if let Ok(decoded) = log.log_decode() {
            let swap: Swap = decoded.inner.data;
            println!("New Swap!");
            println!("  TxHash: {:?}", log.transaction_hash);
            println!("  BlockNumber: {}", log.block_number.unwrap_or_default());
            println!("  PoolAddress: {}", decoded.inner.address);
            println!("  Sender: {}", swap.sender);
            println!("  To: {}", swap.to);
            println!("  Amount 0 In: {}", swap.amount0In);
            println!("  Amount 1 In: {}", swap.amount1In);
            println!("  Amount 0 Out: {}", swap.amount0Out);
            println!("  Amount 1 Out: {}", swap.amount1Out);
            println!("---------------------------------");
        } else if let Ok(decoded) = log.log_decode() {
            let sync: Sync = decoded.inner.data;
            println!("New Sync!");
            println!("  TxHash: {:?}", log.transaction_hash);
            println!("  BlockNumber: {}", log.block_number.unwrap_or_default());
            println!("  PoolAddress: {}", decoded.inner.address);
            println!("  Reserve 0: {}", sync.reserve0);
            println!("  Reserve 1: {}", sync.reserve1);
            println!("---------------------------------");
        }
    }

    Ok(())
}
