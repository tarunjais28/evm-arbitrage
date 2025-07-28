use super::*;

pub type TickMap = HashMap<Address, TickData>;

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolAddress {
    pub v2: Vec<Address>,
    pub v3: Vec<Address>,
    pub curve: Vec<Address>,
}

impl PoolAddress {
    pub fn single(&self) -> Vec<Address> {
        self.v2.iter().chain(self.v3.iter()).cloned().collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TickDataReader {
    block: u64,
    pool: Address,
    current_tick: I24,
    sqrt_price_x96: U160,
    liquidity: u128,
    ticks: Vec<TickSync>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TickData {
    pub block: u64,
    pub current_tick: I24,
    pub sqrt_price_x96: U160,
    pub liquidity: u128,
    pub ticks: Vec<TickSync>,
}

impl From<TickDataReader> for TickData {
    fn from(tdr: TickDataReader) -> Self {
        Self {
            block: tdr.block,
            current_tick: tdr.current_tick,
            sqrt_price_x96: tdr.sqrt_price_x96,
            liquidity: tdr.liquidity,
            ticks: tdr.ticks,
        }
    }
}

pub struct EnvParser {
    pub ws_address: String,
    pub pool_address: PoolAddress,
    pub token_metadata: Vec<TokenMetadata>,
    pub pools_v2: Vec<Pools>,
    pub pools_v3: Vec<Pools>,
    pub curve_pools: Vec<CurvePools>,
    pub tick_map: TickMap,
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

        // Open ticks file
        let curve_pools_file = File::open(env::var("CURVE_TOKENS_PATH")?)?;
        let curve_pools_reader = BufReader::new(curve_pools_file);

        // Open ticks file
        let ticks_file = File::open(env::var("TICKS_PATH")?)?;
        let ticks_reader = BufReader::new(ticks_file);
        let tick_data_reader: Vec<TickDataReader> = from_reader(ticks_reader)?;

        Ok(Self {
            ws_address: env::var("WEBSOCKET_ENDPOINT")?,
            pool_address: from_reader(pool_reader)?,
            pools_v2: from_reader(pools_v2_reader)?,
            token_metadata: from_reader(metadata_reader)?,
            pools_v3: from_reader(pools_v3_reader)?,
            curve_pools: from_reader(curve_pools_reader)?,
            tick_map: tick_data_reader
                .iter()
                .map(|tdr| (tdr.pool, TickData::from(tdr.clone())))
                .collect(),
        })
    }
}
