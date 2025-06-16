use hex::FromHex;
use std::{collections::HashMap, fs::File, io::BufReader};
use web3::{
    ethabi::{Address, Contract, Function},
};

#[derive(Default, Debug)]
pub struct Output {
    name: String,
    amount_0: u128,
    amount_1: u128,
    path: Vec<Address>,
    to: Address,
    deadline: u128,
}

impl Output {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    fn display(&self, names: &[&str]) {
        if names.len() == 1 {
            println!(
                "name: {}\n{}: {}\npath: {:?}\nto: {:?}\ndeadline: {}\n{}",
                self.name, names[0], self.amount_0, self.path, self.to, self.deadline, "=".repeat(70)
            );
        } else {
            println!(
                "name: {}\n{}: {}\n{}: {}\npath: {:?}\nto: {:?}\ndeadline: {}\n{}",
                self.name,
                names[0],
                self.amount_0,
                names[0],
                self.amount_1,
                self.path,
                self.to,
                self.deadline,
                "=".repeat(70)
            );
        }
    }
}

fn main() -> anyhow::Result<()> {
    // Transaction input data (example: swapExactETHForTokensSupportingFeeOnTransferTokens)
    let tx_input_hex = "0x7ff36ab50000000000000000000000000000000000000000000275fec7210dff52e43618000000000000000000000000000000000000000000000000000000000000008000000000000000000000000053be0ca92cd62aa7e8d4ad949236c09021de88a000000000000000000000000000000000000000000000000000000000684d54900000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000005e0b256bf770762f42874c278b5fabd772d182e8";
    // let tx_input_hex = "0x84d61c970000000000000000000000000000000000000000000000000000000000000060000000000000000000000000b8f275fbf7a959f4bce59999a2ef122a099e81a800000000000000000000000000000000000000000000000000000000000003200000000000000000000000000000000000000000000000000000000000000284c23a4c88000000000000000000000000000000000000000000000000000000000007a1206f88d6a3d1fa5842348ff59c7eeb9a0f470019685324cc4624e4a7c403e93a4b7d15fa38849d052f8870a857394a1bf329422c392e1dd87b9c574728885409f6000000000000000000000000643818353783e90e8b62cabd983103d69ce032210000000000000000000000000000000000000000000000000000000000132c44000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb480000000000000000000000007a250d5630b4cf539739df2c5dacb4c659f2488d00000000000000000000000000000000000000000000000000000000000001200000000000000000000000000000000000000000000000000000000000000024000000000000000000000000000000000000000000000000000000000000012438ed173900000000000000000000000000000000000000000000000000000000000b8b24000000000000000000000000000000000000000000000000169ffc1e0be099b500000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000643818353783e90e8b62cabd983103d69ce0322100000000000000000000000000000000000000000000000000000000d0a4c5d30000000000000000000000000000000000000000000000000000000000000003000000000000000000000000a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2000000000000000000000000f57e7e7c23978c3caec3c3548e3d615c346e79ff00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000041dbcdaf1925ec61a7f103a6e753c7a83820d6a7f6ed5ac92b280155087fcc61d367d1d34151bc866192985c3868e8f1b43ef4898df375a3b2f99ecdbf9450d0b11b00000000000000000000000000000000000000000000000000000000000000";
    let file = File::open("programs/identifier/resources/uniswap_routerv2_abi.json")?;
    let reader = BufReader::new(file);

    let contract = Contract::load(reader)?;

    let func_names = [
        "swapETHForExactTokens",
        "swapExactETHForTokens",
        "swapExactETHForTokensSupportingFeeOnTransferTokens",
        "swapExactTokensForETH",
        "swapExactTokensForETHSupportingFeeOnTransferTokens",
        "swapExactTokensForTokens",
        "swapExactTokensForTokensSupportingFeeOnTransferTokens",
        "swapTokensForExactETH",
        "swapTokensForExactTokens",
    ];

    let funcs: Vec<_> = func_names
        .iter()
        .map(|name| contract.function(name))
        .collect::<Result<_, _>>()?;

    let identifiers: Vec<_> = funcs
        .iter()
        .map(|func| hex::encode(&func.short_signature()))
        .collect();

    // Convert hex string to bytes
    let tx_clean = tx_input_hex.trim_start_matches("0x");
    let data = Vec::from_hex(&tx_clean)?;

    // Drop the selector (first 4 bytes)
    let payload = &data[4..];
    let selector = tx_input_hex.to_string()[2..10].to_string();

    let mut fn_map: HashMap<String, &Function> = HashMap::new();
    let mut names_map: HashMap<String, &[&str]> = HashMap::new();
    for idx in 0..8 {
        fn_map.insert(identifiers[idx].clone(), funcs[idx]);
    }

    names_map.insert(identifiers[0].clone(), &["amount_out"]);
    names_map.insert(identifiers[1].clone(), &["amount_out_min"]);
    names_map.insert(identifiers[0].clone(), &["amount_out_min"]);
    names_map.insert(identifiers[0].clone(), &["amount_in", "amount_out_min"]);
    names_map.insert(identifiers[0].clone(), &["amount_in", "amount_out_min"]);
    names_map.insert(identifiers[0].clone(), &["amount_in", "amount_out_min"]);
    names_map.insert(identifiers[0].clone(), &["amount_in", "amount_out_min"]);
    names_map.insert(identifiers[0].clone(), &["amount_out", "amount_in_max"]);
    names_map.insert(identifiers[0].clone(), &["amount_out", "amount_in_max"]);

    let mut generator: HashMap<String, fn(&Function, &[u8], &[&str]) -> Result<(), anyhow::Error>> =
        HashMap::new();

    generator.insert(identifiers[0].clone(), swap_eth_for_exact_tokens);
    generator.insert(identifiers[1].clone(), swap_eth_for_exact_tokens);
    generator.insert(identifiers[2].clone(), swap_eth_for_exact_tokens);
    generator.insert(identifiers[3].clone(), swap_exact_tokens_for_eth);
    generator.insert(identifiers[4].clone(), swap_exact_tokens_for_eth);
    generator.insert(identifiers[5].clone(), swap_exact_tokens_for_eth);
    generator.insert(identifiers[6].clone(), swap_exact_tokens_for_eth);
    generator.insert(identifiers[7].clone(), swap_exact_tokens_for_eth);
    generator.insert(identifiers[8].clone(), swap_exact_tokens_for_eth);

    if let Some(idx) = identifiers.iter().position(|x| selector.eq(x)) {
        if let Some(val) = generator.get(&selector) {
            let names = names_map.get(&identifiers[idx]).unwrap();
            val(funcs[idx], &payload, names)?;
        }
    }

    Ok(())
}

