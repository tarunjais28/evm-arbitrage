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
    "../../resources/contracts/curve_crypto_contract.json"
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

async fn get_pool_data(provider: &SolverProvider, pools: Pools) -> Output {
    let _dx = 1000000000000000000000u128;

    let precisions: Vec<BigInt> = Vec::new();
    let precision = BigInt::from(10u128.pow(18));
    let fee_denomination = BigInt::from(10u128.pow(10));

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
        let mut precisions = precisions.clone();
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
                precisions.push(BigInt::from(10u128.pow(u32::from(18 - p))));
            }

            let _price_scale = contract.price_scale().call().await.unwrap().to_big_int();
            let price_scale = _price_scale * precisions[1];

            let mut xp = x;
            println!("xp: {xp:#?}");

            let a = contract.A().call().await.unwrap().to_big_int();
            let gamma = contract.gamma().call().await.unwrap().to_big_int();
            let mut d = contract.D().call().await.unwrap().to_big_int();
            let future_a_gamma_time = contract.future_A_gamma_time().call().await.unwrap();
            let fee_gamma = contract.fee_gamma().call().await.unwrap().to_big_int();
            let mid_fee = contract.mid_fee().call().await.unwrap().to_big_int();
            let out_fee = contract.out_fee().call().await.unwrap().to_big_int();

            let _xp = vec![
                xp[0].to_big_int() * precisions[0],
                (xp[1].to_big_int() * precisions[1] * _price_scale) / precision,
            ];

            if future_a_gamma_time > U256::ZERO {
                d = newton_d(a, gamma, n, _xp);
            }

            let dx = U256::from(_dx);
            xp[i] += dx;
            let mut xp = vec![
                xp[0].to_big_int() * precisions[0],
                (xp[i].to_big_int() * price_scale) / precision,
            ];

            let dy_exp = contract
                .get_dy(U256::from(i), U256::from(j), U256::from(_dx))
                .call()
                .await
                .unwrap();

            let y = newton_y(a, gamma, xp.clone(), d, j, n);
            println!("y: {y}");

            let mut dy = xp[j] - y - BigInt::ONE;
            xp[j] = y;

            dy = if j > 0 {
                (dy * precision) / price_scale
            } else {
                dy / precisions[0]
            };

            let fee = _fee(xp, fee_gamma, mid_fee, out_fee);
            dy -= (fee * dy) / fee_denomination;
            
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

fn _fee(xp: Vec<BigInt>, fee_gamma: BigInt, mid_fee: BigInt, out_fee: BigInt) -> BigInt {
    let n = xp.len() as u32;
    let n_n = BigInt::from(n.pow(n));
    let precision = BigInt::from(10u128.pow(18));
    let mut f = xp.iter().sum::<BigInt>();
    f = (fee_gamma * precision)
        / ((fee_gamma + precision) - (((precision * n_n * xp[0]) / (f * xp[1])) / f));

    ((mid_fee * f) + (out_fee * (precision - f))) / precision
}

fn geometric_mean(unsorted_x: Vec<BigInt>, sort: bool, n: BigInt) -> BigInt {
    let precision = BigInt::from(10u128.pow(18));
    let mut x = unsorted_x;
    if sort {
        x.sort_by(|a, b| b.cmp(a));
    }

    let mut d = x[0];
    let mut d_prev;
    let mut diff;
    for _ in 0..255 {
        d_prev = d;
        d = (d + ((x[0] * x[1]) / d)) / n;

        if d > d_prev {
            diff = d - d_prev;
        } else {
            diff = d_prev - d
        }

        if diff <= BigInt::ONE || (diff * precision) < d {
            return d;
        }
    }

    d
}

