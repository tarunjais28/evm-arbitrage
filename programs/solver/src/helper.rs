use super::*;

/// Compute virtual reserves given sqrtPriceX96 and liquidity.
pub fn compute_reserves_v3(
    sqrt_price_x96: alloy::primitives::U160,
    liquidity: u128,
) -> (U256, U256) {
    // 2^96 = 79228162514264337593543950336
    let q96 = U256::from(2u128.pow(96));
    let liquidity = U256::from(liquidity);
    let sqrt_price_x96 = U256::from(sqrt_price_x96);

    // P = sqrtPriceX96 / q96
    // reserve0 = liquidity / P = liquidity * q96 / sqrtPriceX96
    // reserve1 = liquidity * P = liquidity * sqrtPriceX96 / q96

    // reserve0 = liquidity * q96 / sqrtPriceX96
    let reserve0 = liquidity
        .checked_mul(q96)
        .expect("Multiplication overflow")
        .checked_div(sqrt_price_x96)
        .expect("Division by zero");

    // reserve1 = liquidity * sqrtPriceX96 / q96
    let reserve1 = liquidity
        .checked_mul(sqrt_price_x96)
        .expect("Multiplication overflow")
        .checked_div(q96)
        .expect("Division by zero");

    (reserve0, reserve1)
}
