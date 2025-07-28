use super::*;

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

    pub fn calc_start_price<'a>(
        &mut self,
        tick_map: &parser::TickMap,
    ) -> Result<(), CustomError<'a>> {
        for (pool_addr, token_data) in self.data.iter_mut() {
            if let Some(tick_data) = tick_map.get(pool_addr) {
                let tick_data = tick_data.clone();
                let pool = Pool::new(
                    token_data.token_a.token.clone(),
                    token_data.token_b.token.clone(),
                    token_data.fee(),
                    tick_data.sqrt_price_x96,
                    tick_data.liquidity,
                )?;

                let mut price = pool.token0_price();
                token_data.token_a.price_start = (price.numerator() * token_data.token_a.scale())
                    / (price.denominator() * token_data.token_a.precision());

                price = pool.token1_price();
                token_data.token_b.price_start = (price.numerator() * token_data.token_b.scale())
                    / (price.denominator() * token_data.token_b.precision());

                token_data.liquidity = tick_data.liquidity;
                token_data.sqrt_price_x96 = tick_data.sqrt_price_x96;
                token_data.current_tick = tick_data.current_tick;
                token_data.ticks = tick_data.ticks;
            }
        }

        Ok(())
    }

    pub fn calc_start_price_from_sqrt_price_x96<'a>(
        &mut self,
        pool_address: &Address,
        swap: IUniswapV3Pool::Swap,
    ) -> Result<(), CustomError<'a>> {
        let token_data = self
            .data
            .get_mut(pool_address)
            .ok_or_else(|| CustomError::AddressNotFound(*pool_address))?;

        let pool = Pool::new(
            token_data.token_a.token.clone(),
            token_data.token_b.token.clone(),
            token_data.fee(),
            token_data.sqrt_price_x96,
            token_data.liquidity,
        )?;

        let amount0 = CurrencyAmount::from_raw_amount(
            token_data.token_a.token.clone(),
            swap.amount0.to_big_int(),
        )?;

        match pool.get_output_amount_sync(
            &amount0,
            None,
            token_data.current_tick,
            &token_data.ticks,
        ) {
            Ok(amount1) => {
                println!("{}", format!("found amount0: {}", swap.amount0).green());
                println!("{}", format!("found amount1: {}", swap.amount1).green());
                println!(
                    "{}",
                    format!("   calculated: {}\n", BigInt::from(-1) * amount1.quotient()).green()
                );
            }
            Err(err) => log::error!("{err}"),
        };

        let pool = Pool::new(
            token_data.token_a.token.clone(),
            token_data.token_b.token.clone(),
            token_data.fee(),
            swap.sqrtPriceX96,
            swap.liquidity,
        )?;

        let mut price = pool.token0_price();
        token_data.token_a.price_start = (price.numerator() * token_data.token_a.scale())
            / (price.denominator() * token_data.token_a.precision());

        price = pool.token1_price();
        token_data.token_b.price_start = (price.numerator() * token_data.token_b.scale())
            / (price.denominator() * token_data.token_b.precision());

        token_data.liquidity = pool.liquidity;
        token_data.sqrt_price_x96 = pool.sqrt_ratio_x96;
        token_data.current_tick = swap.tick;

        Ok(())
    }

    pub async fn calc_effective_price<'a>(
        &mut self,
        amount: BigInt,
    ) -> Result<(), CustomError<'a>> {
        for (_pool_addr, token_data) in self.data.iter_mut() {
            if let Ok(pool) = Pool::new(
                token_data.token_a.token.clone(),
                token_data.token_b.token.clone(),
                token_data.fee(),
                token_data.sqrt_price_x96,
                token_data.liquidity,
            ) {
                let amount0_in =
                    CurrencyAmount::from_raw_amount(token_data.token_a.token.clone(), amount)?;
                let amount0_out = match pool.get_output_amount_sync(
                    &amount0_in,
                    None,
                    token_data.current_tick,
                    &token_data.ticks,
                ) {
                    Ok(a) => a,
                    Err(_err) => {
                        // log::error!("amount0_out calculation failed for {_pool_addr}, due to {_err}");
                        continue;
                    }
                };

                let amount1_out =
                    CurrencyAmount::from_raw_amount(token_data.token_b.token.clone(), amount)?;
                let amount1_in = match pool.get_input_amount_sync(
                    &amount1_out,
                    None,
                    token_data.current_tick,
                    &token_data.ticks,
                ) {
                    Ok(a) => a,
                    Err(_err) => {
                        // log::error!("amount0_out calculation failed for {_pool_addr}, due to {_err}");
                        continue;
                    }
                };

                let mut price = Price::from_currency_amounts(amount0_in, amount0_out);
                token_data.token_a.price_effective = (price.numerator()
                    * token_data.token_a.scale())
                    / (price.denominator() * token_data.token_a.precision());

                price = Price::from_currency_amounts(amount1_out, amount1_in);
                token_data.token_b.price_effective = (price.numerator()
                    * token_data.token_b.scale())
                    / (price.denominator() * token_data.token_b.precision());
            }
        }
        Ok(())
    }

    pub fn calc_slippage<'a>(
        &mut self,
        mut slippage_adj: &mut Option<BigInt>,
    ) -> Result<(), CustomError<'a>> {
        for token_data in self.data.values_mut() {
            token_data.token_a.slippage = calc_slippage(
                token_data.token_a.price_start,
                token_data.token_a.price_effective,
                &mut slippage_adj,
            );

            token_data.token_b.slippage = calc_slippage(
                token_data.token_b.price_start,
                token_data.token_b.price_effective,
                &mut slippage_adj,
            );
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
    pub fee: u16,
    pub liquidity: u128,
    pub sqrt_price_x96: U160,
    pub current_tick: I24,
    pub ticks: Vec<TickSync>,
}

impl TokenData {
    fn new(token_a: Token, token_b: Token, fee: u16) -> Self {
        Self {
            token_a: TokenDetails::new(token_a),
            token_b: TokenDetails::new(token_b),
            fee,
            liquidity: u128::default(),
            sqrt_price_x96: U160::default(),
            current_tick: I24::ZERO,
            ticks: Vec::default(),
        }
    }

    fn fee(&self) -> FeeAmount {
        use FeeAmount::*;
        match self.fee {
            100 => LOWEST,
            300 => LOW_300,
            500 => LOW,
            3000 => MEDIUM,
            10000 => HIGH,
            _ => CUSTOM(0),
        }
    }
}
