use super::*;

pub fn calc_slippage(
    start_price: BigInt,
    end_price: BigInt,
    slippage_adj: &mut Option<BigInt>,
) -> BigInt {
    let percent = BigInt::from(1000000);

    // TODO: Confirm end_price 0 logic
    let slippage = if end_price.is_zero() {
        BigInt::ZERO
    } else {
        (end_price - start_price) * percent / end_price
    };

    if let Some(slip_adj) = slippage_adj {
        *slippage_adj = Some(BigInt::min(slippage, *slip_adj));
    }

    slippage
}

pub fn is_abs_le_1(n1: &BigInt, n2: &BigInt) -> bool {
    if n1 > n2 {
        if n1 - n2 <= BigInt::ONE {
            return true;
        }
    } else {
        if n2 - n1 <= BigInt::ONE {
            return true;
        }
    }

    false
}
