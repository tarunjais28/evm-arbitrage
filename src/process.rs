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

    for log in logs {
        if let Ok(parsed_log) = events[0].parse_log(web3::ethabi::RawLog {
            topics: log.topics.clone(),
            data: log.data.0.clone(),
        }) {
            output.update(parsed_log)?;
        } else if let Ok(parsed_log) = events[1].parse_log(web3::ethabi::RawLog {
            topics: log.topics,
            data: log.data.0,
        }) {
            output.update(parsed_log)?;
        }
    }

    output.show();

    Ok(())
}
