use alloy::{
    primitives::{aliases::I24, Address},
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::types::Filter,
    sol,
};
use dashmap::DashMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{fs::File, io::{BufReader, Write}, sync::Arc};
use utils::{debug_time, EnvParser};

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV3Pool,
    "../../resources/contracts/uniswapv3_pool_abi.json"
);

#[derive(Debug, Serialize, Deserialize)]
struct TickData {
    block: u64,
    tick_details: Vec<TickDetails>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TickDetails {
    pub pool: Address,
    pub ticks: Vec<Tick>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tick {
    pub index: i32,
    pub liquidity_gross: u128,
    pub liquidity_net: i128,
}

impl Tick {
    fn from_extract(tick_extract: TickExtract, amount: i128) -> Vec<Self> {
        vec![
            Self {
                index: tick_extract.tick_lower,
                liquidity_gross: amount as u128,
                liquidity_net: amount * -1,
            },
            Self {
                index: tick_extract.tick_lower,
                liquidity_gross: amount as u128,
                liquidity_net: amount,
            },
        ]
    }
}

impl TickData {
    fn new(block: u64, size: usize) -> Self {
        Self {
            block,
            tick_details: Vec::with_capacity(size),
        }
    }

    fn insert(&mut self, tick_extract: TickExtract, amount: i128) {
        let mut tick = Tick::from_extract(tick_extract, amount);
        if let Some(idx) = self
            .tick_details
            .iter()
            .position(|td| tick_extract.pool.eq(&td.pool))
        {
            self.tick_details[idx].ticks.append(&mut tick);
        } else {
            self.tick_details.push(TickDetails {
                pool: tick_extract.pool,
                ticks: Tick::from_extract(tick_extract, amount),
            });
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Copy)]
pub struct TickExtract {
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub pool: Address,
}

impl TickExtract {
    fn new(tick_lower: I24, tick_upper: I24, pool: Address) -> Self {
        Self {
            tick_lower: tick_lower.as_i32(),
            tick_upper: tick_upper.as_i32(),
            pool,
        }
    }
}

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

    let file = File::open("resources/pools_v3.json").unwrap();
    let reader = BufReader::new(file);

    // Parse and decode addresses
    let pools: Vec<Address> = from_reader(reader).unwrap();

    // Uniswap v3 factory deployed at this block
    let mut block_number = 12369621;
    let limit = 100000;
    let mut next_block = block_number + limit;
    let target_block = provider.get_block_number().await? + limit;
    let ticks: Arc<DashMap<TickExtract, i128>> = Arc::new(DashMap::new());
    let mut count = 1;

    let size = pools.len();
    let filter = Filter::new()
        .address(pools)
        .from_block(block_number)
        .to_block(next_block);

    let mut latest_block = 0;
    debug_time!("Block scanning", {
        while block_number <= target_block {
            // Create a filter for the events.
            let logs = provider.clone().get_logs(&filter).await?;
            count += 1;

            let ticks_clone = Arc::clone(&ticks);
            logs.par_iter().for_each(move |log| {
                if let Ok(decoded) = log.log_decode::<IUniswapV3Pool::Mint>() {
                    let mint = decoded.inner.data;
                    let amount = mint.amount as i128;
                    let key =
                        TickExtract::new(mint.tickLower, mint.tickUpper, decoded.inner.address);
                    *ticks_clone.entry(key).or_insert(0) += amount;
                } else if let Ok(decoded) = log.log_decode::<IUniswapV3Pool::Burn>() {
                    let burn = decoded.inner.data;
                    let amount = burn.amount as i128;
                    let key =
                        TickExtract::new(burn.tickLower, burn.tickUpper, decoded.inner.address);
                    *ticks_clone.entry(key).or_insert(0) -= amount;
                }
            });

            block_number = next_block;
            next_block += limit;
            latest_block = provider.get_block_number().await?
        }
    });

    log::info!("Rpc hits: {count}");

    let mut tick_data = TickData::new(latest_block, size);
    ticks.iter().for_each(|map| {
        let tick_extract = map.key();
        let amount = map.value();
        tick_data.insert(*tick_extract, *amount);
    });

    let mut file = File::create("resources/tick_map.json")?;
    file.write_all(serde_json::to_string_pretty(&tick_data)?.as_bytes())?;

    Ok(())
}
