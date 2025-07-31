use std::{fs::File, io::BufReader};

use alloy::{
    primitives::{address, Address, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use uniswap_sdk_core::prelude::*;
use utils::EnvParser;

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
    unspecified: Vec<Address>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PoolData {
    pool: Address,
    dy: i128,
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

async fn get_pool_data(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pools: Pools,
    n: usize,
) -> Output {
    let _dx = 1000000000000000000000u128;
    let mut out = Output {
        dx: _dx,
        data: Vec::with_capacity(pools.meta.len()),
    };
    for pool in pools.meta {
        let precision = BigInt::from(1000_000_000_000_000_000u128);
        let fee_denomination = BigInt::from(10_000_000_000u128);
        let contract = CurvePool::new(pool, provider.clone());
        let mut x = vec![U256::ZERO; n];
        let mut rates = Vec::with_capacity(n);
        let a;
        let a_precise;
        let base_virtual_price;
        let fee;
        let multicall = provider
            .multicall()
            .add(contract.A())
            .add(contract.A_precise())
            .add(contract.balances(U256::from(0)))
            .add(contract.balances(U256::from(1)))
            .add(contract.base_virtual_price())
            .add(contract.fee());

        (a, a_precise, x[0], x[1], base_virtual_price, fee) = multicall.aggregate().await.unwrap();

        let a_precision = (a_precise / a).to_big_int();

        let mut multicall = provider.multicall().dynamic();
        for i in 0..n {
            multicall = multicall.add_dynamic(contract.coins(U256::from(i)));
        }

        let coins = multicall.aggregate().await.unwrap();
        for coin in coins {
            let erc_20 = ERC20::new(coin, provider.clone());
            let p = erc_20.decimals().call().await.unwrap();
            rates.push(BigInt::from(10u128.pow(u32::from(36 - p))));
        }

        rates[n - 1] = base_virtual_price.to_big_int();

        let xp = vec![
            x[0].to_big_int() * rates[0] / precision,
            x[1].to_big_int() * rates[1] / precision,
        ];

        let s: BigInt = xp.iter().sum();
        let ann = a * U256::from(n);

        let d = get_d(ann, a_precision, s, n, xp.clone());

        let i = 0; // input index
        let j = 1; // output index
        let dx = BigInt::from(_dx);
        let x = xp[i] + (dx * rates[i] / precision);

        let y = get_y(i, j, d, xp.clone(), x, ann, a_precision, n);

        let dy = xp[j] - y - BigInt::ONE;

        let _fee = fee.to_big_int() * dy / fee_denomination;

        let dy = (dy - _fee) * precision / rates[j];
        out.data.push(PoolData {
            pool,
            dy: dy.to_string().parse().unwrap_or_default(),
        });
    }

    out
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
    for i in 0..255 {
        d_p = d;
        for _x in xp.clone() {
            // TODO: Handle divide by 0
            // If division by 0, this will be borked: only withdrawal will work. And that is good
            d_p *= d / (_x * n);
        }
        d_prev = d;
        d = ((ann * s / a_precision + (n * d_p)) * d)
            / ((ann - a_precision) * d / a_precision + n_1 * d_p);

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
        c *= d / (_x * BigInt::from(n));
    }

    c *= d * a_precision / (ann * n_big);
    let b = s_ + d * a_precision / ann;
    let mut y_prev;
    let mut y = d;

    for _i in 0..255 {
        y_prev = y;
        y = (y * y + c) / (y * BigInt::from(2) + b - d);

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

    let file = File::open("resources/curve_pools_splitted.json").unwrap();
    let reader = BufReader::new(file);

    // Parse and decode addresses
    let pools: Pools = from_reader(reader).unwrap();

    let output = get_pool_data(&provider, pools, 2).await;
}
