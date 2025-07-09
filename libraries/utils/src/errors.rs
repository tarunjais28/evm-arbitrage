use super::*;

#[derive(Debug, Error)]
pub enum CustomError<'a> {
    #[error("Hex error: `{0}`!")]
    HexError(#[from] hex::FromHexError),

    #[error("Hex error: `{0}`!")]
    HexErrorAlloy(#[from] alloy::primitives::hex::FromHexError),

    #[error("Transport Error: `{0}`!")]
    TransportError(#[from] RpcError<TransportErrorKind>),

    #[error("Contract Error: `{0}`!")]
    ContractError(#[from] contract::Error),

    #[error("Environment variable error: `{0}`!")]
    EnvVarError(#[from] VarError),

    #[error("Uniswap v2 sdk error: `{0}`!")]
    UniswapV2SdkError(#[from] uniswap_v2_sdk::error::Error),

    #[error("Uniswap v3 sdk error: `{0}`!")]
    UniswapV3SdkError(#[from] uniswap_v3_sdk::error::Error),

    #[error("Uniswap sdk core error: `{0}`!")]
    UniswapSdkCoreError(#[from] uniswap_sdk_core::error::Error),

    #[error("Json serialisation failed: `{0}`!")]
    JsonParseError(#[from] serde_json::Error),

    #[error("IO error: `{0}`!")]
    IoError(#[from] io::Error),

    #[error("IO error: `{0}`!")]
    EthAbiError(#[from] web3::ethabi::Error),

    #[error("Error while getting `{0}`!")]
    NotFound(&'a str),

    #[error("Error while getting address: `{0}`!")]
    AddressNotFound(Address),

    #[error("Error while parsing bigInt!")]
    ParseBigIntError(#[from] ParseBigIntError),
}