fn swap_eth_for_exact_tokens<'a>(
    func: &Function,
    input: &[u8],
    names: &[&str],
) -> Result<(), anyhow::Error> {
    let mut out = Output::new();

    let decoded = func.decode_input(input).unwrap();
    out.name = func.name.to_string();
    out.amount_0 = decoded[0].clone().into_uint().unwrap_or_default().as_u128();

    decoded[1]
        .clone()
        .into_array()
        .unwrap_or_default()
        .iter()
        .for_each(|address| {
            out.path
                .push(address.clone().into_address().unwrap_or_default())
        });

    out.to = decoded[2].clone().into_address().unwrap_or_default();
    out.to = decoded[3].clone().into_address().unwrap_or_default();

    out.display(names);

    Ok(())
}

fn swap_exact_tokens_for_eth<'a>(
    func: &Function,
    input: &[u8],
    names: &[&str],
) -> Result<(), anyhow::Error> {
    let mut out = Output::new();

    let decoded = func.decode_input(input)?;
    out.amount_0 = decoded[0].clone().into_uint().unwrap_or_default().as_u128();
    out.amount_1 = decoded[1].clone().into_uint().unwrap_or_default().as_u128();

    decoded[2]
        .clone()
        .into_array()
        .unwrap_or_default()
        .iter()
        .for_each(|address| {
            out.path
                .push(address.clone().into_address().unwrap_or_default())
        });

    out.to = decoded[3].clone().into_address().unwrap_or_default();

    out.display(names);

    Ok(())
}
