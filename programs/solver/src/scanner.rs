use super::*;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct InputData {
    pub token_a: Address,
    pub token_b: Address,
    pub amount_in: U256,
}

async fn calculate_path<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pool_data_v2: &mut v2::PoolData,
    pool_data_v3: &mut v3::PoolData,
    input_data: InputData,
) -> Result<(), CustomError<'a>> {
    let mut slippage_adj = BigInt::MAX;
    let amount_in = input_data.amount_in.to_big_int();

    debug_time!("calculate_path::calc_effective_price()", {
        pool_data_v2.calc_effective_price(amount_in)?;
    });

    debug_time!("calculate_path::calc_slippage_v2()", {
        pool_data_v2.calc_slippage(&mut slippage_adj)?;
    });

    debug_time!("calculate_path::calc_effective_price_v3()", {
        pool_data_v3
            .calc_effective_price(provider, amount_in)
            .await?;
    });

    debug_time!("calculate_path::calc_slippage_v3()", {
        pool_data_v3.calc_slippage(&mut slippage_adj)?;
    });

    let mut graph: SwapGraph =
        HashMap::with_capacity((pool_data_v2.data.len() + pool_data_v2.data.len()) * 2);

    debug_time!("calculate_path::into_v2_swap_graph()", {
        pool_data_v2.to_swap_graph(&mut graph);
    });

    debug_time!("calculate_path::into_v3_swap_graph()", {
        pool_data_v3.to_swap_graph(&mut graph);
    });

    log::info!("Total {} nodes collected!", graph.len());

    slippage_adj = slippage_adj.abs() + BigInt::ONE;
    let mut path = debug_time!("calculate_path::best_path()", {
        best_path(
            &graph,
            &input_data.token_a,
            &input_data.token_b,
            slippage_adj,
        )
    });
    path.cost -= slippage_adj * BigInt::from(path.pools.len());

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
    pool_data_v2: v2::PoolData,
    pool_data_v3: v3::PoolData,
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
    let pool_data_v2 = Arc::new(Mutex::new(pool_data_v2));
    let pool_data_v3 = Arc::new(Mutex::new(pool_data_v3));

    // Spawn a task to handle user input
    let input_handle = {
        let pool_data_v2_clone = Arc::clone(&pool_data_v2);
        let pool_data_v3_clone = Arc::clone(&pool_data_v3);
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
                    if let Err(e) = calculate_path(
                        &provider,
                        &mut *pool_data_v2_clone.lock().await,
                        &mut *pool_data_v3_clone.lock().await,
                        input_data,
                    )
                    .await
                    {
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
            log::info!("v2 swap captured!");
            let sync: IUniswapV2Pool::Sync = decoded.inner.data;
            scanner.update_sync(sync, decoded.inner.address);

            // Update reserves based on the event
            debug_time!("v2::calc_slippage::update_reserve_abs()", {
                update_reserve_abs(scanner, &mut *pool_data_v2.lock().await)?;
            });
        } else if let Ok(decoded) = log.log_decode() {
            log::info!("v3 swap captured!");
            let swap: IUniswapV3Pool::Swap = decoded.inner.data;
            let pool_data = &mut pool_data_v3.lock().await;

            // Update start price
            debug_time!("v3::calc_start_price_from_sqrt_price_x96", {
                pool_data.calc_start_price_from_sqrt_price_x96(
                    &decoded.inner.address,
                    swap.sqrtPriceX96.to_big_int(),
                )?;
            });
        }
    }

    // Clean up
    drop(rx);
    input_handle.abort();

    Ok(())
}
