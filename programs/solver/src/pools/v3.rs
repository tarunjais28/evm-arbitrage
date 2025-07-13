use super::*;

pub struct PoolData {
    pub data: HashMap<Address, TokenData>,
}

#[derive(Debug, Default, Clone)]
pub struct TickData {
    initialised_ticks: Vec<(I24, bool)>,
    ticks: Vec<Tick<I24>>,
}

impl PoolData {
    pub fn new<'a>(
        serialised_v3_pool: &[Pools],
        tokens: &TokenMap,
    ) -> Result<PoolData, CustomError<'a>> {
        let mut data = HashMap::with_capacity(tokens.len());

        for pool in serialised_v3_pool {
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

    pub async fn calc_start_price<'a>(
        &mut self,
        provider: &FillProvider<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            RootProvider,
        >,
    ) -> Result<(), CustomError<'a>> {
        let mut futures = Vec::new();
        for (pool_addr, token_data) in self.data.iter() {
            let pool_addr = pool_addr.clone();
            let fut = async move {
                let pool =
                    Pool::<EphemeralTickMapDataProvider>::from_pool_key_with_tick_data_provider(
                        1,
                        FACTORY_ADDRESS,
                        token_data.token_a.token.address(),
                        token_data.token_b.token.address(),
                        token_data.fee(),
                        provider,
                        None,
                    )
                    .await?;

                let tick_data = get_tick_data(&pool).await?;

                Ok((
                    pool_addr,
                    pool.token0_price(),
                    pool.token1_price(),
                    tick_data,
                    pool.liquidity,
                    pool.sqrt_ratio_x96,
                ))
            };
            futures.push(fut);
        }

        let results: Vec<
            Result<(Address, PriceData, PriceData, TickData, u128, U160), CustomError<'a>>,
        > = futures::stream::iter(futures)
            .buffer_unordered(50)
            .collect::<Vec<_>>()
            .await;

        results.into_iter().for_each(|res| {
            if let Ok((pool_addr, price0, price1, tick_data, liquidity, sqrt_ratio_x96)) = res {
                self.data.entry(pool_addr).and_modify(|token_data| {
                    token_data.token_a.price_start = price0;
                    token_data.token_b.price_start = price1;
                    token_data.tick_data = tick_data;
                    token_data.liquidity = liquidity;
                    token_data.sqrt_price_x96 = sqrt_ratio_x96;
                });
            }
        });

        Ok(())
    }

    pub async fn calc_start_price_from_sqrt_price_x96<'a>(
        &mut self,
        provider: &FillProvider<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            RootProvider,
        >,
        pool_address: &Address,
    ) -> Result<(), CustomError<'a>> {
        let token_data = self
            .data
            .get_mut(pool_address)
            .ok_or_else(|| CustomError::AddressNotFound(*pool_address))?;

        if let Ok(pool) =
            Pool::<EphemeralTickMapDataProvider>::from_pool_key_with_tick_data_provider(
                1,
                FACTORY_ADDRESS,
                token_data.token_a.token.address(),
                token_data.token_b.token.address(),
                token_data.fee(),
                provider,
                None,
            )
            .await
        {
            let tick_data = get_tick_data(&pool).await?;
            token_data.token_a.price_start = pool.token0_price();
            token_data.token_b.price_start = pool.token1_price();
            token_data.tick_data = tick_data;
            token_data.liquidity = pool.liquidity;
            token_data.sqrt_price_x96 = pool.sqrt_ratio_x96;
        }

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
                    &token_data.tick_data.initialised_ticks,
                    &token_data.tick_data.ticks,
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
                    &token_data.tick_data.initialised_ticks,
                    &token_data.tick_data.ticks,
                ) {
                    Ok(a) => a,
                    Err(_err) => {
                        // log::error!("amount0_out calculation failed for {_pool_addr}, due to {_err}");
                        continue;
                    }
                };

                token_data.token_a.price_effective =
                    Price::from_currency_amounts(amount0_in, amount0_out);
                token_data.token_b.price_effective =
                    Price::from_currency_amounts(amount1_out, amount1_in);
            }
        }
        Ok(())
    }

    pub fn calc_slippage<'a>(
        &mut self,
        mut slippage_adj: &mut BigInt,
    ) -> Result<(), CustomError<'a>> {
        for token_data in self.data.values_mut() {
            token_data.token_a.slippage = calc_slippage(
                token_data.token_a.price_start.clone(),
                token_data.token_a.price_effective.clone(),
                &mut slippage_adj,
            )?;

            token_data.token_b.slippage = calc_slippage(
                token_data.token_b.price_start.clone(),
                token_data.token_b.price_effective.clone(),
                &mut slippage_adj,
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
    pub fee: u16,
    pub tick_data: TickData,
    pub liquidity: u128,
    pub sqrt_price_x96: U160,
}

impl TokenData {
    fn new(token_a: Token, token_b: Token, fee: u16) -> Self {
        Self {
            token_a: TokenDetails::new(token_a),
            token_b: TokenDetails::new(token_b),
            fee,
            tick_data: TickData::default(),
            liquidity: u128::default(),
            sqrt_price_x96: U160::default(),
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

async fn get_tick_data<'a>(
    pool: &Pool<EphemeralTickMapDataProvider>,
) -> Result<TickData, CustomError<'a>> {
    let mut initialised_ticks = Vec::new();
    let mut ticks = Vec::new();
    let mut current_state = pool.tick_current;
    loop {
        if let Ok((tick_next, initialized)) = pool
            .tick_data_provider
            .next_initialized_tick_within_one_word(current_state, true, pool.tick_spacing())
            .await
        {
            initialised_ticks.push((tick_next, initialized));
            if let Ok(tick) = pool.tick_data_provider.get_tick(tick_next).await {
                ticks.push(tick);
            } else {
                initialised_ticks.pop();
                break;
            }
            current_state = tick_next - pool.tick_spacing();
        } else {
            break;
        };
    }

    Ok(TickData {
        initialised_ticks,
        ticks,
    })
}
