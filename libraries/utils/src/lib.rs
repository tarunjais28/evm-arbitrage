pub use crate::{errors::*, parser::*, util::*};
use dotenv::dotenv;
use hex::FromHexError;
use num_bigint::ParseBigIntError;
use std::{
    env::{self, VarError},
    fs::File,
    io::{self, BufRead, BufReader},
};
use thiserror::Error;
use web3::types::H160;

mod errors;
mod parser;
mod util;
