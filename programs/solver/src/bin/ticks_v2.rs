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
use alloy::{
    primitives::{aliases::I24, Address, U160, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use futures::future::join_all;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{
    fs::File,
    io::{BufReader, Write},
};
use tokio::task::spawn_blocking;
use uniswap_sdk_core::prelude::*;
use uniswap_v3_sdk::prelude::{tick_sync::TickSync, *};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TickDetails {
    block: u64,
    pool: Address,
    tick_spacing: i32,
    sqrt_price_x96: U160,
    liquidity: u128,
    ticks: Vec<TickSync>,
}

const MIN_WORD: i16 = (MIN_TICK_I32 / 256) as i16;
const MAX_WORD: i16 = (MAX_TICK_I32 / 256) as i16;

pub async fn get_pool_data<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pools: &[Address],
) -> Vec<TickDetails> {
    let provider = provider.clone();
    let pools = pools.to_vec();

    // Run entire Rayon logic inside a blocking thread using Tokio
    let results: Vec<TickDetails> = spawn_blocking(move || {
        pools
            .par_iter()
            .map(|pool| {
                let pool = *pool;
                let provider = provider.clone();

                // Create a local runtime inside each rayon thread
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    let contract = IUniswapV3Pool::new(pool, provider.clone());
                    let slot0 = contract.slot0();
                    let tick_spacing = contract.tickSpacing();
                    let liquidity = contract.liquidity();

                    let multicall = provider
                        .multicall()
                        .add(slot0)
                        .add(tick_spacing)
                        .add(liquidity);

                    let (slot0, tick_spacing, liquidity) = multicall.aggregate().await.unwrap();
                    let min_word: i16 = MIN_WORD / tick_spacing.as_i16();
                    let max_word: i16 = MAX_WORD / tick_spacing.as_i16();

                    let word_futures: Vec<_> = (min_word..=max_word)
                        .map(|word_position| {
                            let contract = contract.clone();
                            let provider = provider.clone();

                            async move {
                                let bitmap = match contract.tickBitmap(word_position).call().await {
                                    Ok(b) => b,
                                    Err(_) => return vec![],
                                };

                                let mut current_tick_indices = Vec::new();
                                let mut multicall = provider.multicall().dynamic();

                                for bit_pos in 0..256 {
                                    if bitmap.bit(bit_pos) {
                                        let tick_index =
                                            (word_position as i32) * 256 + bit_pos as i32;
                                        if let Ok(tick_index_i24) = I24::try_from(tick_index) {
                                            multicall = multicall
                                                .add_dynamic(contract.ticks(tick_index_i24));
                                            current_tick_indices.push(tick_index);
                                        }
                                    }
                                }

                                let mut ticks_data = Vec::new();
                                if !current_tick_indices.is_empty() {
                                    if let Ok(ticks) = multicall.aggregate3().await {
                                        for (idx, tick_result) in ticks.into_iter().enumerate() {
                                            if let Ok(tick) = tick_result {
                                                ticks_data.push(TickSync {
                                                    index: current_tick_indices[idx],
                                                    liquidity_gross: tick.liquidityGross,
                                                    liquidity_net: tick.liquidityNet,
                                                    is_init: tick.initialized,
                                                });
                                            }
                                        }
                                    }
                                }

                                ticks_data
                            }
                        })
                        .collect();

                    let mut all_ticks: Vec<TickSync> =
                        join_all(word_futures).await.into_iter().flatten().collect();

                    all_ticks.sort_by(|t1, t2| t1.index.cmp(&t2.index));
                    log::info!("Pool {pool} processed!");

                    TickDetails {
                        block: provider.get_block_number().await.unwrap(),
                        pool,
                        sqrt_price_x96: slot0.sqrtPriceX96,
                        liquidity,
                        ticks: all_ticks,
                        tick_spacing: tick_spacing.as_i32(),
                    }
                })
            })
            .collect()
    })
    .await
    .unwrap();

    results
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

    let file = File::open("resources/pools_v3.json").unwrap();
    let reader = BufReader::new(file);
    let pools: Vec<Address> = from_reader(reader).unwrap();

    let tick_data = debug_time!("tick map:", { get_pool_data(&provider, &pools).await });

    let mut file = File::create("resources/ticks.json").unwrap();
    file.write_all(serde_json::to_string_pretty(&tick_data).unwrap().as_bytes())
        .unwrap();
}

// 20118
