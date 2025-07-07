use super::*;

pub type PoolData = HashMap<Address, TokenData>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pools {
    pub token_a: Address,
    pub token_b: Address,
    pub decimals0: u8,
    pub decimals1: u8,
    pub fee: u16,
    pub address: Address,
}

impl Pools {
    pub fn to_key_value(&self) -> (Address, TokenData) {
        (
            self.address,
            TokenData {
                token_a: self.token_a,
                token_b: self.token_b,
                slippage0: U256::ZERO,
                slippage1: U256::ZERO,
                reserve0: U256::ZERO,
                reserve1: U256::ZERO,
                decimals0: self.decimals0,
                decimals1: self.decimals1,
                fee: self.fee,
            },
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TokenData {
    pub token_a: Address,
    pub token_b: Address,
    pub slippage0: U256,
    pub slippage1: U256,
    pub reserve0: U256,
    pub reserve1: U256,
    pub decimals0: u8,
    pub decimals1: u8,
    pub fee: u16,
}

impl TokenData {
    pub fn update_reserves(&mut self, reserves: Reserves) {
        self.reserve0 = reserves.reserve0;
        self.reserve1 = reserves.reserve1;
    }

    pub fn calc_slippages(&mut self) {
        let reserve0 = U256::from(self.reserve0);
        let reserve1 = U256::from(self.reserve1);
        let precision0 = U256::from(10u128.pow(self.decimals0 as u32));
        let precision1 = U256::from(10u128.pow(self.decimals1 as u32));

        let fee = if self.fee == 0 {
            U256::from(3000)
        } else {
            U256::from(self.fee)
        };

        self.slippage0 = calc_individual_slippage(reserve0, precision0, reserve1, precision1, fee);
        self.slippage1 = calc_individual_slippage(reserve1, precision1, reserve0, precision0, fee);
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
            slippage0: U256::ZERO,
            slippage1: U256::ZERO,
            reserve0: U256::ZERO,
            reserve1: U256::ZERO,
            decimals0: 0,
            decimals1: 0,
            fee: 0,
        };
        token_data.calc_slippages();
        assert_eq!(token_data.slippage0, U256::from(1000000000));
        assert_eq!(token_data.slippage1, U256::from(1000000000));
    }

    #[test]
    fn test_calc_slippage_equal_reserves() {
        let mut token_data = TokenData {
            token_a: Address::ZERO,
            token_b: Address::ZERO,
            slippage0: U256::ZERO,
            slippage1: U256::ZERO,
            reserve0: U256::from(1000),
            reserve1: U256::from(1000),
            decimals0: 0,
            decimals1: 0,
            fee: 0,
        };
        token_data.calc_slippages();
        assert_eq!(token_data.slippage0, U256::from(999999001));
        assert_eq!(token_data.slippage1, U256::from(999999001));
    }

    #[test]
    fn test_calc_slippage_unequal_reserves() {
        let mut token_data = TokenData {
            token_a: Address::ZERO,
            token_b: Address::ZERO,
            slippage0: U256::ZERO,
            slippage1: U256::ZERO,
            reserve0: U256::from(1000),
            reserve1: U256::from(2000),
            decimals0: 0,
            decimals1: 0,
            fee: 0,
        };
        token_data.calc_slippages();
        assert_eq!(token_data.slippage0, U256::from(999999001));
        assert_eq!(token_data.slippage1, U256::from(999998002));
    }

    #[test]
    fn test_calc_slippage_one_reserve_is_zero() {
        let mut token_data = TokenData {
            token_a: Address::ZERO,
            token_b: Address::ZERO,
            slippage0: U256::ZERO,
            slippage1: U256::ZERO,
            reserve0: U256::from(1000),
            reserve1: U256::ZERO,
            decimals0: 0,
            decimals1: 0,
            fee: 0,
        };
        token_data.calc_slippages();
        assert_eq!(token_data.slippage0, U256::from(1000000000));
        assert_eq!(token_data.slippage1, U256::from(1000000000));
    }

    #[test]
    fn test_calc_slippage() {
        let mut token_data = TokenData {
            token_a: Address::ZERO,
            token_b: Address::ZERO,
            slippage0: U256::ZERO,
            slippage1: U256::ZERO,
            reserve0: U256::from(1551650201200975628814u128),
            reserve1: U256::from(1178164302654065252u128),
            fee: 0,
            decimals0: 18,
            decimals1: 18,
        };
        token_data.calc_slippages();
        assert_eq!(token_data.slippage0, U256::from(3000144));
        assert_eq!(token_data.slippage1, U256::from(3000001));
    }
}
