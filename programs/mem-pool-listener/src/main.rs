use crate::{helper::*, structs::*};
use futures::StreamExt;
use hex::FromHex;
use std::{collections::HashMap, fs::File, io::BufReader};
use utils::{CustomError, EnvParser};
use web3::{
    ethabi::{Address, Contract, Function},
    signing::keccak256,
    transports::WebSocket,
    types::{Transaction, H160, H256},
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
                match web3
                    .eth()
                    .transaction(web3::types::TransactionId::Hash(hash))
                    .await
                {
                    Ok(Some(tx)) => {
                        if tx.block_hash.is_none()
                            && tx.block_number.is_none()
                            && tx.transaction_index.is_none()
                            && tx.input.0.len() > 4
                        {
                            let selector = hex::encode(&tx.input.0[..4]);
                            let input_data = &tx.input.0[4..];
                            
                            if let Some(idx) = identifiers.iter().position(|x| selector.eq(x)) {
                                if let Some(swap_fn) = generator.get(&selector) {
                                    let names = names_map.get(&identifiers[idx]).unwrap();
                                    swap_fn(funcs[idx], &input_data, names)?;
                                    println!("hash: {:?}", tx.hash);
                                    println!("from: {:?}", tx.from);
                                    println!("to {:?}", tx.to);
                                    println!("{}", "=".repeat(70));
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        println!("Tx {:?} not yet available", hash);
                    }
                    Err(e) => {
                        eprintln!("Error fetching tx {:?}: {:?}", hash, e);
                    }
                }
            }
            Err(e) => eprintln!("Subscription error: {:?}", e),
        }
    }

    Ok(())
}
