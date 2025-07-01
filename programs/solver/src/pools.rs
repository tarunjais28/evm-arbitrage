use super::*;

pub type PoolData = HashMap<Address, TokenData>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pools {
    token_a: Address,
    token_b: Address,
    pub address: Address,
}

impl Pools {
    pub fn to_key_value(&self) -> (Address, TokenData) {
        (
            self.address,
            TokenData {
                token_a: self.token_a,
                token_b: self.token_b,

                slippage: U256::ZERO,
                reserve0: U112::ZERO,
                reserve1: U112::ZERO,
            },
        )
    }
}

#[derive(Debug)]
pub struct TokenData {
    pub token_a: Address,
    pub token_b: Address,
    pub slippage: U256,
    pub reserve0: U112,
    pub reserve1: U112,
}

impl TokenData {
    pub fn update_reserves(&mut self, reserves: Reserves) {
        self.reserve0 = reserves.reserve0;
        self.reserve1 = reserves.reserve1;
    }

    pub fn calc_slippage(&mut self, amount_in: U256) {
        let fee = U256::from(3);
        let net_percent = U256::from(1000);
        let reserve0 = U256::from(self.reserve0);
        let reserve1 = U256::from(self.reserve1);

        let expected_price = reserve1.checked_div(reserve0).unwrap_or_default();

        let amount_in_net = amount_in
            .checked_mul(net_percent.checked_sub(fee).unwrap_or_default())
            .unwrap_or_default();

        let reserve0_net = reserve0.checked_mul(net_percent).unwrap_or_default();

        let amount_out = (reserve1.checked_mul(amount_in_net).unwrap_or_default())
            .checked_div(reserve0_net.checked_add(amount_in_net).unwrap_or_default())
            .unwrap_or_default();

        let executed_price = amount_out.checked_div(amount_in).unwrap_or_default();

        let slippage = U256::from(1)
            .checked_sub(
                executed_price
                    .checked_div(expected_price)
                    .unwrap_or_default(),
            )
            .unwrap_or_default();

        self.slippage = slippage.checked_mul(net_percent).unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calc_slippage_zero_reserves() {
        let mut token_data = TokenData {
            token_a: Address::ZERO,
            token_b: Address::ZERO,
            slippage: U256::ZERO,
            reserve0: U112::ZERO,
            reserve1: U112::ZERO,
        };
        let amount_in = U256::from(1);
        token_data.calc_slippage(amount_in);
        assert_eq!(token_data.slippage, U256::from(1000));
    }

    #[test]
    fn test_calc_slippage_equal_reserves() {
        let mut token_data = TokenData {
            token_a: Address::ZERO,
            token_b: Address::ZERO,
            slippage: U256::ZERO,
            reserve0: U112::from(1000),
            reserve1: U112::from(1000),
        };
        let amount_in = U256::from(1);
        token_data.calc_slippage(amount_in);
        assert_eq!(token_data.slippage, U256::from(1000));
    }

    #[test]
    fn test_calc_slippage_unequal_reserves() {
        let mut token_data = TokenData {
            token_a: Address::ZERO,
            token_b: Address::ZERO,
            slippage: U256::ZERO,
            reserve0: U112::from(1000),
            reserve1: U112::from(2000),
        };
        let amount_in = U256::from(1);
        token_data.calc_slippage(amount_in);
        assert_eq!(token_data.slippage, U256::from(1000));
    }

    #[test]
    fn test_calc_slippage_one_reserve_is_zero() {
        let mut token_data = TokenData {
            token_a: Address::ZERO,
            token_b: Address::ZERO,
            slippage: U256::ZERO,
            reserve0: U112::from(1000),
            reserve1: U112::ZERO,
        };
        let amount_in = U256::from(1);
        token_data.calc_slippage(amount_in);
        assert_eq!(token_data.slippage, U256::from(1000));
    }
}