fn newton_d(ann: BigInt, gamma: BigInt, n: usize, x_unsorted: Vec<BigInt>) -> BigInt {
    let mut x = x_unsorted;
    x.sort_by(|a, b| b.cmp(a));

    let n_big = BigInt::from(n);
    let ten_14 = BigInt::from(10u128.pow(14));
    let ten_16 = BigInt::from(10u128.pow(16));
    let ten_18 = BigInt::from(10u128.pow(18));
    let ten_20 = BigInt::from(10u128.pow(20));
    let a_multiplier = BigInt::from(1000);

    let mut d = n_big * geometric_mean(x.clone(), false, n_big);
    let s = x.iter().sum::<BigInt>();
    let __g1k0 = gamma + ten_18;

    let mut _g1k0;
    let mut d_prev;
    let mut k0;
    for _ in 0..255 {
        d_prev = d;

        k0 = ((ten_18 * n_big * n_big * x[0]) / d) * x[1] / d;

        _g1k0 = __g1k0;

        if _g1k0 > k0 {
            _g1k0 -= k0 + BigInt::ONE;
        } else {
            _g1k0 = k0 - _g1k0 + BigInt::ONE;
        }

        let mul1 = (((((ten_18 * d) / gamma) * _g1k0) / gamma) * _g1k0 * a_multiplier) / ann;
        let mul2 = (BigInt::TWO * ten_18 * n_big * k0) / _g1k0;
        let neg_fprime =
            (s + ((s * mul2) / ten_18)) + ((mul1 * n_big) / k0) - ((mul2 * d) / ten_18);

        let d_plus = (d * (neg_fprime + s)) / neg_fprime;
        let mut d_minus = (d * d) / neg_fprime;

        if ten_18 > k0 {
            d_minus += ((d * (mul1 / neg_fprime)) / ten_18) * ((ten_18 - k0) / k0);
        } else {
            d_minus -= ((d * (mul1 / neg_fprime)) / ten_18) * ((k0 - ten_18) / k0);
        }

        if d_plus > d_minus {
            d = d_plus - d_minus;
        } else {
            d = (d_minus - d_plus) / BigInt::TWO;
        }

        let diff;
        if d > d_prev {
            diff = d - d_prev;
        } else {
            diff = d_prev - d;
        }

        if diff * ten_14 < BigInt::max(ten_16, d) {
            for _x in x {
                let frac = (_x * ten_18) / d;
                if frac <= ten_16 - BigInt::ONE || frac >= ten_20 + BigInt::ONE {
                    return BigInt::ZERO;
                }
            }
            return d;
        }
    }

    BigInt::ZERO
}

fn newton_y(ann: BigInt, gamma: BigInt, x: Vec<BigInt>, d: BigInt, i: usize, n: usize) -> BigInt {
    let ten_14 = BigInt::from(10u128.pow(14));
    let ten_16 = BigInt::from(10u128.pow(16));
    let ten_18 = BigInt::from(10u128.pow(18));
    let ten_20 = BigInt::from(10u128.pow(20));
    let a_multiplier = BigInt::from(1000);

    let n = BigInt::from(n);
    let x_j = x[1 - i];
    let mut y = (d * d) / (x_j * n * n);
    let k0_i = (ten_18 * n * x_j) / d;

    let convergence_limit = BigInt::max(BigInt::max(x_j / ten_14, d / ten_14), BigInt::from(100));

    let __g1k0 = gamma + ten_18;

    let mut y_prev;
    let mut k0;
    let mut s;
    let mut _g1k0;
    let mut mul1;
    let mut mul2;
    let mut fprime;
    let mut yfprime;
    let mut _dyfprime;
    let mut y_minus;
    let mut y_plus;
    let mut diff;
    let frac;
    for _ in 0..255 {
        y_prev = y;

        k0 = (k0_i * y * n) / d;
        s = x_j + y;

        _g1k0 = __g1k0;
        if _g1k0 > k0 {
            _g1k0 -= k0 + BigInt::ONE;
        } else {
            _g1k0 = k0 - _g1k0 + BigInt::ONE;
        }

        mul1 = (((((ten_18 * d) / gamma) * _g1k0) / gamma) * _g1k0 * a_multiplier) / ann;

        mul2 = (BigInt::TWO * ten_18 * n * k0) / _g1k0;

        yfprime = (ten_18 * y) + (s * mul2) + mul1;
        _dyfprime = d * mul2;
        if yfprime < _dyfprime {
            y = y_prev / BigInt::TWO;
            continue;
        } else {
            yfprime -= yfprime;
        }
        fprime = yfprime / y;

        y_minus = mul1 / fprime;
        y_plus = (yfprime + (ten_18 * d)) / fprime + ((y_minus * ten_18) / k0);
        y_minus += (ten_18 * s) / fprime;

        if y_plus < y_minus {
            y = y_prev / BigInt::TWO;
        } else {
            y = y_plus - y_minus;
        }

        if y > y_prev {
            diff = y - y_prev;
        } else {
            diff = y_prev - y;
        }
        if diff < BigInt::max(convergence_limit, y / ten_14) {
            frac = (y * ten_18) / d;
            if frac <= ten_16 - BigInt::ONE || frac >= ten_20 + BigInt::ONE {
                return BigInt::ZERO;
            }
            return d;
        }
    }

    BigInt::ZERO
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
            "0xB576491F1E6e5E62f1d8F26062Ee822B40B0E0d4"
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
