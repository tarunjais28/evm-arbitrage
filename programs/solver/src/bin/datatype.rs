use alloy_primitives::U256;
use uniswap_sdk_core::prelude::{BigInt, BigUint};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    println!("U256: {}", U256::MAX);
    println!("BigInt: {}", BigInt::MAX);
    println!("BigUint: {}", BigUint::MAX);

    Ok(())
}
