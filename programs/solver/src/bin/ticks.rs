//! Example demonstrating pool creation with tick data provider and swap simulation
//!
//! # Prerequisites
//! - Environment variable MAINNET_RPC_URL must be set
//! - Requires the "extensions" feature
//!
//! # Note
//! This example uses mainnet block 17000000 for consistent results
use std::{fs::File, io::BufReader};

use futures::{stream, StreamExt};
use serde_json::from_reader;
use uniswap_sdk_core::{prelude::*, token};
use uniswap_v3_sdk::prelude::*;
use utils::EnvParser;

use alloy::{
    primitives::{address, aliases::I24, aliases::U24, Address, U160, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};

/// Computes price = (sqrtPriceX96)^2 / 2^192
/// Returns price as an f64 for readability.
///
/// sqrtPriceX96 is the Q96 fixed-point square root price.
pub fn price_from_sqrt_price_x96(sqrt_price_x96: U256) -> (U256, U256) {
    // Numerator: (sqrtPriceX96)^2
    let numerator = sqrt_price_x96 * sqrt_price_x96;

    // Denominator: 2^192 = 1 << 192
    let denominator = U256::ONE << 192;

    let price = numerator.checked_div(denominator).unwrap_or_default();
    let rec = U256::ONE.checked_div(price).unwrap_or_default();
    (price, rec)
}

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV3Pool,
    "../../resources/contracts/uniswapv3_pool_abi.json"
);

macro_rules! debug_time {
    ($label:expr, $block:block) => {{
        use std::time::Instant;
        let start = Instant::now();
        let result = $block;
        log::debug!("{} took {:?}", $label, start.elapsed());
        result
    }};
}

async fn get_pool_data<'a>(
    provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pools: Address,
) -> (U160, u128) {
    let contract = IUniswapV3Pool::new(pools, provider);
    let slot0 = contract.slot0().call().await.unwrap();
    let sqrt_price_x96 = slot0.sqrtPriceX96;
    let tick = slot0.tick;
    let liquidity = contract.liquidity().call().await.unwrap();

    // Compute wordPosition = tick / 256
    // Convert Signed<24,1> to i32 using to_i32() method, then divide and cast to i16
    let word_position = ((tick.as_i32()) / 256) as i16;
    let bitmap = contract.tickBitmap(word_position).call().await.unwrap();
    let mut tick_indices = Vec::new();
    debug_time!("Calling scanner()", {
        for word_position in ((MIN_TICK_I32 / 256) as i16)..=((MAX_TICK_I32 / 256) as i16) {
            let bitmap = contract.tickBitmap(word_position).call().await.unwrap();
            for bit_pos in 0..256 {
                if bitmap.bit(bit_pos) {
                    let tick_index = (word_position as i32) * 256 + bit_pos as i32;

                    // Now fetch tick info
                    let tick_info = contract
                        .ticks(I24::try_from(tick_index).unwrap())
                        .call()
                        .await
                        .unwrap();
                    tick_indices.push(tick_index);
                    println!(
                        "Initialized tick {}: liquidityGross = {}",
                        tick_index, tick_info.liquidityGross
                    );
                }
            }
        }
    });
    tick_indices.sort();
    println!("{:#?}", tick_indices);
    println!("{}", tick_indices.len());

    (sqrt_price_x96, liquidity)
}

const MIN_WORD: i16 = (-887272 / 256) as i16;
const MAX_WORD: i16 = (887272 / 256) as i16;
const MAX_CONCURRENT_BITMAP_CALLS: usize = 100;
const MAX_CONCURRENT_TICK_CALLS: usize = 100;
const MAX_CONCURRENT_POOLS: usize = 5;

async fn fetch_all_initialized_ticks(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pools: &[Address],
) -> anyhow::Result<()> {
    println!("{}", pools.len());
    stream::iter(pools.iter())
        .map(|pool| {
            let provider = provider.clone();
            let p = *pool;
            async move {
                let contract = IUniswapV3Pool::new(p, provider);

                // 1️⃣ Optionally use slot0 to narrow scanning
                // let slot0 = contract.slot0().call().await?;
                // let current_tick = slot0.tick.as_i32();
                // let current_word = (current_tick / 256) as i16;
                // let word_positions: Vec<i16> = (current_word - 1..=current_word + 1).collect();

                // If you want full scan:
                let word_positions: Vec<i16> = (MIN_WORD..=MAX_WORD).collect();

                // 2️⃣ Fetch bitmaps concurrently
                let bitmaps = stream::iter(word_positions)
                    .map(|word_pos| {
                        let c = contract.clone();
                        async move {
                            let bitmap = c.tickBitmap(word_pos).call().await?;
                            Ok::<_, anyhow::Error>((word_pos, bitmap))
                        }
                    })
                    .buffer_unordered(MAX_CONCURRENT_BITMAP_CALLS)
                    .filter_map(|res| async move { res.ok() })
                    .collect::<Vec<_>>()
                    .await;

                // 3️⃣ Build tick futures
                let mut tick_indices = Vec::with_capacity(2048);
                for (word_pos, bitmap) in &bitmaps {
                    if bitmap.is_zero() {
                        continue;
                    }
                    for bit_pos in 0..256 {
                        if bitmap.bit(bit_pos) {
                            let tick_idx = (*word_pos as i32) * 256 + bit_pos as i32;
                            tick_indices.push(tick_idx);
                        }
                    }
                }

                println!(
                    "Pool {:?}: discovered {} initialized ticks",
                    p,
                    tick_indices.len()
                );

                // 4️⃣ Fetch tick data concurrently
                let tick_infos = stream::iter(tick_indices.into_iter())
                    .map(|tick_idx| {
                        let c = contract.clone();
                        async move {
                            let tick_i24 = I24::try_from(tick_idx).unwrap();
                            let info = c.ticks(tick_i24).call().await?;
                            Ok::<_, anyhow::Error>((tick_idx, info))
                        }
                    })
                    .buffer_unordered(MAX_CONCURRENT_TICK_CALLS)
                    .filter_map(|res| async move { res.ok() })
                    .collect::<Vec<_>>()
                    .await;

                for (tick_idx, tick_info) in &tick_infos {
                    println!(
                        "Pool {} tick {}: liquidityGross = {:#?}",
                        p, tick_idx, tick_info
                    );
                }

                Ok::<(), anyhow::Error>(())
            }
        })
        .buffer_unordered(MAX_CONCURRENT_POOLS)
        .collect::<Vec<_>>()
        .await;

    Ok(())
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

    const CHAIN_ID: u64 = 1;
    let link = token!(
        CHAIN_ID,
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
        18,
        "LINK",
        "ChainLink Token"
    );

    let usdt = token!(
        CHAIN_ID,
        "0xdac17f958d2ee523a2206206994597c13d831ec7",
        6,
        "LINK",
        "ChainLink Token"
    );

    let weth = WETH9::on_chain(CHAIN_ID).unwrap();

    let pool_addr = address!("0x477e1a178f308fb8c2967d3e56e157c4b8b6f5df");
    // let (sqrt_ratio_x96, liquidity) = get_pool_data(provider.clone(), pool_addr).await;

    let file = File::open("resources/pools_v3.json").unwrap();
    let reader = BufReader::new(file);

    // Parse and decode addresses
    let pools: Vec<Address> = from_reader(reader).unwrap();

    debug_time!("fetching ticks", {
        fetch_all_initialized_ticks(&provider, &vec![pool_addr])
            .await
            .unwrap()
    });
    // println!("{:#?}", ticks);
    // println!("{}", ticks.len());
}
