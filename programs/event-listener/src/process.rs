use super::*;

async fn read_logs(
    web3: Arc<Web3<WebSocket>>,
    block_hash: H256,
    contract_address: H160,
    event_signatures: Vec<H256>,
) -> Result<Vec<web3::types::Log>, anyhow::Error> {
    let logs = web3
        .eth()
        .logs(
            web3::types::FilterBuilder::default()
                .block_hash(block_hash)
                .address(vec![contract_address])
                .topics(Some(event_signatures), None, None, None)
                .build(),
        )
        .await?;

    Ok(logs)
}

pub async fn scan(
    web3: Arc<Web3<WebSocket>>,
    contract_address: H160,
    events: &[Event],
    event_signatures: Vec<H256>,
    block_hash: H256,
) -> Result<(), anyhow::Error> {
    let logs = read_logs(web3, block_hash, contract_address, event_signatures).await?;
    if logs.is_empty() {
        return Ok(());
    }

    let mut output = Output::new(contract_address);

    // topic_hash => (event_index, tx_type, should_show)
    let event_map: HashMap<H256, (usize, Option<TxType>, bool)> = vec![
        (events[0].signature(), (0, Some(TxType::Swap), true)),
        (events[1].signature(), (1, None, false)),
        (events[2].signature(), (2, Some(TxType::Add), true)),
        (events[3].signature(), (3, Some(TxType::Remove), true)),
    ]
    .into_iter()
    .collect();

    for log in logs {
        println!("{:?}", log.transaction_hash.unwrap());
        let raw_log = RawLog {
            topics: log.topics.clone(),
            data: log.data.0.clone(),
        };

        let Some(topic0) = log.topics.first() else {
            println!("{}", format!("{log:#?}").red());
            continue;
        };

        // TODO: Tx_Hash can be use for sync match
        match event_map.get(topic0) {
            Some((idx, tx_type, show)) => match events[*idx].parse_log(raw_log) {
                Ok(parsed_log) => {
                    if let Some(t) = tx_type {
                        output.update_tx_type(*t);
                    }
                    output.update(parsed_log)?;
                    if *show {
                        output.show();
                        output.clear();
                    }
                }
                Err(_) => println!("{}", format!("{log:#?}").red().bold()),
            },
            None => println!("{}", format!("{log:#?}").red().bold()),
        }
    }

    Ok(())
}
