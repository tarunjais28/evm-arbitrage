use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pools {
    token_a: Address,
    token_b: Address,
    address: Address,
}

impl Pools {
    pub fn to_key_value(&self) -> (TokenPair, TokenData) {
        (
            TokenPair {
                token_a: self.token_a,
                token_b: self.token_b,
            },
            TokenData {
                address: self.address,
                slippage: U256::ZERO,
                reserve0: U112::ZERO,
                reserve1: U112::ZERO,
            },
        )
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct TokenPair {
    pub token_a: Address,
    pub token_b: Address,
}

#[derive(Debug)]
pub struct TokenData {
    pub address: Address,
    pub slippage: U256,
    pub reserve0: U112,
    pub reserve1: U112,
}

impl TokenData {
    pub fn update_reserves(&mut self, reserves: Reserves) {
        self.reserve0 = reserves.reserve0;
        self.reserve1 = reserves.reserve1;
    }

    pub fn calc_slippage(&mut self) {
        let amount_in = U256::from(1);
        let fee = U256::from(300);
        let reserve0 = U256::from(self.reserve0);
        let reserve1 = U256::from(self.reserve1);

        let expected_price = reserve1.checked_div(reserve0).unwrap_or_default();

        let amount_in_net = amount_in
            .checked_mul(U256::from(10000).checked_sub(fee).unwrap_or_default())
            .unwrap_or_default();

        let amount_out = (reserve1.checked_mul(amount_in_net).unwrap_or_default())
            .checked_div(reserve0.checked_add(amount_in_net).unwrap_or_default())
            .unwrap_or_default();

        let executed_price = amount_out.checked_div(amount_in).unwrap_or_default();

        let slippage = U256::from(1)
            .checked_sub(
                executed_price
                    .checked_div(expected_price)
                    .unwrap_or_default(),
            )
            .unwrap_or_default();

        self.slippage = slippage.checked_mul(U256::from(10000)).unwrap_or_default();
    }
}
