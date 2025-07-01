use super::*;
use tokio::io::{AsyncBufReadExt, BufReader};

async fn calculate_path<'a>(
    pool_address: Address,
    pool_data: &mut PoolData,
    scanner: ScanData,
    amount_in: U256,
) -> Result<(), CustomError<'a>> {
    let graph = debug_time!("scanner::calculate_path::calc_slippage()", {
        calc_slippage(pool_address, pool_data, Reserves::from(scanner), amount_in).await?
    });

    let path = debug_time!("scanner::calculate_path::best_path()", {
        best_path(
            &graph,
            &address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"),
            &address!("0x2260fac5e5542a773aa44fbcfedf7c193bc2c599"),
        )
    });

    println!(
        "Optimal path for amount {}:
{:#?}",
        amount_in, path
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
    pool_data: &mut PoolData,
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
    let current_amount = Arc::new(Mutex::new(U256::from(1)));

    // Spawn a task to handle user input
    let input_handle = {
        let current_amount = Arc::clone(&current_amount);
        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let mut reader = BufReader::new(stdin);
            let mut input = String::new();

            loop {
                println!("\nEnter amount to calculate path (or 'q' to quit): ");
                input.clear();

                if let Err(e) = reader.read_line(&mut input).await {
                    eprintln!("Error reading input: {}", e);
                    break;
                }

                let input = input.trim();
                if input.to_lowercase() == "q" {
                    break;
                }

                if let Ok(amount) = input.parse::<u128>() {
                    let amount = U256::from(amount);
                    *current_amount.lock().await = amount;
                    println!("New amount set to: {}", amount);

                    if let Err(e) = tx.send(amount).await {
                        log::error!("Error sending amount: {}", e);
                        break;
                    }
                } else if !input.is_empty() {
                    log::error!("Invalid input. Please enter a valid number or 'q' to quit.");
                }
            }
        })
    };

    // Process events from the stream
    while let Some(log) = stream.next().await {
        let mut scanner = ScanData::new(&log);
        let amount = *current_amount.lock().await;

        if let Ok(decoded) = log.log_decode() {
            let swap: Swap = decoded.inner.data;
            scanner.update_swap(swap, decoded.inner.address);

            // Process the swap with current amount
            if let Err(e) = calculate_path(decoded.inner.address, pool_data, scanner, amount).await
            {
                log::error!("Error calculating path: {}", e);
            }
        } else if let Ok(decoded) = log.log_decode() {
            let sync: Sync = decoded.inner.data;
            scanner.update_sync(sync, decoded.inner.address);
        } else if let Ok(decoded) = log.log_decode() {
            let mint: Mint = decoded.inner.data;
            scanner.update_liquidity_events(mint, decoded.inner.address);
        } else if let Ok(decoded) = log.log_decode() {
            let burn: Burn = decoded.inner.data;
            scanner.update_liquidity_events(burn, decoded.inner.address);
        }

        scanner.show();
    }

    // Clean up
    drop(rx);
    input_handle.abort();

    Ok(())
}
