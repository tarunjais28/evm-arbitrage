use super::*;

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub struct InputData {
    pub token_a: Address,
    pub token_b: Address,
    pub amount_in: U256,
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
    file: &mut File,
) -> Result<(), CustomError<'a>> {
    // Create a filter for the events.
    let filter = provider
        .clone()
        .subscribe_logs(&Filter::new().address(pool_addresses))
        .await?;

    log::info!("Waiting for events...");

    let mut stream = filter.into_stream();

    // Create a shared state for the current amount
    let pool_data = Arc::new(Mutex::new(pool_data));

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
            let data = pool_data.lock().await;
            let token_data = data.get(&scanner.pool_address).unwrap();
            let scan_data = SearchData::new(
                log.block_number.unwrap_or_default(),
                scanner.pool_address,
                token_data.token_a,
                token_data.token_b,
                token_data.reserve0,
                token_data.reserve1,
            );
            file.write_all(scan_data.to_string().as_bytes())?;
        } else if let Ok(decoded) = log.log_decode() {
            let mint: Mint = decoded.inner.data;
            scanner.update_liquidity_events(mint, decoded.inner.address);
        } else if let Ok(decoded) = log.log_decode() {
            let burn: Burn = decoded.inner.data;
            scanner.update_liquidity_events(burn, decoded.inner.address);
        }
    }

    Ok(())
}
