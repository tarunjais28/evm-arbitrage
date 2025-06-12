use super::*;

#[derive(Debug, Error)]
pub enum CustomError<'a> {
    #[error("Hex error: `{0}`!")]
    HexError(#[from] FromHexError),

    #[error("Environment variable error: `{0}`!")]
    EnvVarError(#[from] VarError),

    #[error("Error while getting event by name: `{0}`!")]
    EventNameError(&'a str),

    #[error("Error while getting `{0}`!")]
    NotFound(&'a str),

    #[error("Error while parsing bigInt!")]
    ParseBigIntError(#[from] ParseBigIntError),
}
