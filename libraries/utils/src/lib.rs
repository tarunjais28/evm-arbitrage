pub use crate::{errors::*, parser::*, util::*};
use alloy::primitives::Address;
use dotenv::dotenv;
use num_bigint::ParseBigIntError;
use std::{
    env::{self, VarError},
    fs::File,
    io::{self, BufRead, BufReader},
    str::FromStr,
};
use thiserror::Error;
use web3::types::H160;

mod errors;
mod parser;
mod util;
