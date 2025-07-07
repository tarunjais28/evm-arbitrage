use super::*;

/// Calculate slippage
pub fn calc_individual_slippage(
    reserve_in: U256,
    precision_in: U256,
    reserve_out: U256,
    precision_out: U256,
    fee: U256,
) -> U256 {
    let amount_in = U256::from(1) * precision_in;
    let precision = U256::from(10u128.pow(9));
    let net_percent = U256::from(1000000);

    let expected_price = (reserve_out * precision_in * precision)
        .checked_div(reserve_in * precision_out)
        .unwrap_or_default();

    let amount_in_net = amount_in * (net_percent - fee) * precision;

    let reserve_in_net = reserve_in * net_percent;

    let amount_out = (reserve_out * amount_in_net)
        .checked_div(reserve_in_net + amount_in_net)
        .unwrap_or_default();

    let effective_price = (amount_out * precision_in * precision)
        .checked_div(amount_in * precision_out)
        .unwrap_or_default();

    let slippage = (U256::from(1) * precision)
        .checked_sub(
            effective_price
                .checked_div(expected_price)
                .unwrap_or_default(),
        )
        .unwrap_or_default();

    slippage
}
