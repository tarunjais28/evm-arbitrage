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
    pool_address: &PoolAddress,
) -> Result<PoolData, CustomError<'a>> {
    let mut pool_data: PoolData = HashMap::with_capacity(pools.len());
    let pools_v3: Vec<Pools> = pools.iter().filter(|p| p.fee > 0).cloned().collect();

    debug_time!("update_reserves::pool_data()", {
        pools.iter().for_each(|pool| {
            let (pair, data) = pool.to_key_value();
            pool_data.insert(pair, data);
        })
    });

    // Get all reserves in a single batch
    let mut reserves_map = debug_time!("update_reserves::get_reserves_v2()", {
        get_reserves_v2(&provider, &pool_address.v2).await?
    });

    debug_time!("update_reserves::get_reserves_v3()", {
        get_reserves_v3(&provider, &mut reserves_map, pools_v3).await?
    });

    debug_time!("update_reserves::pool_data_abstraction()", {
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
    debug_time!("calc_slippage::update_reserve_abs()", {
        pool_data
            .entry(scanner.pool_address)
            .and_modify(|data| data.update_reserves(Reserves::from(scanner)))
    });

    Ok(())
}

pub async fn update_reserve_v3<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pool_address: Address,
    pool_data: &mut PoolData,
) -> Result<(), CustomError<'a>> {
    debug_time!("calc_slippage::update_reserve_abs()", {
        let token_data = pool_data
            .get(&pool_address)
            .ok_or_else(|| CustomError::NotFound("token_data"))?;

        let reserves = get_reserves_v3_single(&provider, pool_address, *token_data).await?;
        pool_data
            .entry(pool_address)
            .and_modify(|data| data.update_reserves(reserves))
    });

    Ok(())
}

pub fn calc_slippage<'a>(pool_data: &mut PoolData) -> Result<SwapGraph, CustomError<'a>> {
    let mut edges = Vec::with_capacity(pool_data.len());

    debug_time!("calc_slippage::calc_slippage()", {
        pool_data.iter_mut().for_each(|(pool, data)| {
            data.calc_slippages();
            edges.push((
                data.token_a,
                data.token_b,
                *pool,
                data.slippage0,
                data.slippage1,
                data.fee,
            ));
        })
    });

    let graph = debug_time!("build_bidirectional_graph()", {
        build_bidirectional_graph(&edges)
    });

    Ok(graph)
}
