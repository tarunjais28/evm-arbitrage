use crate::{
    enums::*,
    fetch::*,
    parser::*,
    pools::*,
    scanner::*,
    slippage::*,
    structs::*,
    IUniswapV2Pool::{Burn, Mint, Swap, Sync},
};
use alloy::{
    primitives::{Address, TxHash, Uint, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    rpc::types::{Filter, Log},
    sol,
};
use colored::Colorize;
use dotenv::dotenv;
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{
    collections::HashMap,
    env,
    fmt::Display,
    fs::{File, OpenOptions},
    io::{BufReader, Write},
    sync::Arc,
};

use tokio::sync::Mutex;
use utils::CustomError;

type U112 = Uint<112, 2>;

mod enums;
mod fetch;
mod parser;
#[macro_use]
mod logger;
mod pools;
mod scanner;
mod slippage;
mod structs;

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV2Pool,
    "../../resources/contracts/uniswapv2_pool_abi.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV2Pair,
    "../../resources/contracts/uniswapv2_pair.json"
);

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize the logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Logger initialized");

    info_time!("main()", {
        // Load environment variables from .env file
        let env_parser = info_time!("env_parser", { EnvParser::new()? });

        // Set up the WS transport and connect.
        let ws = WsConnect::new(env_parser.ws_address);
        let provider = ProviderBuilder::new().connect_ws(ws).await?;

        let file = debug_time!("file_open()", {
            File::open("resources/tokens_to_pool_weth.json")?
        });
        let reader = debug_time!("reader()", { BufReader::new(file) });
        let pools: Vec<Pools> = debug_time!("pools_serialize()", { from_reader(reader)? });
        let pool_data: PoolData = debug_time!("update_reserves", {
            update_reserves(provider.clone(), pools).await?
        });

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open("resources/run.txt")?;

        file.write_all(SearchData::headers().as_bytes())?;

        pool_data.iter().for_each(|(pool_addr, token_data)| {
            let scan_data = SearchData::new(
                u64::default(),
                pool_addr.clone(),
                token_data.token_a,
                token_data.token_b,
                token_data.reserve0,
                token_data.reserve1,
            );
            let _ = file.write_all(scan_data.to_string().as_bytes());
        });

        // Scanning the ethereum blockchain for events
        debug_time!("Calling scanner()", {
            scan(
                provider.clone(),
                env_parser.pools_addrs,
                pool_data,
                &mut file,
            )
            .await?
        });
    });

    Ok(())
}
