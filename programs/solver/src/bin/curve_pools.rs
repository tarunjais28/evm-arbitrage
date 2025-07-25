use std::{fs::File, io::Write};

use alloy::{
    primitives::{address, Address, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use colored::Colorize;
use std::collections::HashSet;
use utils::{debug_time, EnvParser};

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    MetaAddress,
    "../../resources/contracts/curve_address_provider.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    CurvePools,
    "../../resources/contracts/curve_pools.json"
);

pub async fn get_pools<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
) -> HashSet<Address> {
    let meta_addr = address!("0x0000000022D53366457F9d5E68Ec105046FC4383");
    let contract = MetaAddress::new(meta_addr, provider);
    let mut multicall = provider.multicall().dynamic();

    for i in 0..15 {
        multicall = multicall.add_dynamic(contract.get_address(U256::from(i)));
    }
    let mut metapools = multicall.aggregate().await.unwrap();
    metapools.retain(|addr| addr != &address!());

    let mut pools: HashSet<Address> = HashSet::with_capacity(3000);
    for meta in metapools {
        let contract = CurvePools::new(meta, provider);
        if let Ok(count) = contract.pool_count().call().await {
            let n = count.to_string().parse::<usize>().unwrap();
            println!("{}", format!("Processing {n} pools...").blue());
            for i in 0..n {
                if let Ok(addr) = contract.pool_list(U256::from(i)).call().await {
                    pools.insert(addr);
                } else {
                    eprintln!(
                        "{}",
                        format!("pool extract: {meta}, i: {i} -> pool list").red()
                    )
                }
            }
        } else {
            eprintln!("{}", format!("meta_pool: {meta} -> coin count").red())
        }
    }
    pools.retain(|addr| addr != &address!());
    println!("{}", format!("pools found: {}", pools.len()).green());

    pools
}

#[tokio::main]
async fn main() {
    // Initialize the logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Logger initialized");

    // Load environment variables from .env file
    let env_parser = EnvParser::new().unwrap();

    // Set up the WS transport and connect.
    let ws = WsConnect::new(env_parser.ws_address);
    let provider = ProviderBuilder::new().connect_ws(ws).await.unwrap();

    let pools = debug_time!("get_pools()", { get_pools(&provider).await });

    let mut file = File::create("resources/curve_pools.json").unwrap();
    file.write_all(serde_json::to_string_pretty(&pools).unwrap().as_bytes())
        .unwrap();
}
