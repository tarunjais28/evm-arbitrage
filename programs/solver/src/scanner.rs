use super::*;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct InputData {
    pub token_a: Address,
    pub token_b: Address,
    pub amount_in: U256,
}

fn calculate_path<'a>(
    pool_data: &mut PoolData,
    input_data: InputData,
) -> Result<(), CustomError<'a>> {
    let graph = debug_time!("scanner::calculate_path::calc_slippage()", {
        calc_slippage(pool_data, input_data.amount_in)?
    });

    let path = debug_time!("scanner::calculate_path::best_path()", {
        best_path(&graph, &input_data.token_a, &input_data.token_b)
    });

    println!(
        "Optimal path for input {:#?}:
{:#?}",
        input_data, path
    );
    Ok(())
}

pub async fn scan<'a>(
    provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pool_addresses: Vec<Address>,
    pool_data: PoolData,
) -> Result<(), CustomError<'a>> {
    // Create a filter for the events.
    let filter = provider
        .clone()
        .subscribe_logs(&Filter::new().address(pool_addresses))
        .await?;

    log::info!("Waiting for events...");

    let mut stream = filter.into_stream();
    let (tx, rx) = mpsc::channel(32);

    // Create a shared state for the current amount
    let pool_data = Arc::new(Mutex::new(pool_data));

    // Spawn a task to handle user input
    let input_handle = {
        let pool_data_clone = Arc::clone(&pool_data);
        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin);
            let mut input = String::new();

            loop {
                println!("\nEnter input data to calculate path (or 'q' to quit): ");
                input.clear();

                if let Err(e) = reader.read_line(&mut input).await {
                    log::error!("Error reading input: {}", e);
                    break;
                }

                let input = input.trim();
                if input.to_lowercase() == "q" {
                    break;
                }

                let buffer = input.trim();
                if let Ok(input_data) = serde_json::from_str::<InputData>(buffer) {
                    // Calculate path immediately after receiving amount
                    if let Err(e) = calculate_path(&mut *pool_data_clone.lock().await, input_data) {
                        log::error!("Error calculating path: {}", e);
                    }

                    if let Err(e) = tx.send(input_data.amount_in).await {
                        log::error!("Error sending amount: {}", e);
                        break;
                    }
                } else {
                    log::error!("Invalid input. Please enter a valid number or 'q' to quit.");
                }
            }
        })
    };

    // Process events from the stream
    while let Some(log) = stream.next().await {
        let mut scanner = ScanData::new(&log);

        if let Ok(decoded) = log.log_decode() {
            let swap: Swap = decoded.inner.data;
            scanner.update_swap(swap, decoded.inner.address);
        } else if let Ok(decoded) = log.log_decode() {
            let sync: Sync = decoded.inner.data;
            scanner.update_sync(sync, decoded.inner.address);

            // Update reserves based on the event
            update_reserve_abs(scanner, &mut *pool_data.lock().await)?;
        } else if let Ok(decoded) = log.log_decode() {
            let mint: Mint = decoded.inner.data;
            scanner.update_liquidity_events(mint, decoded.inner.address);
        } else if let Ok(decoded) = log.log_decode() {
            let burn: Burn = decoded.inner.data;
            scanner.update_liquidity_events(burn, decoded.inner.address);
        }

        log::info!("{:?}", scanner.tx_type);
        // scanner.show();
    }

    // Clean up
    drop(rx);
    input_handle.abort();

    Ok(())
}
