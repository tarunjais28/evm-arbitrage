use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolAddress {
    pub v2: Vec<Address>,
    pub v3: Vec<Address>,
}

impl PoolAddress {
    pub fn single(&self) -> Vec<Address> {
        self.v2.iter().chain(self.v3.iter()).cloned().collect()
    }
}

pub struct EnvParser {
    pub ws_address: String,
    pub pool_address: PoolAddress,
}

impl<'a> EnvParser {
    pub fn new() -> Result<Self, CustomError<'a>> {
        dotenv().ok();

        // Open the file with contract addresses
        let file = File::open("resources/pools_combined.json")?;
        let reader = BufReader::new(file);

        Ok(Self {
            ws_address: env::var("WEBSOCKET_ENDPOINT")?,
            pool_address: from_reader(reader)?,
        })
    }
}
