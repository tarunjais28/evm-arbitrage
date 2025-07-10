use super::*;

pub struct PoolData {
    pub data: HashMap<Address, TokenData>,
}

impl PoolData {
    pub fn new<'a>(
        serialised_v3_pool: &[SerialisedV3Pools],
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
        let mut count = 0;
        for (addr, token_data) in self.data.iter_mut() {
            let pool =
                match Pool::<EphemeralTickMapDataProvider>::from_pool_key_with_tick_data_provider(
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
                    Ok(p) => p,
                    Err(err) => {
                        count += 1;
                        log::error!(
                            "token0: {}, token1: {}, pool: {}, fee: {}, err: {}",
                            token_data.token_a.token.address(),
                            token_data.token_b.token.address(),
                            addr,
                            token_data.fee,
                            err
                        );
                        continue;
                    }
                };

            token_data.token_a.price_start = pool.token0_price();
            token_data.token_b.price_start = pool.token1_price();
        }

        log::info!("Total skipped: {count}");

        Ok(())
    }

    pub fn calc_start_price_from_sqrt_price_x96<'a>(
        &mut self,
        pool_address: &Address,
        sqrt_price_x96: BigInt,
    ) -> Result<(), CustomError<'a>> {
        debug_time!("v3::calc_start_price_from_sqrt_price_x96", {
            // Numerator: (sqrtPriceX96)^2
            let numerator = sqrt_price_x96 * sqrt_price_x96;

            // Denominator: 2^192 = 1 << 192
            let denominator = BigInt::ONE << 192;

            let token_data = self
                .data
                .get_mut(pool_address)
                .ok_or_else(|| CustomError::AddressNotFound(*pool_address))?;

            let price = Price::new(
                token_data.token_a.token.clone(),
                token_data.token_a.token.clone(),
                denominator,
                numerator,
            );
            token_data.token_a.price_start = price.clone();
            token_data.token_b.price_start = price.invert();
        });

        Ok(())
    }

    pub async fn calc_effective_price<'a>(
        &mut self,
        provider: &FillProvider<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            RootProvider,
        >,
        amount: BigInt,
    ) -> Result<(), CustomError<'a>> {
        for token_data in self.data.values_mut() {
            let pool = Pool::<EphemeralTickMapDataProvider>::from_pool_key_with_tick_data_provider(
                1,
                FACTORY_ADDRESS,
                token_data.token_a.token.address(),
                token_data.token_b.token.address(),
                token_data.fee(),
                provider,
                None,
            )
            .await?;

            let amount0_in =
                CurrencyAmount::from_raw_amount(token_data.token_a.token.clone(), amount)?;
            let amount0_out = pool.get_output_amount(&amount0_in, None).await?;

            let amount1_out =
                CurrencyAmount::from_raw_amount(token_data.token_b.token.clone(), amount)?;
            let amount1_in = pool.get_input_amount(&amount1_out, None).await?;

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
    pub fee: u16,
}

impl TokenData {
    fn new(token_a: Token, token_b: Token, fee: u16) -> Self {
        Self {
            token_a: TokenDetails::new(token_a),
            token_b: TokenDetails::new(token_b),
            fee,
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
