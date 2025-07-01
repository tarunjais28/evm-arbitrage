use super::*;

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

    // Process events from the stream.
    while let Some(log) = stream.next().await {
        let mut scanner = ScanData::new(&log);

        if let Ok(decoded) = log.log_decode() {
            let swap: Swap = decoded.inner.data;
            scanner.update_swap(swap, decoded.inner.address);
        } else if let Ok(decoded) = log.log_decode() {
            let sync: Sync = decoded.inner.data;
            let pool_address = decoded.inner.address;
            scanner.update_sync(sync, pool_address);

            let graph = debug_time!("scanner::calc_slippage()", {
                calc_slippage(
                    pool_address,
                    pool_data,
                    Reserves::from(scanner),
                    U256::from(1),
                )
                .await?
            });
            let path = debug_time!("scanner::best_path()", {
                best_path(
                    &graph,
                    &address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"),
                    &address!("0x2260fac5e5542a773aa44fbcfedf7c193bc2c599"),
                )
            });
            println!("{:#?}", path);
        } else if let Ok(decoded) = log.log_decode() {
            let mint: Mint = decoded.inner.data;
            scanner.update_liquidity_events(mint, decoded.inner.address);
        } else if let Ok(decoded) = log.log_decode() {
            let burn: Burn = decoded.inner.data;
            scanner.update_liquidity_events(burn, decoded.inner.address);
        } else {
            continue;
        }
        scanner.update_reserves(provider.clone()).await?;
        scanner.show();
    }

    Ok(())
}
