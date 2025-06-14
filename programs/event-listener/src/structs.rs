use super::*;

#[derive(Default, Debug)]
pub struct Output {
    contract_address: Address,
    tx_type: TxType,
    sender: Address,
    to: Address,
    amount0: u128,
    amount1: u128,
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

    pub fn update_tx_type(&mut self, tx_type: TxType) {
        self.tx_type = tx_type
    }

    pub fn clear(&mut self) -> Self {
        Self {
            contract_address: self.contract_address,
            ..Default::default()
        }
    }

    pub fn update<'a>(&mut self, log: Log) -> Result<(), CustomError<'a>> {
        for param in log.params {
            match param.name.as_str() {
                "sender" => {
                    self.sender = param
                        .value
                        .into_address()
                        .ok_or(CustomError::NotFound("to address"))?
                }
                "to" => {
                    self.to = param
                        .value
                        .into_address()
                        .ok_or(CustomError::NotFound("to address"))?
                }
                "amount0" => self.amount0 = param.value.into_uint().unwrap_or_default().as_u128(),
                "amount1" => self.amount1 = param.value.into_uint().unwrap_or_default().as_u128(),
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
        use TxType::*;

        let mut output = format!("{}", self);
        match self.tx_type {
            Add => {
                output.push_str(&format!(
                    "sender: 0x{:x}\namount0: {:018}\namount1: {:018}",
                    self.sender,
                    format_with_decimals(self.amount0),
                    format_with_decimals(self.amount1)
                ));
                println!("{}", format!("{}", output).green());
                println!("{}", "=".repeat(70).green().bold());
            }
            Remove => {
                output.push_str(&format!(
                    "sender: 0x{:x}\nto: 0x{:x}\namount0: {:018}\namount1: {:018}",
                    self.sender,
                    self.to,
                    format_with_decimals(self.amount0),
                    format_with_decimals(self.amount1)
                ));
                println!("{}", format!("{}", output).purple());
                println!("{}", "=".repeat(70).purple().bold());
            }
            Swap => {
                output.push_str(&format!(
                "sender: 0x{:x}\nto: 0x{:x}\namount0_in: {:018}\namount1_in: {:018}\namount0_out: {:018}\namount1_out: {:018}",
                self.sender,
                self.to,
                format_with_decimals(self.amount0_in),
                format_with_decimals(self.amount1_in),
                format_with_decimals(self.amount0_out),
                format_with_decimals(self.amount1_out)
            ));
                println!("{}", format!("{}", output).yellow());
                println!("{}", "=".repeat(70).yellow().bold());
            }
            Sync => (),
        };
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "contract_address: 0x{:x}\ntx_type: {:?}\nreserve0: {:018}\nreserve1: {:018}\n",
            self.contract_address,
            self.tx_type,
            format_with_decimals(self.reserve0),
            format_with_decimals(self.reserve1)
        )
    }
}
