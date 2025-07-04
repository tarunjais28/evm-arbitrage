use super::*;

pub type PoolData = HashMap<Address, TokenData>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pools {
    token_a: Address,
    token_b: Address,
    pub address: Address,
}

impl Pools {
    pub fn to_key_value(&self) -> (Address, TokenData) {
        (
            self.address,
            TokenData {
                token_a: self.token_a,
                token_b: self.token_b,
                reserve0: U112::ZERO,
                reserve1: U112::ZERO,
                price: U112::ZERO,
            },
        )
    }
}

#[derive(Debug)]
pub struct TokenData {
    pub token_a: Address,
    pub token_b: Address,
    pub reserve0: U112,
    pub reserve1: U112,
    pub price: U112,
}

#[derive(Debug)]
pub struct SearchData {
    block_number: u64,
    pool: Address,
    token_a: Address,
    token_b: Address,
    reserve0: U112,
    reserve1: U112,
    price: U112,
}

impl SearchData {
    pub fn new(
        block_number: u64,
        pool: Address,
        token_a: Address,
        token_b: Address,
        reserve0: U112,
        reserve1: U112,
    ) -> Self {
        Self {
            block_number,
            pool,
            token_a,
            token_b,
            reserve0,
            reserve1,
            price: reserve0 * U112::from(1000_000_000_000_000_000u64)
                / (reserve1 * U112::from(1000_000u64)),
        }
    }

    pub fn headers() -> String {
        String::from("Block Number, Pool Address, TokenA, TokenB, Reserve0, Reserve1, Price\n")
    }
}

impl Display for SearchData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{},{},{},{},{}\n",
            self.block_number,
            self.pool,
            self.token_a,
            self.token_b,
            self.reserve0,
            self.reserve1,
            self.price
        )
    }
}

impl TokenData {
    pub fn update_reserves(&mut self, reserves: Reserves) {
        self.reserve0 = reserves.reserve0;
        self.reserve1 = reserves.reserve1;
        self.price = reserves.reserve0 * U112::from(1000_000_000_000_000_000u64)
            / (reserves.reserve1 * U112::from(1000_000u64));
    }
}
