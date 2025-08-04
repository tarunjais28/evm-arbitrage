use alloy::{
    primitives::{Address, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use futures::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{
    fs::File,
    io::{BufReader, Write},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use uniswap_sdk_core::prelude::*;
use utils::EnvParser;

type SolverProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider,
>;

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    CurvePool,
    "../../resources/contracts/curve_meta_contract.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    ERC20,
    "../../resources/contracts/erc20_abi.json"
);

// source: https://github.com/curvefi/curve-contract/blob/master/contracts/pools/dusd/StableSwapDUSD.vy

#[derive(Debug, Deserialize, Serialize)]
struct Pools {
    meta: Vec<Address>,
    crypto: Vec<Address>,
    unspecified: Vec<Address>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PoolData {
    pool: Address,
    dy: i128,
    dy_exp: i128,
    diff: i128,
}

#[derive(Debug, Deserialize, Serialize)]
struct Output {
    dx: u128,
    data: Vec<PoolData>,
}

pub fn count_digits(mut n: BigInt) -> u32 {
    if n == BigInt::ZERO {
        return 1;
    }
    let mut count = 0;
    while n > BigInt::ZERO {
        count += 1;
        n /= BigInt::from(10);
    }
    count
}

async fn get_pool_data(provider: &SolverProvider, pools: Pools) -> Output {
    let _dx = 1000000000000000000000u128;

    let precision = BigInt::from(1000_000_000_000_000_000u128);
    let fee_denomination = BigInt::from(10_000_000_000u128);

    let i = 0; // input index
    let j = 1; // output index
    let out = Arc::new(Mutex::new(Output {
        dx: _dx,
        data: Vec::with_capacity(pools.crypto.len()),
    }));
    let provider = Arc::new(provider);
    let mut tasks = FuturesUnordered::new();
    for pool in pools.crypto {
        let provider = provider.clone();
        let out = out.clone();
        let mut n = 0;
        tasks.push(async move {
            let contract: CurvePool::CurvePoolInstance<Arc<&SolverProvider>> =
                CurvePool::new(pool, provider.clone());

            let mut x = Vec::new();
            while let Ok(bal) = contract.balances(U256::from(n)).call().await {
                x.push(bal);
                n += 1;
            }

            if n < 2 {
                eprintln!("pool: {pool} -> balances");
                return;
            }

            let mut rates = Vec::with_capacity(n);

            let mut multicall = provider.multicall().dynamic();
            for i in 0..n {
                multicall = multicall.add_dynamic(contract.coins(U256::from(i)));
            }

            let coins = multicall.aggregate().await.unwrap();
            for coin in coins {
                let erc_20 = ERC20::new(coin, provider.clone());
                let p = erc_20.decimals().call().await.unwrap();
                println!("decimals: {p}");
                rates.push(BigInt::from(10u128.pow(u32::from(36 - p))));
            }

            let mut xp = Vec::with_capacity(n);
            println!("xi: {x:#?}");
            for i in 0..n {
                xp.push(x[i].to_big_int() * rates[i] / precision);
            }
            println!("xp: {xp:#?}");

            let a = contract.A().call().await.unwrap();

            let a_precise = if let Ok(a_p) = contract.A_precise().call().await {
                a_p
            } else {
                eprintln!("pool: {pool} -> a_precise");
                return;
            };

            let dy_exp = contract
                .get_dy(i128::from(i as i8), i128::from(j as i8), U256::from(_dx))
                .call()
                .await
                .unwrap();

            let fee = contract.fee().call().await.unwrap();

            let a_precision = (a_precise / a).to_big_int();
            println!("a: {a}, a_precise: {a_precise}");
            let a = a_precise;

            let s: BigInt = xp.iter().sum();
            let ann = a * U256::from(n);

            let d = get_d(ann, a_precision, s, n, xp.clone());

            println!("d: {d}");

            let dx = BigInt::from(_dx);
            let x = xp[i] + ((dx * rates[i]) / precision);

            let y = get_y(i, j, d, xp.clone(), x, ann, a_precision, n);
            println!("y: {y}");

            let dy = xp[j] - y - BigInt::ONE;

            let _fee = (fee.to_big_int() * dy) / fee_denomination;

            let dy = ((dy - _fee) * precision) / rates[j];
            out.lock().unwrap().data.push(PoolData {
                pool,
                dy: dy.to_string().parse().unwrap_or_default(),
                dy_exp: dy_exp.to_string().parse().unwrap_or_default(),
                diff: dy.to_string().parse::<i128>().unwrap_or_default()
                    - dy_exp.to_string().parse::<i128>().unwrap_or_default(),
            });
        });
    }

    while let Some(_) = tasks.next().await {}

    Arc::try_unwrap(out).unwrap().into_inner().unwrap()
}

fn get_d(ann: U256, a_precision: BigInt, s: BigInt, n: usize, xp: Vec<BigInt>) -> BigInt {
    let ann = ann.to_big_int();
    let mut d = s;
    let n = BigInt::from(n);
    let n_1 = n + BigInt::ONE;

    if s.is_zero() {
        return BigInt::default();
    }

    let mut d_prev;
    let mut d_p;
    for _ in 0..255 {
        d_p = d;
        for _x in xp.clone() {
            // TODO: Handle divide by 0
            // If division by 0, this will be borked: only withdrawal will work. And that is good
            d_p = (d_p * d) / (_x * n);
        }
        d_prev = d;
        d = (((ann * s) / a_precision) + (d_p * n)) * d
            / ((((ann - a_precision) * d) / a_precision) + (n_1 * d_p));

        if is_abs_le_1(&d, &d_prev) {
            break;
        }
    }

    d
}

fn get_y(
    i: usize,
    j: usize,
    d: BigInt,
    xp_: Vec<BigInt>,
    x: BigInt,
    ann: U256,
    a_precision: BigInt,
    n: usize,
) -> BigInt {
    let mut c = d;
    let mut s_ = BigInt::default();
    let ann = ann.to_big_int();
    let mut _x = BigInt::default();
    let n_big = BigInt::from(n);

    for _i in 0..n {
        if _i == i {
            _x = x;
        } else if _i != j {
            _x = xp_[_i];
        } else {
            continue;
        }
        s_ += _x;
        c = (c * d) / (_x * n_big);
    }

    c = (c * d * a_precision) / (ann * n_big);
    let b = s_ + ((d * a_precision) / ann);
    let mut y = d;
    let mut y_prev;

    for _i in 0..255 {
        y_prev = y;
        y = ((y * y) + c) / ((y * BigInt::from(2)) + b - d);

        if is_abs_le_1(&y_prev, &y) {
            break;
        }
    }

    y
}

fn is_abs_le_1(n1: &BigInt, n2: &BigInt) -> bool {
    if n1 > n2 {
        if n1 - n2 <= BigInt::ONE {
            return true;
        }
    } else {
        if n2 - n1 <= BigInt::ONE {
            return true;
        }
    }

    false
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

    // let file = File::open("resources/curve_pools_splitted.json").unwrap();
    // let reader = BufReader::new(file);

    // // Parse and decode addresses
    // let pools: Pools = from_reader(reader).unwrap();

    // let meta = pools.meta.len();
    let pools = Pools {
        meta: Vec::default(),
        crypto: vec![alloy::primitives::address!(
            "0xa4c567c662349BeC3D0fB94C4e7f85bA95E208e4"
        )],
        unspecified: Vec::default(),
    };
    let output = get_pool_data(&provider, pools).await;
    // println!("Processed {} / {}", output.data.len(), meta);

    println!("output: {output:#?}");
    // println!("{output:#?}");
    // let mut file = File::create("test-beds/curve_meta_pool_dy.json").unwrap();
    // file.write_all(serde_json::to_string_pretty(&output).unwrap().as_bytes())
    //     .unwrap();
}
