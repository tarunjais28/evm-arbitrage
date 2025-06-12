use super::*;

pub struct EnvParser {
    pub ws_address: String,
    pub contract_addresses: Vec<H160>,
}

impl<'a> EnvParser {
    pub fn new() -> Result<Self, CustomError<'a>> {
        Ok(Self {
            ws_address: env::var("WEBSOCKET_INFURA_ENDPOINT")?,
            contract_addresses: env::var("CONTRACT_ADDRESS")?
                .split(',')
                .map(|s| web3::types::H160::from_slice(&hex::decode(s.trim()).unwrap()))
                .collect(),
        })
    }
}
