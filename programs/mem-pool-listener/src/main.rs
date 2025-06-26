use crate::{helper::*, structs::*};
use colored::Colorize;
use futures::StreamExt;
use std::{collections::HashMap, fs::File, io::BufReader};
use utils::{CustomError, EnvParser};
use web3::{
    ethabi::{Address, Contract, Function},
    signing::keccak256,
    transports::WebSocket,
    types::H160,
    Web3,
};

mod helper;
mod structs;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let env_parser = EnvParser::new()?;

    let file = File::open("programs/identifier/resources/uniswap_routerv2_abi.json")?;
    let reader = BufReader::new(file);

    let contract = Contract::load(reader)?;

    let ws = WebSocket::new(&env_parser.ws_address).await?;
    let web3 = Web3::new(ws);

    let mut sub = web3
        .eth_subscribe()
        .subscribe_new_pending_transactions()
        .await?;

    let (identifiers, funcs, generator, names_map) = generate_maps(&contract)?;

    println!("Listening for pending Ethereum transactions...");

    while let Some(tx_hash) = sub.next().await {
        match tx_hash {
            Ok(hash) => {
                // Fetch full transaction details using the hash
                {
                    if let Ok(Some(tx)) = web3
                        .eth()
                        .transaction(web3::types::TransactionId::Hash(hash))
                        .await
                    {
                        if tx.input.0.len() > 4 {
                            let selector = hex::encode(&tx.input.0[..4]);
                            let input_data = &tx.input.0[4..];
                            if let Some(idx) = identifiers.iter().position(|x| selector.eq(x)) {
                                if let Some(swap_fn) = generator.get(&selector) {
                                    let names = names_map.get(&identifiers[idx]).unwrap();
                                    println!("{}", selector);
                                    println!("{}", hex::encode(input_data));
                                    swap_fn(
                                        funcs[idx],
                                        &input_data,
                                        names,
                                        &env_parser.pools,
                                    )?;
                                    if let Some(from) = tx.from {
                                        if env_parser.pools.contains(&from) {
                                            println!("{}", format!("Found: {from:?}").green());
                                        }
                                    }
                                    if let Some(to) = tx.to {
                                        if env_parser.pools.contains(&to) {
                                            println!("{}", format!("Found: {to:?}").green());
                                        }
                                    }
                                    println!("hash: {:?}", tx.hash);
                                    println!("from: {:?}", tx.from);
                                    println!("to {:?}", tx.to);
                                    println!("{}", "=".repeat(70));
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => eprintln!("Subscription error: {:?}", e),
        }
    }

    Ok(())
}
