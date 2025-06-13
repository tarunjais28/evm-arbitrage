use super::*;

#[derive(Default, Debug)]
pub struct Output {
    contract_address: Address,
    tx_type: TxType,
    sender: Address,
    recipient: Address,
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

    pub fn update<'a>(&mut self, log: Log, tx_type: Option<TxType>) -> Result<(), CustomError<'a>> {
        if let Some(typ) = tx_type {
            self.tx_type = typ;
        }

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
        println!("{}", format!("{}", self).green());
        println!("{}", "=".repeat(100).green().bold());
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Found:\ncontract_address: 0x{:x}\ntx_type: {:?}\nsender: 0x{:x}\nrecipient: 0x{:x}\namount0: {:018}\namount1: {:018}\namount0_in: {:018}\namount1_in: {:018}\namount0_out: {:018}\namount0_out: {:018}\nreserve0: {:018}\nreserve1: {:018}",
            self.contract_address,
            self.tx_type,
            self.sender,
            self.recipient,
            format_with_decimals(self.amount0),
            format_with_decimals(self.amount1),
            format_with_decimals(self.amount0_in),
            format_with_decimals(self.amount1_in),
            format_with_decimals(self.amount0_out),
            format_with_decimals(self.amount1_out),
            format_with_decimals(self.reserve0),
            format_with_decimals(self.reserve1)
        )
    }
}
