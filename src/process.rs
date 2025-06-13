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

    // Map event signature to index + TxType
    let event_map: HashMap<H256, (usize, Option<TxType>)> = vec![
        (events[0].signature(), (0, Some(TxType::Swap))),
        (events[1].signature(), (1, None)),
        (events[2].signature(), (2, Some(TxType::Add))),
        (events[3].signature(), (3, Some(TxType::Remove))),
    ]
    .into_iter()
    .collect();

    for log in logs {
        let raw = RawLog {
            topics: log.topics.clone(),
            data: log.data.0.clone(),
        };

        if let Some(topic0) = log.topics.first() {
            if let Some((idx, tx_type)) = event_map.get(topic0) {
                if let Ok(parsed_log) = events[*idx].parse_log(raw) {
                    output.update(parsed_log, *tx_type)?;
                    continue;
                }
            }
        }

        // Fallback if no match or parse fails
        println!("{}", format!("{log:#?}").red());
    }

    output.show();
    Ok(())
}
