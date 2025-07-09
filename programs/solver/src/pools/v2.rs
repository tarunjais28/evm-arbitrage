use super::*;

pub struct PoolData {
    pub data: HashMap<Address, TokenData>,
}

impl PoolData {
    pub fn new<'a>(pools: &[Pools], tokens: &TokenMap) -> Result<PoolData, CustomError<'a>> {
        let mut data = HashMap::with_capacity(tokens.len());

        for pool in pools {
            data.insert(
                pool.address,
                TokenData::new(
                    tokens
                        .get(&pool.token_a)
                        .ok_or_else(|| CustomError::AddressNotFound(pool.token_a))?
                        .clone(),
                    tokens
                        .get(&pool.token_b)
                        .ok_or_else(|| CustomError::AddressNotFound(pool.token_b))?
                        .clone(),
                    pool.fee,
                ),
            );
        }

        Ok(PoolData { data })
    }

    pub async fn update_reserves<'a>(
        &mut self,
        provider: &FillProvider<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            RootProvider,
        >,
        pool_address: &[Address],
    ) -> Result<(), CustomError<'a>> {
        // Get all reserves in a single batch
        let reserves_map = debug_time!("update_reserves::get_reserves_v2()", {
            get_reserves_v2(&provider, pool_address).await?
        });

        for (pool, data) in self.data.iter_mut() {
            if let Some(reserves) = reserves_map.get(pool) {
                data.update_reserves(*reserves);
            }
        }

        Ok(())
    }

    pub fn calc_slippage<'a>(&mut self, amount_in: BigInt) -> Result<(), CustomError<'a>> {
        self.data
            .iter_mut()
            .for_each(|(_, token_data)| token_data.calc_slippages(amount_in)?);
        Ok(())
    }

    pub fn to_swap_graph(&self, graph: &mut SwapGraph) {
        for (pool, token_data) in self.data.iter() {
            let from = token_data.token_a.token.address();
            let to = token_data.token_b.token.address();
            let slippage0 = token_data.token_a.slippage;
            let slippage1 = token_data.token_b.slippage;
            let fee = token_data.fee;

            graph
                .entry(from)
                .or_default()
                .push(SwapEdge::new(to, *pool, slippage0, fee));

            graph
                .entry(to)
                .or_default()
                .push(SwapEdge::new(from, *pool, slippage1, fee));
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenData {
    pub token_a: TokenDetails,
    pub token_b: TokenDetails,
    pub reserve0: BigInt,
    pub reserve1: BigInt,
    pub fee: u16,
}

impl TokenData {
    fn new(token_a: Token, token_b: Token, fee: u16) -> Self {
        Self {
            token_a: TokenDetails::new(token_a),
            token_b: TokenDetails::new(token_b),
            fee,
            reserve0: BigInt::ZERO,
            reserve1: BigInt::ZERO,
        }
    }

    pub fn update_reserves(&mut self, reserves: Reserves) {
        self.reserve0 = reserves.reserve0;
        self.reserve1 = reserves.reserve1;
    }

    fn calc_slippages<'a>(&mut self, amount_in: BigInt) -> Result<(), CustomError<'a>> {
        let reserve0 = CurrencyAmount::from_raw_amount(self.token_a.token.clone(), self.reserve0)?;
        let reserve1 = CurrencyAmount::from_raw_amount(self.token_b.token.clone(), self.reserve1)?;

        let mut amount = CurrencyAmount::from_raw_amount(self.token_a.token.clone(), amount_in)?;
        self.token_a.slippage = calc_individual_slippage(
            reserve0.clone(),
            reserve1.clone(),
            BigInt::from(self.fee),
            amount,
        );
        println!("slippage: {}\n\n", self.token_a.slippage);

        amount = CurrencyAmount::from_raw_amount(self.token_b.token.clone(), amount_in)?;
        self.token_b.slippage =
            calc_individual_slippage(reserve1, reserve0, BigInt::from(self.fee), amount);
        println!("slippage: {}\n\n", self.token_b.slippage);

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pools {
    token_a: Address,
    token_b: Address,
    fee: u16,
    address: Address,
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
        token_data.calc_slippages(U256::ONE);
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
        token_data.calc_slippages(U256::ONE);
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
        token_data.calc_slippages(U256::ONE);
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
        token_data.calc_slippages(U256::ONE);
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
        token_data.calc_slippages(U256::ONE);
        assert_eq!(token_data.slippage0, U256::from(3000144));
        assert_eq!(token_data.slippage1, U256::from(999999999));
    }

    #[test]
    fn test_calc_slippage_with_different_decimals() {
        let mut token_data = TokenData {
            token_a: Address::ZERO,
            token_b: Address::ZERO,
            slippage0: U256::ZERO,
            slippage1: U256::ZERO,
            reserve0: U256::from(1551650201200975628814u128),
            reserve1: U256::from(1178164302654065252u128),
            fee: 0,
            decimals0: 18,
            decimals1: 6,
        };
        token_data.calc_slippages(U256::ONE);
        assert_eq!(token_data.slippage0, U256::ZERO);
        assert_eq!(token_data.slippage1, U256::ZERO);
    }
}
