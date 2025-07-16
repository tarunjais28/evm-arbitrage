use alloy::{
    primitives::address,
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::Filter,
    sol,
};
use utils::{debug_time, EnvParser};

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV3Pool,
    "../../resources/contracts/uniswapv3_pool_abi.json"
);

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

    let addr = address!("0x477e1a178f308fb8c2967d3e56e157c4b8b6f5df");

    let mut block_number = 12688517;
    let mut next_block = block_number + 100000;
    let target_block = provider.get_block_number().await?;

    let mut count = 0;
    debug_time!("Block scanning", {
        while block_number <= target_block {
            // Create a filter for the events.
            let logs = provider
                .clone()
                .get_logs(
                    &Filter::new()
                        .address(addr)
                        .from_block(block_number)
                        .to_block(next_block),
                )
                .await?;
            count += 1;

            for log in logs {
                if let Ok(decoded) = log.log_decode::<IUniswapV3Pool::Mint>() {
                    let mint = decoded.inner.data;
                    println!("{:#?}, {}", mint, decoded.inner.address);
                } else if let Ok(decoded) = log.log_decode::<IUniswapV3Pool::Burn>() {
                    let burn = decoded.inner.data;
                    println!("{:#?}, {}", burn, decoded.inner.address);
                }
            }

            block_number = next_block;
            next_block += 100000;
        }
    });

    log::info!("Rpc hits: {count}");

    Ok(())
}
