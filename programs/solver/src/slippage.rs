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
        for (_, data) in pool_data.iter_mut() {
            if let Some(reserves) = reserves_map.get(&data.pool) {
                data.update_reserves(*reserves);
            }
        }
    });

    Ok(pool_data)
}

pub async fn calc_slippage<'a>(
    pair: TokenPair,
    pool_data: &mut PoolData,
    reserves: Reserves,
    amount_in: U256,
) -> Result<SwapGraph, CustomError<'a>> {
    let mut edges = Vec::with_capacity(pool_data.len());

    debug_time!("slippage::calc_slippage::reserves_updation()", {
        pool_data
            .entry(pair)
            .and_modify(|data| data.update_reserves(reserves))
    });

    debug_time!("slippage::calc_slippage::calc_slippage()", {
        pool_data.iter_mut().for_each(|(pair, data)| {
            data.calc_slippage(amount_in);
            edges.push((pair.token_a, pair.token_b, data.pool, data.slippage));
        })
    });

    let graph = debug_time!("build_bidirectional_graph()", {
        build_bidirectional_graph(&edges)
    });

    Ok(graph)
}
