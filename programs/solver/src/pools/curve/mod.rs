use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePools {
    pub tokens: Vec<Address>,
    pub balances: Vec<U256>,
    pub fee: U256,
    pub a: U256,
    pub address: Address,
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
        let len = cp.tokens.len();
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
            slippage: vec![BigInt::ZERO; len],
        }
    }

    fn calc_slippage(&mut self, i: usize, j: usize, amount: U256) {
        let fee_denomination = BigInt::from(10_000_000_000u128);
        let d = self.get_d();

        let dx = amount.to_big_int();
        let precision = BigInt::from(PRECISION);
        let x = self.xp[i] + (dx * precision / self.precisions[i]);
        let y = self.get_y(i, j, d, x);
        let dy = (self.xp[j] - y - BigInt::ONE) * self.precisions[j] / precision;
        let _fee = self.fee * dy / fee_denomination;
        let dy = dy - _fee;

        self.slippage = vec![calc_slippage(dx, dy, &mut None)];
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
                d_p = d_p * d / (_x * n);
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
            c = c * d / (_x * BigInt::from(n));
        }

        c = c * d / (ann * n_big);
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
}
