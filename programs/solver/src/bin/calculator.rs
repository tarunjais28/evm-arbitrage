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

fn main() {
    let a = BigInt::from(1967961004141248372392u128);
    let b = BigInt::from(945968064780900579039u128);
    println!("{}", add!(a, b));
}
