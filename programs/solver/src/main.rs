use crate::{
    contracts::*, dijkstra::*, enums::*, fetch::*, helper::*, parser::*, pools::*, scanner::*,
    slippage::*, structs::*,
};
use alloy::{
    primitives::{
        address,
        aliases::{I24, U160},
        Address, TxHash, U256,
    },
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
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    env,
    fmt::Display,
    fs::File,
    io::BufReader,
    sync::Arc,
};
use tokio::sync::{mpsc, Mutex};
use uniswap_sdk_core::{prelude::*, token};
use utils::{debug_time, info_time, CustomError};

mod contracts;
mod dijkstra;
mod enums;
mod fetch;
mod helper;
mod parser;
mod pools;
mod scanner;
mod slippage;
mod structs;

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

        let token_map: TokenMap = debug_time!("token_map creation()", {
            token_metadata_to_tokens(&env_parser.token_metadata)
        });

        let mut pool_data_v2: v2::PoolData = debug_time!("pool_data_v2()", {
            v2::PoolData::new(&env_parser.pools_v2, &token_map)?
        });

        debug_time!("update_reserve()", {
            pool_data_v2
                .update_reserves(&provider, &env_parser.pool_address.v2)
                .await?
        });

        debug_time!("calulate_start_price_v2()", {
            pool_data_v2.calc_start_price()?
        });

        let mut pool_data_v3: v3::PoolData = debug_time!("pool_data_v3()", {
            v3::PoolData::new(&env_parser.pools_v3, &token_map)?
        });

        debug_time!("calulate_start_price_v3()", {
            pool_data_v3.calc_start_price(&provider).await?
        });

        // Scanning the ethereum blockchain for events
        debug_time!("Calling scanner()", {
            scan(
                provider.clone(),
                env_parser.pool_address.single(),
                pool_data_v2,
                pool_data_v3,
            )
            .await?
        });
    });

    Ok(())
}
