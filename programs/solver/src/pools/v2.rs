use super::*;
use uniswap_v2_sdk::prelude::*;

#[derive(Debug)]
pub struct PoolData {
    pub data: HashMap<Address, TokenData>,
}

impl PoolData {
    pub fn new<'a>(pools: &[Pools], tokens: &TokenMap) -> Result<PoolData, CustomError<'a>> {
        let mut data = HashMap::with_capacity(pools.len());

        for pool in pools {
            data.insert(
                pool.address,
                TokenData::new(
                    tokens
                        .get(&pool.token0)
                        .ok_or_else(|| CustomError::AddressNotFound(pool.token0))?
                        .clone(),
                    tokens
                        .get(&pool.token1)
                        .ok_or_else(|| CustomError::AddressNotFound(pool.token1))?
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
            let token0mount_a = CurrencyAmount::from_raw_amount(
                token_data.token0.token.clone(),
                token_data.reserve0,
            )?;
            let token0mount_b = CurrencyAmount::from_raw_amount(
                token_data.token1.token.clone(),
                token_data.reserve1,
            )?;

            let pair = Pair::new(token0mount_a.clone(), token0mount_b.clone())?;

            let mut price = pair.token0_price();
            token_data.token0.price_start = (price.numerator() * token_data.token0.scale())
                / (price.denominator() * token_data.token0.precision());

            price = pair.token1_price();
            token_data.token1.price_start = (price.numerator() * token_data.token1.scale())
                / (price.denominator() * token_data.token1.precision());
        }
        Ok(())
    }

    pub fn calc_effective_price<'a>(&mut self, amount: BigInt) -> Result<(), CustomError<'a>> {
        for (_pool_addr, token_data) in self.data.iter_mut() {
            let token0mount_a = CurrencyAmount::from_raw_amount(
                token_data.token0.token.clone(),
                token_data.reserve0,
            )?;
            let token0mount_b = CurrencyAmount::from_raw_amount(
                token_data.token1.token.clone(),
                token_data.reserve1,
            )?;

            let pair = Pair::new(token0mount_a.clone(), token0mount_b.clone())?;

            let amount0_in =
                CurrencyAmount::from_raw_amount(token_data.token0.token.clone(), amount)?;
            let (amount0_out, _) = match pair.get_output_amount(&amount0_in, false) {
                Ok(a) => a,
                Err(_err) => {
                    // log::error!("amount0_out calculation failed for {_pool_addr}, due to {_err}");
                    continue;
                }
            };

            let amount1_out =
                CurrencyAmount::from_raw_amount(token_data.token1.token.clone(), amount)?;
            let (amount1_in, _) = match pair.get_input_amount(&amount0_in, false) {
                Ok(a) => a,
                Err(_err) => {
                    // log::error!("amount1_in calculation failed for {_pool_addr}, due to {_err}");
                    continue;
                }
            };

            let mut price = Price::from_currency_amounts(amount0_in, amount0_out);
            token_data.token0.price_effective = (price.numerator() * token_data.token0.scale())
                / (price.denominator() * token_data.token0.precision());

            price = Price::from_currency_amounts(amount1_out, amount1_in);
            token_data.token1.price_effective = (price.numerator() * token_data.token1.scale())
                / (price.denominator() * token_data.token1.precision());
        }
        Ok(())
    }

    pub fn calc_slippage<'a>(
        &mut self,
        mut slippage_adj: &mut Option<BigInt>,
    ) -> Result<(), CustomError<'a>> {
        for token_data in self.data.values_mut() {
            token_data.token0.slippage = calc_slippage(
                token_data.token0.price_start,
                token_data.token0.price_effective,
                &mut slippage_adj,
            );

            token_data.token1.slippage = calc_slippage(
                token_data.token1.price_start,
                token_data.token1.price_effective,
                &mut slippage_adj,
            );
        }

        Ok(())
    }

