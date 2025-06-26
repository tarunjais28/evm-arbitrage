use super::*;

#[derive(Debug, Clone, Copy)]
pub enum TxType {
    Add,
    Remove,
    Swap,
    Sync,
}

impl Default for TxType {
    fn default() -> Self {
        Self::Sync
    }
}

#[derive(Debug)]
pub enum EventType {
    Swap(Swap),
    Sync(Sync),
    Mint(Mint),
    Burn(Burn),
}