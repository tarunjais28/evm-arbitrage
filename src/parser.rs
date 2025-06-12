use super::*;

pub struct EnvParser {
    pub ws_address: String,
    pub contract_address: String,
}

impl<'a> EnvParser {
    pub fn new() -> Result<Self, CustomError<'a>> {
        Ok(Self {
            ws_address: env::var("WEBSOCKET_INFURA_ENDPOINT")?,
            contract_address: env::var("CONTRACT_ADDRESS")?,
        })
    }
}
