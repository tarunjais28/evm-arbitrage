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
    pub token_metadata: Vec<TokenMetadata>,
    pub pools_v2: Vec<Pools>,
    pub pools_v3: Vec<Pools>,
}

impl<'a> EnvParser {
    pub fn new() -> Result<Self, CustomError<'a>> {
        dotenv().ok();

        // Open the file with pool addresses
        let pool_file = File::open(env::var("POOL_PATH")?)?;
        let pool_reader = BufReader::new(pool_file);

        // Open the file with v2 pool addresses
        let pools_v2_file = File::open(env::var("POOLS_V2_PATH")?)?;
        let pools_v2_reader = BufReader::new(pools_v2_file);

        // Open the file with token metadata
        let metadata_file = File::open(env::var("METADATA_FILE_PATH")?)?;
        let metadata_reader = BufReader::new(metadata_file);

        // Open the file with serialised v3 pool addresses
        let pools_v3_file = File::open(env::var("POOLS_V3_PATH")?)?;
        let pools_v3_reader = BufReader::new(pools_v3_file);

        Ok(Self {
            ws_address: env::var("WEBSOCKET_ENDPOINT")?,
            pool_address: from_reader(pool_reader)?,
            pools_v2: from_reader(pools_v2_reader)?,
            token_metadata: from_reader(metadata_reader)?,
            pools_v3: from_reader(pools_v3_reader)?,
        })
    }
}
