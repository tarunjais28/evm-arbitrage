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
    pub serialised_v3_pool: Vec<SerialisedV3Pools>,
}

impl<'a> EnvParser {
    pub fn new() -> Result<Self, CustomError<'a>> {
        dotenv().ok();

        // Open the file with pool addresses
        let pool_file = File::open(env::var("POOL_PATH")?)?;
        let pool_reader = BufReader::new(pool_file);

        // Open the file with pool addresses
        let metadata_file = File::open(env::var("METADATA_FILE_PATH")?)?;
        let metadata_reader = BufReader::new(metadata_file);

        // Open the file with pool addresses
        let seialised_v3_pool_file = File::open(env::var("SERIALISED_V3_POOL_FILE_PATH")?)?;
        let seialised_v3_pool_reader = BufReader::new(seialised_v3_pool_file);

        Ok(Self {
            ws_address: env::var("WEBSOCKET_ENDPOINT")?,
            pool_address: from_reader(pool_reader)?,
            token_metadata: from_reader(metadata_reader)?,
            serialised_v3_pool: from_reader(seialised_v3_pool_reader)?,
        })
    }
}
