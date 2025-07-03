use alloy::{
    primitives::address,
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::Filter,
};
use futures_util::stream::StreamExt;
use utils::EnvParser;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load environment variables from .env file
    let env_parser = EnvParser::new()?;

    // Set up the WS transport and connect.
    let ws = WsConnect::new(env_parser.ws_address);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    // Create a filter for the events.
    let filter = provider.clone().subscribe_logs(&Filter::new()).await?;

    let mut stream = filter.into_stream();

    let addr = address!("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D");
    // Process events from the stream
    while let Some(log) = stream.next().await {
        if log.inner.data.topics().contains(&addr.into_word()) {
            println!("{:#?}", log);
            break;
        }
    }

    Ok(())
}
