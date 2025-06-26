use super::*;

pub struct EnvParser {
    pub ws_address: String,
    pub pools: Vec<H160>,
    pub pools_addrs: Vec<Address>,
}

impl<'a> EnvParser {
    pub fn new() -> Result<Self, CustomError<'a>> {
        dotenv().ok();

        // Open the file with contract addresses
        let file = File::open("resources/pools.txt")?;
        let reader = BufReader::new(file);

        // Parse and decode addresses
        let mut pools = Vec::new();
        let mut pools_addrs = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let address = H160::from_slice(&hex::decode(trimmed)?);
            pools.push(address);

            let addr = Address::from_str(trimmed)?;
            pools_addrs.push(addr);
        }

        Ok(Self {
            ws_address: env::var("WEBSOCKET_ENDPOINT")?,
            pools,
            pools_addrs,
        })
    }
}
