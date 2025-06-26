use super::*;

pub struct EnvParser {
    pub ws_address: String,
    pub pools: Vec<H160>,
}

impl<'a> EnvParser {
    pub fn new() -> Result<Self, CustomError<'a>> {
        dotenv().ok();

        // Open the file with contract addresses
        let file = File::open("resources/pools.txt")?;
        let reader = BufReader::new(file);

        // Parse and decode addresses
        let mut pools = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let address = H160::from_slice(&hex::decode(trimmed).unwrap());
            pools.push(address);
        }

        Ok(Self {
            ws_address: env::var("WEBSOCKET_ENDPOINT")?,
            pools,
        })
    }
}
