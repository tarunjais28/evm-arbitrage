use super::*;

pub struct EnvParser {
    pub ws_address: String,
    pub pools_addrs: Vec<Address>,
}

impl<'a> EnvParser {
    pub fn new() -> Result<Self, CustomError<'a>> {
        dotenv().ok();

        // Open the file with contract addresses
        let file = File::open("resources/pools.json")?;
        let reader = BufReader::new(file);

        // Parse and decode addresses
        let addresses: Vec<String> = from_reader(reader)?;

        let pools_addrs: Vec<Address> = addresses
            .iter()
            .map(|s| s.parse())
            .collect::<Result<_, _>>()?;

        Ok(Self {
            ws_address: env::var("WEBSOCKET_ENDPOINT")?,
            pools_addrs,
        })
    }
}
