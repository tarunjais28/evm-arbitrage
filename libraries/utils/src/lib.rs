pub use crate::{errors::*, parser::*, util::*};
use alloy::{
    contract,
    primitives::Address,
    transports::{RpcError, TransportErrorKind},
};
use dotenv::dotenv;
use num_bigint::ParseBigIntError;
use serde_json::from_reader;
use std::{
    env::{self, VarError},
    fs::File,
    io::{self, BufReader},
};
use thiserror::Error;
use web3::types::H160;

mod errors;
mod parser;
mod util;
#[macro_use]
pub mod logger;
