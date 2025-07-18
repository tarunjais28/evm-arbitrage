use alloy::{
    primitives::{address, aliases::I24, Address, U160, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use futures::{stream, StreamExt};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
/// Step 1:  Calculate Words inside a pool: Max tickrange: +-887272. Divide
/// 887272 by tick spacing for the pool. Divide again by 256 to get the number
/// of words on either side of 0. To calculate WordIndex Range, we use the
/// following formula: lowest word index = -words max. -words max is basically the
/// lowest rounded word index. For example, if 887272/60 =14,787.867. and then
/// 14787.867/256=57.765. The lowest word index will be 58. and the highest word
/// index will be +words max - 1. So in our example, 57.765 - 1 =56.765 and this
/// rounded up gives up 57. And so the total words to scan will be (57+58) + 1 =116.
///
/// Step 2:  Do one multicall tick_bitmap (wordindex) for receiving to get the word
/// encoded tickbitmap. So we will use one .add_call for with tickbitmap(wordindex).
/// Can use multicall3 or any other.
///
/// Step 3: For each 256-bit response will tell you which of the words tick slots
/// are initialized. For each bit, we derive one tick index using the formula
/// tickindex = (wordindex x 256 + bitpos) * tickspacing. Here, Bitpos rotates from
/// 0 ->255 i.e. we have to run 256 loops to check if each bit in that wordindex is
/// active or not. and we use the formula to get the tickindex for active ticks.
/// However, we will apply a hack to this. We will use the trailing 0 operation to
/// identify the first 1 bit (active tick) starting from the rightmost side (lowest bit).
/// Using trailing 0 we will know how many 0 are to the right and so we know the exact
/// bitpos of the 1 (active tick). We then find the tickindex of this tick using the
/// formula above and then we turn the bit off or clear the bit using bits = bits &
/// (bits - 1) and we then use trailing 0 to look for the next 1 in the 256 bitmap.
///
/// Step 4: For each tick index, we will call for liquiditynet  using
/// pools.ticks(tickindex) - we can use multicall for this as well so that its 1RPC
/// for all active ticks. We then the prefix sum the liquidity net in order to get
/// the curve of the pool.
use std::{fs::File, io::BufReader};
use uniswap_sdk_core::{prelude::*, token};
use uniswap_v3_sdk::prelude::*;
use utils::{debug_time, EnvParser};

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

async fn get_pool_data_old<'a>(
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
    let liquidity = contract.liquidity().call().await.unwrap();

    // Compute wordPosition = tick / 256
    // Convert Signed<24,1> to i32 using to_i32() method, then divide and cast to i16
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TickDetails {
    block: u64,
    pool: Address,
    ticks: Vec<Tick>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tick {
    pub index: i32,
    pub liquidity_gross: u128,
    pub liquidity_net: i128,
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
    let contract = IUniswapV3Pool::new(pools, provider.clone());
    let slot0 = contract.slot0();
    let tick_spacing = contract.tickSpacing();
    let liquidity = contract.liquidity();

    let multicall = provider
        .multicall()
        .add(slot0)
        .add(tick_spacing)
        .add(liquidity);
    let (slot0, tick_spacing, liquidity) = multicall.aggregate().await.unwrap();
    let sqrt_price_x96 = slot0.sqrtPriceX96;

    // Compute wordPosition = tick / 256
    // Convert Signed<24,1> to i32 using to_i32() method, then divide and cast to i16
    let mut tick_indices = Vec::new();
    let min_word: i16 = (MIN_TICK_I32 / 256) as i16 / tick_spacing.as_i16();
    let max_word: i16 = (MAX_TICK_I32 / 256) as i16 / tick_spacing.as_i16();
    let mut count = 3;
    debug_time!("Calling scanner()", {
        for word_position in min_word..=max_word {
            // let bitmap = contract.tickBitmap(word_position);
            // multicall.with_cloned_provider().add(bitmap);
            let bitmap = contract.tickBitmap(word_position).call().await.unwrap();
            count += 1;

            // Create a vector to store tick indices for this word position
            let mut current_tick_indices = Vec::new();
            let mut multicall = provider.multicall().dynamic();
            let mut has_ticks = false;

            for bit_pos in 0..256 {
                if bitmap.bit(bit_pos) {
                    let tick_index = (word_position as i32) * 256 + bit_pos as i32;
                    let tick_info = contract.ticks(I24::try_from(tick_index).unwrap());
                    multicall = multicall.add_dynamic(tick_info);
                    count += 1;
                    current_tick_indices.push(tick_index);
                    has_ticks = true;
                }
            }

            // Only execute multicall if we found any ticks in this word position
            if has_ticks {
                let ticks = multicall.aggregate3().await.unwrap();
                for idx in 0..ticks.len() {
                    if let Ok(tick) = ticks[idx].clone() {
                        let tick_data = Tick {
                            index: current_tick_indices[idx],
                            liquidity_gross: tick.liquidityGross,
                            liquidity_net: tick.liquidityNet,
                        };
                        tick_indices.push(tick_data);
                    }
                }
            }
        }
    });

    println!("{:#?}", tick_indices);
    println!("{}", tick_indices.len());
    println!("rpc hit {count}",);

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

    let pool_addr = address!("0x5777d92f208679db4b9778590fa3cab3ac9e2168");
    let (sqrt_ratio_x96, liquidity) = get_pool_data(provider.clone(), pool_addr).await;
    // fetch_all_initialized_ticks(&provider, &vec![pool_addr])
    //     .await
    //     .unwrap();

    // let file = File::open("resources/pools_v3.json").unwrap();
    // let reader = BufReader::new(file);

    // // Parse and decode addresses
    // let pools: Vec<Address> = from_reader(reader).unwrap();

    // debug_time!("fetching ticks", {
    //     fetch_all_initialized_ticks(&provider, &vec![pool_addr])
    //         .await
    //         .unwrap()
    // });
    // println!("{:#?}", ticks);
    // println!("{}", ticks.len());
}
