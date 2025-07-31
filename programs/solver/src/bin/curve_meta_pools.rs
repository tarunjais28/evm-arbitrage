use alloy::{
    primitives::{address, Address, U256},
    providers::{
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        Identity, Provider, ProviderBuilder, RootProvider, WsConnect,
    },
    sol,
};
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

pub async fn get_pool_data(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pool: Address,
    n: usize,
) {
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

    println!("{rates:#?}");

    rates[n - 1] = base_virtual_price.to_big_int();
    println!("xi = {x:#?}");
    println!("precisions = {rates:#?}");

    let xp = vec![
        x[0].to_big_int() * rates[0] / precision,
        x[1].to_big_int() * rates[1] / precision,
    ];

    println!("xp: {xp:#?}");
    let s: BigInt = xp.iter().sum();
    let ann = a * U256::from(n);

    let d = get_d(ann, a_precision, s, n, xp.clone());
    println!("d_new: {}", d);

    let i = 0; // input index
    let j = 1; // output index
    let dx = BigInt::from(1000000000000000000000u128);
    let x = xp[i] + (dx * rates[i] / precision);

    let y = get_y(i, j, d, xp.clone(), x, ann, a_precision, n);

    let dy = xp[j] - y - BigInt::ONE;

    let _fee = fee.to_big_int() * dy / fee_denomination;

    let dy = (dy - _fee) * precision / rates[j];
    println!("_fee: {}", _fee);
    println!("y: {}", y);
    println!("dy: {}", dy);
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
    let mut count = 0;
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

        count = i + 1;
        if is_abs_le_1(&d, &d_prev) {
            break;
        }
    }
    println!("iterations: {count}");

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

    let mut count = 0;
    for _i in 0..255 {
        y_prev = y;
        y = (y * y + c) / (y * BigInt::from(2) + b - d);
        count = i + 1;

        if is_abs_le_1(&y_prev, &y) {
            break;
        }
    }
    println!("iterations: {count}");

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

    // 0x8038C01A0390a8c547446a0b2c18fc9aEFEcc10c`
    let pool = address!("0x3eF6A01A0f81D6046290f3e2A8c5b843e738E604");
    get_pool_data(&provider, pool, 2).await;
}
