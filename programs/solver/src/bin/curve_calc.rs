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

pub async fn get_pool_data<'a>(
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
    let contract = CurvePool::new(pool, provider.clone());
    let mut x = Vec::with_capacity(n);
    let mut precisions = Vec::with_capacity(n);
    (0..n).into_iter().for_each(|_| {
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
        Fraction::new(x[0].to_big_int(), BigInt::from(precisions[0])),
        Fraction::new(x[1].to_big_int(), BigInt::from(precisions[1])),
        Fraction::new(x[2].to_big_int(), BigInt::from(precisions[2])),
    ];

    let s: Fraction = xp[0].clone() + xp[1].clone() + xp[2].clone();
    let ann = a * U256::from(n);

    // let dp = get_dp(s.clone(), n, xp.clone());
    let d_new = get_d_new(ann, s, n, xp.clone());

    let i = 1;
    let j = 0;
    let amount_in = Fraction::new(1000, BigInt::from(precisions[i]));
    let fee = Fraction::new(fee.to_big_int(), BigInt::from(10000000000u64));
    let fract_one = Fraction::new(1, 1);
    let amount = (fract_one - fee) * amount_in;

    let xi_ = amount + xp[i].clone();
    let mut xp_ = xp.clone();
    xp_[i] = xi_.clone();
    let mut s_j = Fraction::default();

    for k in 0..n {
        if k != j {
            s_j = s_j + xp[k].clone();
        }
    }

    let y = get_y(i, j, d_new, xp_, xi_, ann, n);
    // println!("{}", y.quotient());
    let precision = Fraction::new(precisions[j], 1);
    let amount_out = y * precision;
    println!("{}", amount_out.quotient());
}

fn get_d_new(ann: U256, s: Fraction, n: usize, xp: Vec<Fraction>) -> Fraction {
    let ann_org = ann;
    let ann = Fraction::new(ann_org.to_big_int(), 1);
    let ann_1 = Fraction::new((ann_org - U256::ONE).to_big_int(), 1);
    let mut d = s.clone();
    let n_org = n;
    let n = Fraction::new(BigInt::from(n_org), 1);
    let n_1 = Fraction::new(BigInt::from(n_org + 1), 1);

    if s.numerator().is_zero() {
        return Fraction::default();
    }

    let mut d_prev;
    let mut count = 0;
    let mut d_p;
    for i in 0..255 {
        d_p = d.clone();
        for _x in xp.clone() {
            d_p = d_p * d.clone() / (_x * n_1.clone());
        }
        println!("d_p = {}", d_p.quotient());
        d_prev = d.clone();
        d = (((ann.clone() * s.clone()) + (n.clone() * d_p.clone())) * d.clone())
            / ((ann_1.clone() * d.clone()) + (n_1.clone() * d_p.clone()));

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
    d: Fraction,
    xp_: Vec<Fraction>,
    x_: Fraction,
    ann: U256,
    n: usize,
) -> Fraction {
    let mut c = d.clone();
    let mut s_ = Fraction::default();
    let ann = Fraction::new(ann.to_big_int(), 1);
    let mut _x = Fraction::default();
    let annn = Fraction::new(ann.numerator() * BigInt::from(n), ann.denominator());

    for _i in 0..n {
        if _i == i {
            _x = x_.clone();
        } else if _i != j {
            _x = xp_[_i].clone();
        } else {
            continue;
        }
        s_ = s_ + _x;
        c = c * d.clone() / (ann.clone() * Fraction::new(BigInt::from(n), 1));
    }

    c = c * d.clone() / annn;
    let b = (s_ + d.clone()) / ann;
    let mut y_prev;
    let mut y = d.clone();

    let mut count = 0;
    for _i in 0..255 {
        y_prev = y.clone();
        y = (y.clone() * y.clone() + c.clone())
            / (y * Fraction::new(BigInt::from(2), 1) + b.clone() - d.clone());
        count = i + 1;

        if is_abs_le_0(&y_prev, &y) {
            break;
        }
    }
    println!("iterations: {count}");

    y
}

fn is_abs_le_0(n1: &Fraction, n2: &Fraction) -> bool {
    if n1 > n2 {
        if n1.quotient() - n2.quotient() <= BigInt::ONE {
            return true;
        }
    } else {
        if n2.quotient() - n1.quotient() <= BigInt::ONE {
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
