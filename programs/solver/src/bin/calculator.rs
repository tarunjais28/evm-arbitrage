use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use serde_json::from_reader;
use std::{collections::HashSet, fs::File, io::BufReader};
use uniswap_sdk_core::prelude::*;

macro_rules! add {
    ( $( $x:expr ),* ) => {
        {
            let mut sum = BigInt::ZERO;
            $(
                sum += $x;
            )*
            sum
        }
    };
}

macro_rules! mult {
    ( $( $x:expr ),* ) => {
        {
            let mut mul = BigInt::ONE;
            $(
                mul *= $x;
            )*
            mul
        }
    };
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

fn main() {
    let a = BigInt::from(836278926734177605857u128);
    let b = BigInt::from(836278926734790510310u128);
    println!("{}", a - b);
    println!("{}", count_digits((a - b).abs()));
}
