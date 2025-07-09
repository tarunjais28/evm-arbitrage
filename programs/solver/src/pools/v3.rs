use uniswap_v3_sdk::prelude::*;

use super::*;

pub struct PoolData {
    pub data: HashMap<Address, TokenData>,
}

impl PoolData {
    pub fn new<'a>(
        serialised_v3_pool: &[SerialisedV3Pools],
        tokens: TokenMap,
    ) -> Result<PoolData, CustomError<'a>> {
        let mut data = HashMap::with_capacity(tokens.len());

        for pool in serialised_v3_pool {
            data.insert(
                pool.address,
                TokenData::new(
                    tokens
                        .get(&pool.token0)
                        .ok_or_else(move || CustomError::AddressNotFound(pool.token0))?
                        .clone(),
                    tokens
                        .get(&pool.token1)
                        .ok_or_else(move || CustomError::AddressNotFound(pool.token1))?
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

            token_data.token_a.price_start = pool.token0_price().quotient();
            token_data.token_b.price_start = pool.token1_price().quotient();
        }

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

            token_data.token_a.price_start = numerator.checked_div(denominator).unwrap_or_default();
            token_data.token_b.price_start =
                BigInt::from(token_data.token_a.precision() * token_data.token_b.precision())
                    .checked_div(token_data.token_a.price_start)
                    .unwrap_or_default();
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

            let amount0_in = CurrencyAmount::from_fractional_amount(
                token_data.token_a.token.clone(),
                amount,
                token_data.token_a.precision(),
            )?;
            let amount0_out = pool.get_output_amount(&amount0_in, None).await?;

            let amount1_out = CurrencyAmount::from_fractional_amount(
                token_data.token_b.token.clone(),
                amount,
                token_data.token_b.precision(),
            )?;
            let amount1_in = pool.get_input_amount(&amount1_out, None).await?;

            token_data.token_a.price_effective = amount0_out.divide(&amount0_in)?.quotient();
            token_data.token_b.price_effective = amount1_in.divide(&amount1_out)?.quotient();
        }

        Ok(())
    }

    pub fn calc_slippage<'a>(&mut self) -> Result<(), CustomError<'a>> {
        let precision = BigInt::from(10u128.pow(9));

        for token_data in self.data.values_mut() {
            token_data.token_a.slippage = ((token_data.token_a.price_effective
                - token_data.token_a.price_start)
                * precision
                * BigInt::from(100))
            .checked_div(token_data.token_a.price_start)
            .unwrap_or_default();

            token_data.token_b.slippage = ((token_data.token_b.price_effective
                - token_data.token_b.price_start)
                * precision
                * BigInt::from(100))
            .checked_div(token_data.token_b.price_start)
            .unwrap_or_default();
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
pub struct TokenDetails {
    pub token: Token,
    pub slippage: BigInt,
    pub price_start: BigInt,
    pub price_effective: BigInt,
}

impl Default for TokenDetails {
    fn default() -> Self {
        Self {
            token: token!(1, address!(), 0),
            slippage: BigInt::ZERO,
            price_start: BigInt::ZERO,
            price_effective: BigInt::ZERO,
        }
    }
}

impl TokenDetails {
    fn new(token: Token) -> Self {
        Self {
            token,
            ..Default::default()
        }
    }

    fn precision(&self) -> BigInt {
        BigInt::from(10u128.pow(self.token.decimals() as u32))
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
