use super::*;
use IUniswapV2Pool::{Burn, Mint, Swap, Sync};

pub struct EventData {
    tx_type: TxType,
    sender: Address,
    amount0: U256,
    amount1: U256,
}

impl From<Mint> for EventData {
    fn from(m: Mint) -> Self {
        Self {
            tx_type: TxType::Add,
            sender: m.sender,
            amount0: m.amount0,
            amount1: m.amount1,
        }
    }
}

impl From<Burn> for EventData {
    fn from(b: Burn) -> Self {
        Self {
            tx_type: TxType::Remove,
            sender: b.sender,
            amount0: b.amount0,
            amount1: b.amount1,
        }
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ScanData {
    tx_hash: TxHash,
    block_number: u64,
    pub pool_address: Address,
    pub tx_type: TxType,
    sender: Address,
    to: Address,
    amount0: U256,
    amount1: U256,
    amount0_in: U256,
    amount1_in: U256,
    amount0_out: U256,
    amount1_out: U256,
    pub reserve0: U256,
    pub reserve1: U256,
}

impl From<ScanData> for Reserves {
    fn from(res: ScanData) -> Self {
        Self {
            reserve0: res.reserve0,
            reserve1: res.reserve1,
        }
    }
}

impl ScanData {
    pub fn new(log: &Log) -> Self {
        Self {
            tx_hash: log.transaction_hash.unwrap_or_default(),
            block_number: log.block_number.unwrap_or_default(),
            ..Default::default()
        }
    }

    pub fn update_swap(&mut self, swap: Swap, pool_address: Address) {
        self.tx_type = TxType::Swap;
        self.pool_address = pool_address;
        self.sender = swap.sender;
        self.to = swap.to;
        self.amount0_in = swap.amount0In;
        self.amount1_in = swap.amount1In;
        self.amount0_out = swap.amount0Out;
        self.amount1_out = swap.amount1Out;
    }

    pub fn update_sync(&mut self, sync: Sync, pool_address: Address) {
        self.pool_address = pool_address;
        self.reserve0 = U256::from(sync.reserve0);
        self.reserve1 = U256::from(sync.reserve1);
    }

    pub fn update_liquidity_events(&mut self, event: impl Into<EventData>, pool_address: Address) {
        let e = event.into();
        self.tx_type = e.tx_type;
        self.sender = e.sender;
        self.amount0 = e.amount0;
        self.amount1 = e.amount1;

        self.pool_address = pool_address;
    }

    pub fn show(&self) {
        use TxType::*;

        let mut output = format!("{}", self);
        match self.tx_type {
            Add => {
                output.push_str(&format!(
                    "sender: {}\namount0: {:018}\namount1: {:018}",
                    self.sender, self.amount0, self.amount1
                ));
                println!("{}", output.to_string().green());
                println!("{}", "=".repeat(70).green().bold());
            }
            Remove => {
                output.push_str(&format!(
                    "sender: {}\nto: {}\namount0: {:018}\namount1: {:018}",
                    self.sender, self.to, self.amount0, self.amount1
                ));
                println!("{}", output.to_string().purple());
                println!("{}", "=".repeat(70).purple().bold());
            }
            Swap => {
                output.push_str(&format!(
                "sender: {}\nto: {}\namount0_in: {:018}\namount1_in: {:018}\namount0_out: {:018}\namount1_out: {:018}",
                self.sender,
                self.to,
                self.amount0_in,
                self.amount1_in,
                self.amount0_out,
                self.amount1_out
            ));
                println!("{}", output.to_string().yellow());
                println!("{}", "=".repeat(70).yellow().bold());
            }
            Sync => {
                output.push_str(&format!(
                    "reserve0: {:018}\nreserve1: {:018}",
                    self.reserve0, self.reserve1
                ));
                println!("{}", output.to_string().magenta());
                println!("{}", "=".repeat(70).magenta().bold());
            }
        };
    }
}

impl Display for ScanData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "txHash: {}\nblockNumber: {}\npool_address: {}\ntx_type: {:?}\n",
            self.tx_hash, self.block_number, self.pool_address, self.tx_type,
        )
    }
}
