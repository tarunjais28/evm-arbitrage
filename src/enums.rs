#[derive(Debug)]
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
