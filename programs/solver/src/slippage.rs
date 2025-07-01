use super::*;

pub async fn update_reserves<'a>(
    provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pools: Vec<Pools>,
) -> Result<PoolData, CustomError<'a>> {
    let mut pool_data: PoolData = HashMap::with_capacity(pools.len());
    let pool_addresses: Vec<Address> = pools.iter().map(|p| p.address).collect();

    // Get all reserves in a single batch
    let reserves_map = debug_time!("slippage::update_reserves::get_reserves_batch()", {
        get_reserves_batch(&provider, &pool_addresses).await?
    });

    debug_time!("slippage::update_reserves::pool_data()", {
        pools.iter().for_each(|pool| {
            let (pair, data) = pool.to_key_value();
            pool_data.insert(pair, data);
        })
    });

    debug_time!("slippage::update_reserves::pool_data_abstraction()", {
        for (pool, data) in pool_data.iter_mut() {
            if let Some(reserves) = reserves_map.get(pool) {
                data.update_reserves(*reserves);
            }
        }
    });

    Ok(pool_data)
}

pub fn update_reserve_abs<'a>(
    scanner: ScanData,
    pool_data: &mut PoolData,
) -> Result<(), CustomError<'a>> {
    debug_time!("slippage::calc_slippage::update_reserve_abs()", {
        pool_data
            .entry(scanner.pool_address)
            .and_modify(|data| data.update_reserves(Reserves::from(scanner)))
    });

    Ok(())
}

pub fn calc_slippage<'a>(
    pool_data: &mut PoolData,
    amount_in: U256,
) -> Result<SwapGraph, CustomError<'a>> {
    let mut edges = Vec::with_capacity(pool_data.len());

    debug_time!("slippage::calc_slippage::calc_slippage()", {
        pool_data.iter_mut().for_each(|(pool, data)| {
            data.calc_slippage(amount_in);
            edges.push((data.token_a, data.token_b, *pool, data.slippage));
        })
    });

    let graph = debug_time!("build_bidirectional_graph()", {
        build_bidirectional_graph(&edges)
    });

    Ok(graph)
}
