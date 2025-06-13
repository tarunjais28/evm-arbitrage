use super::*;

#[derive(Default, Debug)]
pub struct Output {
    contract_address: Address,
    tx_type: String,
    sender: Address,
    recipient: Address,
    amount0_in: u128,
    amount1_in: u128,
    amount0_out: u128,
    amount1_out: u128,
    reserve0: u128,
    reserve1: u128,
}

impl Output {
    pub fn new(contract_address: Address) -> Self {
        Self {
            contract_address,
            ..Default::default()
        }
    }

    pub fn add_tx_type(&mut self, tx_type: String) {
        self.tx_type = tx_type;
    }

    pub fn update<'a>(&mut self, log: Log) -> Result<(), CustomError<'a>> {
        for param in log.params {
            match param.name.as_str() {
                "sender" => {
                    self.sender = param
                        .value
                        .into_address()
                        .ok_or(CustomError::NotFound("recipient address"))?
                }
                "to" => {
                    self.recipient = param
                        .value
                        .into_address()
                        .ok_or(CustomError::NotFound("recipient address"))?
                }
                "amount0In" => {
                    self.amount0_in = param.value.into_uint().unwrap_or_default().as_u128()
                }
                "amount1In" => {
                    self.amount1_in = param.value.into_uint().unwrap_or_default().as_u128()
                }
                "amount0Out" => {
                    self.amount0_out = param.value.into_uint().unwrap_or_default().as_u128()
                }
                "amount1Out" => {
                    self.amount1_out = param.value.into_uint().unwrap_or_default().as_u128()
                }
                "reserve0" => self.reserve0 = param.value.into_uint().unwrap_or_default().as_u128(),
                "reserve1" => self.reserve1 = param.value.into_uint().unwrap_or_default().as_u128(),
                _ => (),
            }
        }
        Ok(())
    }

    pub fn show(&self) {
        println!("{}", "=".repeat(100).green().bold());
        println!("{}", format!("{}", self).green());
        println!("{}", "=".repeat(100).green().bold());
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Found:\ncontract_address: 0x{:x}\nsender: 0x{:x}\nrecipient: 0x{:x}\namount0_in: {:018}\namount1_in: {:018}\namount0_out: {:018}\namount0_out: {:018}\nreserve0: {:018}\nreserve1: {:018}",
            self.contract_address,
            self.sender,
            self.recipient,
            format_with_decimals(self.amount0_in),
            format_with_decimals(self.amount1_in),
            format_with_decimals(self.amount0_out),
            format_with_decimals(self.amount1_out),
            format_with_decimals(self.reserve0),
            format_with_decimals(self.reserve1)
        )
    }
}
