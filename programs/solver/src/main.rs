use alloy::{
    primitives::address,
    providers::{ProviderBuilder, WsConnect},
    sol,
};
use std::sync::Arc;
use utils::EnvParser;

mod util;

sol!(
    #[sol(rpc)]
    IUniswapV2Pair,
    "../../resources/uniswapv2_pair.json"
);

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let env_parser = EnvParser::new()?;

    let provider = ProviderBuilder::new()
        .connect_ws(WsConnect::new(&env_parser.ws_address))
        .await?;

    let contract = IUniswapV2Pair::new(
        address!("0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852"),
        Arc::new(provider),
    );

    let reserves = contract.getReserves().call().await?;

    println!(
        "Reserves: (reserve0: {}, reserve1: {}, blockTimestampLast: {})",
        reserves._reserve0, reserves._reserve1, reserves._blockTimestampLast
    );

    //     println!("{}", format!("current block: {}", block_number).blue());

    //     // Spawn a task for each contract address
    //     let mut tasks = vec![];
    //     for address in &env_parser.pools {
    //         let web3 = web3.clone();
    //         let address = *address;

    //         let task = tokio::spawn(async move {
    //             let contract = Contract::from_json(
    //                 web3.eth(),
    //                 address,
    //                 include_bytes!("../../../resources//uniswap_pool_abi.json"),
    //             )?;

    //             let (events, signatures) =
    //                 get_events(&contract, &["Swap", "Sync", "Mint", "Burn"])?;

    //             scan(web3.clone(), address, &events, signatures, block_hash).await
    //         });

    //         tasks.push(task);
    //     }

    //     // Wait for all tasks to complete
    //     let results = join_all(tasks).await;
    //     for res in results {
    //         if let Err(e) = res {
    //             eprintln!("{}", format!("Error in contract task: {:?}", e).red());
    //         }
    //     }
    // }

    Ok(())
}
