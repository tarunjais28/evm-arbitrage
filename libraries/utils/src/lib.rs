pub use crate::{errors::*, parser::*, util::*};
use hex::FromHexError;
use num_bigint::ParseBigIntError;
use std::{
    env::{self, VarError},
    fs::File,
    io::{self, BufRead, BufReader},
};
use thiserror::Error;
use web3::types::H160;
use dotenv::dotenv;

mod errors;
mod parser;
mod util;