    pub fn to_swap_graph(&self, graph: &mut SwapGraph) {
        for (pool, token_data) in self.data.iter() {
            let from = token_data.token0.token.address();
            let to = token_data.token1.token.address();
            let slippage0 = token_data.token0.slippage;
            let slippage1 = token_data.token1.slippage;
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
    pub token0: TokenDetails,
    pub token1: TokenDetails,
    pub reserve0: BigInt,
    pub reserve1: BigInt,
    pub fee: u16,
}

impl TokenData {
    fn new(token0: Token, token1: Token, fee: u16) -> Self {
        Self {
            token0: TokenDetails::new(token0),
            token1: TokenDetails::new(token1),
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

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test token
    fn create_test_token(address: Address, decimals: u8) -> Token {
        token!(1, address, decimals)
    }

    // Helper function to create a test pool
    fn create_test_pool(
        token0: Token,
        token1: Token,
        _reserve0: u128,
        _reserve1: u128,
        fee: u16,
    ) -> (Pools, TokenMap) {
        let pool_address = address!("0x0000000000000000000000000000000000000001");

        let mut tokens = TokenMap::new();
        tokens.insert(token0.address, token0.clone());
        tokens.insert(token1.address, token1.clone());

        let pool = Pools {
            token0: token0.address,
            token1: token1.address,
            fee,
            address: pool_address,
        };

        (pool, tokens)
    }

    #[test]
    fn test_calc_effective_price_basic() {
        // Create test tokens
        let token0 = create_test_token(address!("0x1000000000000000000000000000000000000001"), 18);
        let token1 = create_test_token(address!("0x2000000000000000000000000000000000000002"), 18);

        // Create test pool with 1:1 ratio (1000:1000)
        let (pool, tokens) = create_test_pool(
            token0,
            token1,
            1_000_000_000_000_000_000_000, // 1000 token A
            1_000_000_000_000_000_000_000, // 1000 token B
            3000,                          // 0.3% fee
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

        println!("{:#?}", pool_data);
        // Verify effective prices
        for data in pool_data.data.values() {
            // Due to 0.3% fee, we expect slightly less than 1:1 output
            assert!(data.token0.price_effective.lt(&data.token0.price_start));
            assert!(data.token1.price_effective.lt(&data.token1.price_start));
        }
    }

    #[test]
    fn test_calc_slippage() {
        // Create test tokens
        let token0 = create_test_token(address!("0x1000000000000000000000000000000000000001"), 18);
        let token1 = create_test_token(address!("0x2000000000000000000000000000000000000002"), 18);

        // Create test pool with 1:1 ratio (1000:1000)
        let (pool, tokens) = create_test_pool(
            token0,
            token1,
            1_000_000_000_000_000_000_000, // 1000 token A
            1_000_000_000_000_000_000_000, // 1000 token B
            3000,                          // 0.3% fee
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

        let mut slippage_adj = Some(BigInt::MIN);

        // Calculate slippage
        pool_data.calc_slippage(&mut slippage_adj).unwrap();

        // Verify slippage values
        for data in pool_data.data.values() {
            // Slippage should be positive (price impact from trade)
            assert!(data.token0.slippage > BigInt::ZERO);
            assert!(data.token1.slippage > BigInt::ZERO);

            // Slippage should be relatively small for 10% of the pool
            assert!(data.token0.slippage < BigInt::ONE); // Less than 1% slippage
            assert!(data.token1.slippage < BigInt::ONE); // Less than 1% slippage
        }
    }

    #[test]
    fn test_calc_effective_price_large_trade() {
        // Create test tokens
        let token0 = create_test_token(address!("0x1000000000000000000000000000000000000001"), 18);
        let token1 = create_test_token(address!("0x2000000000000000000000000000000000000002"), 18);

        // Create test pool with 1:1 ratio (1000:1000)
        let (pool, tokens) = create_test_pool(
            token0,
            token1,
            1_000_000_000_000_000_000_000, // 1000 token A
            1_000_000_000_000_000_000_000, // 1000 token B
            3000,                          // 0.3% fee
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

        let mut slippage_adj = Some(BigInt::MIN);

        // Calculate slippage
        pool_data.calc_slippage(&mut slippage_adj).unwrap();

        // Verify slippage values
        for data in pool_data.data.values() {
            // Slippage should be significant for a 50% trade
            assert!(data.token0.slippage > BigInt::ONE);
            assert!(data.token1.slippage > BigInt::ONE);

            // But still reasonable (less than 50%)
            assert!(data.token0.slippage < BigInt::from(50));
            assert!(data.token1.slippage < BigInt::from(50));
        }
    }
}
