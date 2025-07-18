use alloy::{
    primitives::address,
    providers::{Provider, ProviderBuilder, WsConnect},
};
use utils::EnvParser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Logger initialized");

    // Load environment variables from .env file
    let env_parser = EnvParser::new()?;
    let addr = address!("0xf98cf0d979cfbb780774f318e3da4f7317af50d7");

    // Set up the WS transport and connect.
    let ws = WsConnect::new(env_parser.ws_address);
    let provider = ProviderBuilder::new().connect_ws(ws).await?;

    // Get latest block
    let latest_block = provider.get_block_number().await?;
    println!("Latest block: {}", latest_block);

    let mut low = 0;
    let mut high = latest_block;

    let mut deployment_block: Option<u64> = None;

    while low <= high {
        let mid = (low + high) / 2;
        let code = provider.get_code_at(addr).await?;

        if code.0.is_empty() {
            // Not deployed yet
            low = mid + 1;
        } else {
            // Code exists here, search lower
            deployment_block = Some(mid);
            if mid == 0 {
                break; // Must be genesis
            }
            high = mid - 1;
        }
    }

    match deployment_block {
        Some(block) => println!("Contract deployed in block {}", block),
        None => println!("Contract code not found (maybe wrong address)"),
    }

    Ok(())
}
