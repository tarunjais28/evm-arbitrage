use super::*;

fn calc_slippage(reserve_in: U112, reserve_out: U112, amount_in: U256, fee: U256) -> U256 {
    let expected_price = reserve_out / reserve_in;

    let amount_in_net = amount_in * (U256::from(10000) - fee);

    let amount_out =
        U256::from(reserve_out) * amount_in_net / (U256::from(reserve_in) + amount_in_net);

    let executed_price = amount_out / amount_in;

    let slippage = U256::from(1) - (executed_price / U256::from(expected_price));

    slippage * U256::from(10000)
}
