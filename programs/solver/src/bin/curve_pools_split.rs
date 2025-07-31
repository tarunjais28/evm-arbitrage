use alloy::{
    primitives::{address, Address, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use colored::Colorize;
use futures::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::File,
    io::Write,
    sync::{Arc, Mutex},
};
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
    "../../resources/contracts/curve_registry_contract.json"
);

#[derive(Debug, Deserialize, Serialize)]
struct Pools {
    meta: HashSet<Address>,
    unspecified: HashSet<Address>,
}

async fn get_pools<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
) -> Pools {
    let meta_addr = address!("0x0000000022D53366457F9d5E68Ec105046FC4383");
    let contract = MetaAddress::new(meta_addr, provider);
    let mut multicall = provider.multicall().dynamic();

    for i in 0..15 {
        multicall = multicall.add_dynamic(contract.get_address(U256::from(i)));
    }

    let mut metapools = multicall.aggregate().await.unwrap();
    metapools.retain(|addr| addr != &address!());

    let pools = Arc::new(Mutex::new(Pools {
        meta: HashSet::new(),
        unspecified: HashSet::new(),
    }));

    let meta_registry = Arc::new(Mutex::new(HashSet::new()));
    let mut tasks = FuturesUnordered::new();

    for meta in metapools {
        let provider = provider.clone();
        let pools = Arc::clone(&pools);
        let meta_registry = Arc::clone(&meta_registry);
        tasks.push(tokio::spawn(async move {
            let contract = CurvePools::new(meta, &provider);
            if let Ok(count) = contract.pool_count().call().await {
                let n = count.to_string().parse::<usize>().unwrap();
                println!("{}", format!("Processing {n} pools...").blue());

                let mut inner_tasks = FuturesUnordered::new();

                for i in 0..n {
                    let provider_clone = provider.clone();
                    let pools = Arc::clone(&pools);
                    let meta_registry = Arc::clone(&meta_registry);

                    inner_tasks.push(tokio::spawn(async move {
                        let contract = CurvePools::new(meta, &provider_clone);
                        if let Ok(addr) = contract.pool_list(U256::from(i)).call().await {
                            if let Ok(is_meta) = contract.is_meta(addr).call().await {
                                let mut pools = pools.lock().unwrap();
                                if is_meta {
                                    pools.meta.insert(addr);
                                    meta_registry.lock().unwrap().insert(meta);
                                } else {
                                    pools.unspecified.insert(addr);
                                }
                            } else {
                                pools.lock().unwrap().unspecified.insert(addr);
                            }
                        } else {
                            eprintln!(
                                "{}",
                                format!("pool extract: {meta}, i: {i} -> pool list").red()
                            );
                        }
                    }));
                }

                while let Some(_) = inner_tasks.next().await {}
            } else {
                eprintln!("{}", format!("meta_pool: {meta} -> coin count").red())
            }
        }));
    }

    while let Some(_) = tasks.next().await {}

    let pools = Arc::try_unwrap(pools).unwrap().into_inner().unwrap();
    let meta_registry = Arc::try_unwrap(meta_registry)
        .unwrap()
        .into_inner()
        .unwrap();

    println!(
        "{}",
        format!("{} meta pools found!", pools.meta.len()).green()
    );
    println!(
        "{}",
        format!("{} unspecified pools found!", pools.unspecified.len()).green()
    );
    println!(
        "{}",
        format!("Meta registries: {:#?}", meta_registry).green()
    );

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

    let mut file = File::create("resources/curve_pools_splitted.json").unwrap();
    file.write_all(serde_json::to_string_pretty(&pools).unwrap().as_bytes())
        .unwrap();
}
