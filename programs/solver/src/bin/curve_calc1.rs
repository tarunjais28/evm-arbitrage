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
    let pres = BigInt::from(100000000000000000u128);
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
        precisions.push(10u64.pow(u32::from(p)));
    }

    let xp = vec![
        x[0].to_big_int() * pres / BigInt::from(precisions[0]),
        x[1].to_big_int() * pres / BigInt::from(precisions[1]),
        x[2].to_big_int() * pres / BigInt::from(precisions[2]),
    ];

    let s: BigInt = xp.iter().sum();
    let ann = a * U256::from(n);

    let d_new = get_d_new(ann, s, n, xp.clone());
    println!("d_new: {}", d_new);

    let i = 1;
    let j = 0;
    let amount_in = pres * BigInt::from(1000000000) / BigInt::from(precisions[i]);
    let fee = fee.to_big_int() * pres / BigInt::from(10000000000u64);
    let fract_one = BigInt::ONE;
    let amount = (fract_one - fee) * amount_in;

    let xi_ = amount + xp[i];
    let mut xp_ = xp.clone();
    xp_[i] = xi_;
    println!("xp_: {}", x[0].to_big_int() / pres);
    let mut s_j = BigInt::default();

    for k in 0..n {
        if k != j {
            s_j += xp[k];
        }
    }

    let y = get_y(i, j, d_new, xp_, xi_, ann, n);
    println!("{}", y);
}

fn get_d_new(ann: U256, s: BigInt, n: usize, xp: Vec<BigInt>) -> BigInt {
    let ann_org = ann;
    let ann = ann_org.to_big_int();
    let ann_1 = (ann_org - U256::ONE).to_big_int();
    let mut d = s;
    let n_org = n;
    let n = BigInt::from(n_org);
    let n_1 = BigInt::from(n_org + 1);

    if s.is_zero() {
        return BigInt::default();
    }

    let mut d_prev;
    let mut count = 0;
    let mut d_p;
    for i in 0..255 {
        d_p = d;
        for _x in xp.clone() {
            d_p = d_p * d / (_x * n_1);
        }
        println!("d_p = {}", d_p);
        d_prev = d;
        d = (((ann * s) + (n * d_p)) * d) / ((ann_1 * d) + (n_1 * d_p));

        count = i + 1;
        if is_abs_le_0(&d, &d_prev) {
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
    x_: BigInt,
    ann: U256,
    n: usize,
) -> BigInt {
    let mut c = d;
    let mut s_ = BigInt::default();
    let ann = ann.to_big_int();
    let mut _x = BigInt::default();
    let annn = ann * BigInt::from(n);

    for _i in 0..n {
        if _i == i {
            _x = x_;
        } else if _i != j {
            _x = xp_[_i];
        } else {
            continue;
        }
        s_ += _x;
        c = c * d / (_x * BigInt::from(n));
    }

    c = c * d / annn;
    let b = (s_ + d) / ann;
    let mut y_prev;
    let mut y = d;

    let mut count = 0;
    for _i in 0..255 {
        y_prev = y;
        y = ((y * y) + c) / ((y * BigInt::from(2)) + b - d);
        count = i + 1;

        if is_abs_le_0(&y_prev, &y) {
            break;
        }
    }
    println!("iterations: {count}");

    y
}

fn is_abs_le_0(n1: &BigInt, n2: &BigInt) -> bool {
    if n1 > n2 {
        if n1 - n2 <= BigInt::ONE {
            println!("{}", n1 - n2);
            return true;
        }
    } else {
        if n2 - n1 <= BigInt::ONE {
            println!("{}", n2 - n1);
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
