use super::*;

pub fn swap_eth_for_exact_tokens<'a>(
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
    out.deadline = decoded[3].clone().into_uint().unwrap_or_default().as_u128();

    out.display(names);

    Ok(())
}

pub fn swap_exact_tokens_for_eth<'a>(
    func: &Function,
    input: &[u8],
    names: &[&str],
) -> Result<(), anyhow::Error> {
    let mut out = Output::new();

    let decoded = func.decode_input(input)?;
    out.name = func.name.to_string();
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
    out.deadline = decoded[4].clone().into_uint().unwrap_or_default().as_u128();

    out.display(names);

    Ok(())
}

pub fn generate_maps<'a>(
    contract: &'a Contract,
) -> anyhow::Result<(
    Vec<String>,
    Vec<&'a Function>,
    HashMap<String, fn(&Function, &[u8], &[&str]) -> Result<(), anyhow::Error>>,
    HashMap<String, &'a[&'a str]>,
)> {
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
        .map(|name| contract.function(&name.to_string()))
        .collect::<Result<_, _>>()?;

    let identifiers: Vec<_> = funcs
        .iter()
        .map(|func| hex::encode(&func.short_signature()))
        .collect();

    let mut names_map: HashMap<String, &[&str]> = HashMap::new();

    names_map.insert(identifiers[0].clone(), &["amount_out"]);
    names_map.insert(identifiers[1].clone(), &["amount_out_min"]);
    names_map.insert(identifiers[2].clone(), &["amount_out_min"]);
    names_map.insert(identifiers[3].clone(), &["amount_in", "amount_out_min"]);
    names_map.insert(identifiers[4].clone(), &["amount_in", "amount_out_min"]);
    names_map.insert(identifiers[5].clone(), &["amount_in", "amount_out_min"]);
    names_map.insert(identifiers[6].clone(), &["amount_in", "amount_out_min"]);
    names_map.insert(identifiers[7].clone(), &["amount_out", "amount_in_max"]);
    names_map.insert(identifiers[8].clone(), &["amount_out", "amount_in_max"]);

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

    Ok((identifiers, funcs, generator, names_map))
}

fn identify_uniswap_v2_function<'a>(
    input: &web3::types::Bytes,
    to: &Option<H160>,
    addresses: &[H160],
    contract: &Contract,
) -> Result<(), CustomError<'a>> {
    if input.0.len() < 4 {
        return Err(CustomError::NotFound("ignore".into()));
    }

    let swap = contract.function("swap").unwrap();
    let sync = contract.function("sync").unwrap();

    let selector = hex::encode(&input.0[..4]);
    let expected_selector = &hex::encode(&keccak256(swap.signature().as_bytes()))[0..8];

    if !selector.eq(expected_selector) {
        return Err(CustomError::NotFound("ignore".into()));
    }

    let input_data = &input.0[4..];

    println!("{}", selector);
    if let Ok(data) = swap.decode_input(input_data) {
        let amount0_out = data[0].clone().into_uint().unwrap_or_default();
        let amount1_out = data[1].clone().into_uint().unwrap_or_default();
        let to = data[2].clone().into_address().unwrap_or_default();
        let swap_data = data[3].clone().into_bytes().unwrap_or_default();
        println!("amount0_out: {amount0_out}\namount1_out: {amount1_out}\nto: {to:?}");
        if swap_data.len() > 2 {
            let call_back_data = &swap_data[2..];
            if let Ok(data) = sync.decode_input(call_back_data) {
                println!("decoded: {data:?}");
            }
        }
    } else {
        return Err(CustomError::NotFound("ignore".into()));
    }

    Ok(())
}

