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
    "../../resources/contracts/curve_pool.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    ERC20,
    "../../resources/contracts/erc20_abi.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    CurveContract,
    "../../resources/contracts/curve_contract.json"
);

// source: https://github.com/curvefi/curve-contract/blob/master/contracts/pools/3pool/StableSwap3Pool.vy

fn count_digits(mut n: BigInt) -> u32 {
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
    let mut x = Vec::with_capacity(n);
    let mut precisions = Vec::with_capacity(n);
    (0..n).for_each(|_| {
        x.push(U256::ZERO);
    });
    let a;
    let fee;
    let multicall = provider
        .multicall()
        .add(contract.A())
        .add(contract.balances(U256::from(0)))
        .add(contract.balances(U256::from(1)))
        .add(contract.balances(U256::from(2)))
        .add(contract.fee());

    (a, x[0], x[1], x[2], fee) = multicall.aggregate().await.unwrap();

    let mut multicall = provider.multicall().dynamic();
    for i in 0..n {
        multicall = multicall.add_dynamic(contract.coins(U256::from(i)));
    }

    let coins = multicall.aggregate().await.unwrap();
    for coin in coins {
        let erc_20 = ERC20::new(coin, provider.clone());
        let p = erc_20.decimals().call().await.unwrap();
        precisions.push(BigInt::from(10u64.pow(u32::from(p))));
    }

    let curve_lp_token = address!("0x6c3F90f043a72FA612cbac8115EE7e52BDe6E490");
    let curve_contract = CurveContract::new(curve_lp_token, provider.clone());
    let total_supply = curve_contract.totalSupply().call().await.unwrap();
    let virtual_price = contract.get_virtual_price().call().await.unwrap();

    println!("xi = {x:#?}");

    let xp = vec![
        x[0].to_big_int() * precision / precisions[0],
        x[1].to_big_int() * precision / precisions[1],
        x[2].to_big_int() * precision / precisions[2],
    ];

    println!("xp: {xp:#?}");
    let s: BigInt = xp.iter().sum();
    let ann = a * U256::from(n);

    let d = get_d_new(ann, s, n, xp.clone());
    println!("d_new: {}", d);

    let d_exp = virtual_price.to_big_int() * total_supply.to_big_int() / precision;
    println!("d_exp: {d_exp}");

    println!("diff: {}", d - d_exp);
    let i = 1; // input index
    let j = 0; // output index
    let dx = BigInt::from(1000);
    let x = xp[i] + (dx * precision / precisions[i]);

    let y = get_y(i, j, d, xp.clone(), x, ann, n);

    let dy = (xp[j] - y - BigInt::ONE) * precisions[j] /  precision;

    let _fee = fee.to_big_int() * dy / fee_denomination;

    println!("_fee: {}", _fee);
    println!("y: {}", y);
    println!("dy: {}", dy - _fee);

    let e_dy = BigInt::from(999844592181965u128);
    println!("digits: {}", count_digits(e_dy));
    let e_y = xp[j] - e_dy;
    println!("{} - {e_dy} = {e_y}", xp[j]);
}

fn get_d_new(ann: U256, s: BigInt, n: usize, xp: Vec<BigInt>) -> BigInt {
    let ann = ann.to_big_int();
    let ann_1 = ann - BigInt::ONE;
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
            d_p = d_p * d / (_x * n);
        }
        d_prev = d;
        d = (((ann * s) + (n * d_p)) * d) / ((ann_1 * d) + (n_1 * d_p));

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
        c = c * d / (_x * BigInt::from(n));
    }

    c = c * d / (ann * n_big);
    let b = s_ + d / ann;
    let mut y_prev;
    let mut y = d;

    let mut count = 0;
    for _i in 0..255 {
        y_prev = y;
        y = ((y * y) + c) / ((y * BigInt::from(2)) + b - d);
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

    // let file = File::open("resources/pools_v3.json").unwrap();
    // let reader = BufReader::new(file);
    // let pools: Vec<Address> = from_reader(reader).unwrap();

    let pool = address!("0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7");
    get_pool_data(&provider, pool, 3).await;

    // let mut file = File::create("resources/ticks.json").unwrap();
    // file.write_all(serde_json::to_string_pretty(&tick_data).unwrap().as_bytes())
    //     .unwrap();
}
