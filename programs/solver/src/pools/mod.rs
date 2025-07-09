use super::*;
use uniswap_v3_sdk::prelude::*;

pub mod v2;
pub mod v3;

pub type TokenMap = HashMap<Address, Token>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenMetadata {
    pub address: Address,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

pub fn token_metadata_to_tokens(token_metadata: &[TokenMetadata]) -> TokenMap {
    token_metadata
        .iter()
        .map(|meta| {
            (
                meta.address,
                token!(1, meta.address, meta.decimals, meta.name, meta.symbol),
            )
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialisedV3Pools {
    pub token0: Address,
    pub token1: Address,
    pub fee: u16,
    pub address: Address,
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
