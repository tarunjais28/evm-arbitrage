use super::*;

/// Calculate slippage
pub fn calc_individual_slippage(
    reserve_in: FractionLike<CurrencyMeta<CurrencyLike<false, TokenMeta>>>,
    reserve_out: FractionLike<CurrencyMeta<CurrencyLike<false, TokenMeta>>>,
    fee: BigInt,
    amount_in: FractionLike<CurrencyMeta<CurrencyLike<false, TokenMeta>>>,
) -> BigInt {
    let precision = BigInt::from(10u128.pow(9));
    let net_percent = BigInt::from(1000000);

    let expected_price = reserve_out.divide(&reserve_in).unwrap();
    println!("expected_price: {}", expected_price.quotient());

    let amount_in_net = amount_in.quotient() * (net_percent - fee);
    println!("amount_in_net: {amount_in_net}");

    let reserve_in_net = reserve_in.quotient() * net_percent;
    println!("reserve_in_net: {reserve_in_net}");

    let amount_out = (reserve_out.quotient() * amount_in_net)
        .checked_div(reserve_in_net + amount_in_net)
        .unwrap_or_default();
    println!("amount_out: {amount_out}");

    let effective_price = amount_out
        .checked_div(amount_in.quotient())
        .unwrap_or_default();
    println!("effective_price: {effective_price}");

    (expected_price
        .quotient()
        .checked_sub(effective_price)
        .unwrap_or_default()
        * BigInt::from(100)
        * precision)
        .checked_div(expected_price.quotient())
        .unwrap_or_default()
}

pub fn calc_slippage<'a>(
    start_price: PriceData,
    end_price: PriceData,
    slippage_adj: &mut BigInt,
) -> Result<BigInt, CustomError<'a>> {
    let percent = BigInt::from(1000000);

    let start_price = start_price.adjusted_for_decimals();
    let end_price = end_price.adjusted_for_decimals();

    let slippage = if start_price.numerator.gt(&BigInt::ZERO) {
        let slippage_fract = (end_price - start_price.clone()) / start_price;
        slippage_fract
            .numerator()
            .mul(percent)
            .div_floor(slippage_fract.denominator())
    } else {
        percent
    };

    *slippage_adj = BigInt::min(slippage, *slippage_adj);

    Ok(slippage)
}
