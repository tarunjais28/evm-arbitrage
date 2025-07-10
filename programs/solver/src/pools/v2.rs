use super::*;
use uniswap_v2_sdk::prelude::*;

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

    pub fn calc_start_price<'a>(&mut self) -> Result<(), CustomError<'a>> {
        for token_data in self.data.values_mut() {
            let token_amount_a = CurrencyAmount::from_raw_amount(
                token_data.token_a.token.clone(),
                token_data.reserve0,
            )?;
            let token_amount_b = CurrencyAmount::from_raw_amount(
                token_data.token_b.token.clone(),
                token_data.reserve1,
            )?;

            let pair = Pair::new(token_amount_a.clone(), token_amount_b.clone())?;

            token_data.token_a.price_start = pair.token0_price();
            token_data.token_b.price_start = pair.token1_price();
        }
        Ok(())
    }

    pub fn calc_effective_price<'a>(&mut self, amount: BigInt) -> Result<(), CustomError<'a>> {
        for token_data in self.data.values_mut() {
            let token_amount_a = CurrencyAmount::from_raw_amount(
                token_data.token_a.token.clone(),
                token_data.reserve0,
            )?;
            let token_amount_b = CurrencyAmount::from_raw_amount(
                token_data.token_b.token.clone(),
                token_data.reserve1,
            )?;

            let pair = Pair::new(token_amount_a.clone(), token_amount_b.clone())?;

            let amount0_in =
                CurrencyAmount::from_raw_amount(token_data.token_a.token.clone(), amount)?;
            let (amount0_out, _) = pair.get_output_amount(&amount0_in, false)?;

            let amount1_out =
                CurrencyAmount::from_raw_amount(token_data.token_b.token.clone(), amount)?;
            let (amount1_in, _) = pair.get_input_amount(&amount0_in, false)?;

            token_data.token_a.price_effective =
                Price::from_currency_amounts(amount0_in, amount0_out);
            token_data.token_b.price_effective =
                Price::from_currency_amounts(amount1_out, amount1_in);
        }
        Ok(())
    }

    pub fn calc_slippage<'a>(&mut self) -> Result<(), CustomError<'a>> {
        for token_data in self.data.values_mut() {
            token_data.token_a.slippage = calc_slippage(
                token_data.token_a.price_start.clone(),
                token_data.token_a.price_effective.clone(),
            )?;

            token_data.token_b.slippage = calc_slippage(
                token_data.token_b.price_start.clone(),
                token_data.token_b.price_effective.clone(),
            )?;
        }

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

    // Helper function to create a test token
    fn create_test_token(address: Address, decimals: u8) -> Token {
        token!(1, address, decimals)
    }

    // Helper function to create a test pool
    fn create_test_pool(
        token_a: Token,
        token_b: Token,
        reserve0: u128,
        reserve1: u128,
        fee: u16,
    ) -> (Pools, TokenMap) {
        let pool_address = address!("0x0000000000000000000000000000000000000001");

        let mut tokens = TokenMap::new();
        tokens.insert(token_a.address, token_a.clone());
        tokens.insert(token_b.address, token_b.clone());

        let pool = Pools {
            token_a: token_a.address,
            token_b: token_b.address,
            fee,
            address: pool_address,
        };

        (pool, tokens)
    }

    #[test]
    fn test_calc_effective_price_basic() {
        // Create test tokens
        let token_a = create_test_token(address!("0x1000000000000000000000000000000000000001"), 18);
        let token_b = create_test_token(address!("0x2000000000000000000000000000000000000002"), 18);

        // Create test pool with 1:1 ratio (1000:1000)
        let (pool, tokens) = create_test_pool(
            token_a,
            token_b,
            1_000_000_000_000_000_000_000, // 1000 token A
            1_000_000_000_000_000_000_000, // 1000 token B
            3000,                            // 0.3% fee
        );

        // Create pool data
        let mut pool_data = PoolData::new(&[pool], &tokens).unwrap();

        // Set initial reserves
        pool_data.data.values_mut().for_each(|data| {
            data.reserve0 = BigInt::from(1_000_000_000_000_000_000_000u128); // 1000 token A
            data.reserve1 = BigInt::from(1_000_000_000_000_000_000_000u128); // 1000 token B
        });

        // Calculate start price (should be 1:1)
        pool_data.calc_start_price().unwrap();

        // Calculate effective price for 1 token (accounting for 0.3% fee)
        let amount = BigInt::from(1_000_000_000_000_000_000u128); // 1.0 token
        pool_data.calc_effective_price(amount).unwrap();

        // Verify effective prices
        for data in pool_data.data.values() {
            // Due to 0.3% fee, we expect slightly less than 1:1 output
            assert!(data
                .token_a
                .price_effective
                .lt(&data.token_a.price_start));
            assert!(data
                .token_b
                .price_effective
                .lt(&data.token_b.price_start));
        }
    }

    #[test]
    fn test_calc_slippage() {
        // Create test tokens
        let token_a = create_test_token(address!("0x1000000000000000000000000000000000000001"), 18);
        let token_b = create_test_token(address!("0x2000000000000000000000000000000000000002"), 18);

        // Create test pool with 1:1 ratio (1000:1000)
        let (pool, tokens) = create_test_pool(
            token_a,
            token_b,
            1_000_000_000_000_000_000_000, // 1000 token A
            1_000_000_000_000_000_000_000, // 1000 token B
            3000,                            // 0.3% fee
        );

        // Create pool data
        let mut pool_data = PoolData::new(&[pool], &tokens).unwrap();

        // Set initial reserves
        pool_data.data.values_mut().for_each(|data| {
            data.reserve0 = BigInt::from(1_000_000_000_000_000_000_000u128); // 1000 token A
            data.reserve1 = BigInt::from(1_000_000_000_000_000_000_000u128); // 1000 token B
        });

        // Calculate start price (should be 1:1)
        pool_data.calc_start_price().unwrap();

        // Calculate effective price for 10% of the pool
        let amount = BigInt::from(100_000_000_000_000_000_000u128); // 100 tokens (10% of reserve)
        pool_data.calc_effective_price(amount).unwrap();

        // Calculate slippage
        pool_data.calc_slippage().unwrap();

        // Verify slippage values
        for data in pool_data.data.values() {
            // Slippage should be positive (price impact from trade)
            assert!(data.token_a.slippage > BigInt::ZERO);
            assert!(data.token_b.slippage > BigInt::ZERO);

            // Slippage should be relatively small for 10% of the pool
            assert!(data.token_a.slippage < BigInt::ONE); // Less than 1% slippage
            assert!(data.token_b.slippage < BigInt::ONE); // Less than 1% slippage
        }
    }

    #[test]
    fn test_calc_effective_price_large_trade() {
        // Create test tokens
        let token_a = create_test_token(address!("0x1000000000000000000000000000000000000001"), 18);
        let token_b = create_test_token(address!("0x2000000000000000000000000000000000000002"), 18);

        // Create test pool with 1:1 ratio (1000:1000)
        let (pool, tokens) = create_test_pool(
            token_a,
            token_b,
            1_000_000_000_000_000_000_000, // 1000 token A
            1_000_000_000_000_000_000_000, // 1000 token B
            3000,                            // 0.3% fee
        );

        // Create pool data
        let mut pool_data = PoolData::new(&[pool], &tokens).unwrap();

        // Set initial reserves
        pool_data.data.values_mut().for_each(|data| {
            data.reserve0 = BigInt::from(1_000_000_000_000_000_000_000u128); // 1000 token A
            data.reserve1 = BigInt::from(1_000_000_000_000_000_000_000u128); // 1000 token B
        });

        // Calculate start price (should be 1:1)
        pool_data.calc_start_price().unwrap();

        // Calculate effective price for 50% of the pool (large trade)
        let amount = BigInt::from(500_000_000_000_000_000_000u128); // 500 tokens (50% of reserve)
        pool_data.calc_effective_price(amount).unwrap();

        // Calculate slippage
        pool_data.calc_slippage().unwrap();

        // Verify slippage values
        for data in pool_data.data.values() {
            // Slippage should be significant for a 50% trade
            assert!(data.token_a.slippage > BigInt::ONE);
            assert!(data.token_b.slippage > BigInt::ONE);

            // But still reasonable (less than 50%)
            assert!(data.token_a.slippage < BigInt::from(50));
            assert!(data.token_b.slippage < BigInt::from(50));
        }
    }
}
