use super::*;
use uniswap_v3_sdk::prelude::*;

pub mod v2;
pub mod v3;

pub type TokenMap = HashMap<Address, Token>;
pub type PriceData =
    FractionLike<PriceMeta<CurrencyLike<false, TokenMeta>, CurrencyLike<false, TokenMeta>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pools {
    token0: Address,
    token1: Address,
    fee: u16,
    address: Address,
}

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
                token!(1, meta.address, meta.decimals, meta.symbol, meta.name),
            )
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct TokenDetails {
    pub token: Token,
    pub slippage: BigInt,
    pub price_start: PriceData,
    pub price_effective: PriceData,
}

impl Default for TokenDetails {
    fn default() -> Self {
        let def_token = token!(1, address!(), 0);
        let def_price = Price::new(def_token.clone(), def_token.clone(), 1, 0);
        Self {
            token: def_token.clone(),
            slippage: BigInt::ZERO,
            price_start: def_price.clone(),
            price_effective: def_price,
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
