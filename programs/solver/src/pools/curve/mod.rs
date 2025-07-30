use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePools {
    pub tokens: Vec<Address>,
    pub balances: Vec<U256>,
    pub fee: U256,
    pub a: U256,
    pub address: Address,
}

impl CurvePools {
    pub async fn fetch_balances<'a>(provider: &SolverProvider, pools: &mut Vec<CurvePools>) {
        let provider = Arc::new(provider.clone());

        let mut tasks = FuturesUnordered::new();

        for pool in pools.iter_mut() {
            let provider = Arc::clone(&provider);
            let pool_ptr: *mut CurvePools = pool;

            tasks.push(async move {
                let pool = unsafe { &mut *pool_ptr };
                let contract = CurvePool::new(pool.address, provider.as_ref().clone());
                let contract_1 = CurvePool1::new(pool.address, provider.as_ref().clone());
                let mut multicall = provider.multicall().dynamic();

                for i in 0..pool.tokens.len() {
                    multicall = multicall.add_dynamic(contract.balances(U256::from(i)));
                }

                if let Ok(bals) = multicall.aggregate().await {
                    pool.balances = bals;
                } else {
                    let mut multicall = provider.multicall().dynamic();

                    for i in 0..pool.tokens.len() {
                        multicall = multicall.add_dynamic(contract_1.balances(i as i128));
                    }

                    if let Ok(bals) = multicall.aggregate().await {
                        pool.balances = bals;
                    }
                }
            });
        }

        while tasks.next().await.is_some() {}
    }
}

#[derive(Debug, Clone)]
pub struct TokenData {
    pub tokens: Vec<Address>,
    pub xp: Vec<BigInt>,
    pub precisions: Vec<BigInt>,
    pub fee: BigInt,
    pub a: BigInt,
    pub slippage: Vec<BigInt>,
}

impl TokenData {
    fn new(cp: CurvePools, precisions: Vec<BigInt>) -> Self {
        Self {
            tokens: cp.tokens,
            xp: cp
                .balances
                .iter()
                .enumerate()
                .map(|(i, b)| b.to_big_int() * BigInt::from(PRECISION) / precisions[i])
                .collect(),
            precisions,
            fee: cp.fee.to_big_int(),
            a: cp.a.to_big_int(),
            slippage: Vec::default(),
        }
    }

    fn calc_slippage(&mut self, dx: BigInt) {
        let fee_denomination = BigInt::from(10_000_000_000u128);
        let d = self.get_d();

        let precision = BigInt::from(PRECISION);
        let n = self.tokens.len();

        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let x = self.xp[i] + (dx * precision / self.precisions[i]);
                    let y = self.get_y(i, j, d, x);
                    let dy = (self.xp[j] - y - BigInt::ONE) * self.precisions[j] / precision;
                    let _fee = self.fee * dy / fee_denomination;
                    let dy = dy - _fee;

                    self.slippage.push(calc_slippage(dx, dy, &mut None));
                }
            }
        }
    }

    fn get_d(&self) -> BigInt {
        let n = self.tokens.len();
        let ann = self.a * BigInt::from(n);
        let ann_1 = ann - BigInt::ONE;
        let s = self.xp.iter().sum::<BigInt>();
        let mut d = s;
        let n = BigInt::from(n);
        let n_1 = n + BigInt::ONE;

        if s.is_zero() {
            return BigInt::default();
        }

        let mut d_prev;
        let mut d_p;
        for _ in 0..255 {
            d_p = d;
            for _x in self.xp.clone() {
                // TODO: Handle divide by 0
                // If division by 0, this will be borked: only withdrawal will work. And that is good
                d_p *= d / (_x * n);
            }
            d_prev = d;
            d = (((ann * s) + (n * d_p)) * d) / ((ann_1 * d) + (n_1 * d_p));

            if is_abs_le_1(&d, &d_prev) {
                break;
            }
        }

        d
    }

    fn get_y(&self, i: usize, j: usize, d: BigInt, x: BigInt) -> BigInt {
        let n = self.tokens.len();
        let ann = self.a * BigInt::from(n);
        let mut c = d;
        let mut s_ = BigInt::default();
        let mut _x = BigInt::default();
        let n_big = BigInt::from(n);

        for _i in 0..n {
            if _i == i {
                _x = x;
            } else if _i != j {
                _x = self.xp[_i];
            } else {
                continue;
            }
            s_ += _x;
            c *= d / (_x * BigInt::from(n));
        }

        c *= d / (ann * n_big);
        let b = s_ + d / ann;
        let mut y_prev;
        let mut y = d;

        for _i in 0..255 {
            y_prev = y;
            y = ((y * y) + c) / ((y * BigInt::from(2)) + b - d);

            if is_abs_le_1(&y_prev, &y) {
                break;
            }
        }

        y
    }
}

#[derive(Debug)]
pub struct PoolData {
    pub data: HashMap<Address, TokenData>,
}

impl PoolData {
    pub fn new<'a>(pools: &[CurvePools], tokens: &TokenMap) -> Result<PoolData, CustomError<'a>> {
        let mut data = HashMap::with_capacity(pools.len());

        for pool in pools {
            let precisions: Result<Vec<BigInt>, CustomError> = pool
                .tokens
                .iter()
                .map(|addr| {
                    let token = tokens
                        .get(addr)
                        .ok_or_else(|| CustomError::AddressNotFound(*addr))?;
                    Ok(BigInt::from(10u128.pow(u32::from(token.decimals))))
                })
                .collect();

            data.insert(pool.address, TokenData::new(pool.clone(), precisions?));
        }

        Ok(PoolData { data })
    }

    pub fn calc_slippage(&mut self, amount: BigInt) {
        for token_data in self.data.values_mut() {
            token_data.calc_slippage(amount);
        }
    }

    pub fn to_swap_graph(&self, graph: &mut SwapGraph) {
        for (pool, token_data) in self.data.iter() {
            let n = token_data.tokens.len();

            let mut k = 0;
            for i in 0..n {
                for j in 0..n {
                    if i != j {
                        let from = token_data.tokens[i];
                        let to = token_data.tokens[j];
                        let slippage = token_data.slippage[k];

                        graph
                            .entry(from)
                            .or_default()
                            .push(SwapEdge::new(to, *pool, slippage, 0));

                        k += 1;
                    }
                }
            }
        }
    }
}
