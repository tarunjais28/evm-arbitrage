use super::*;

pub async fn calc_slippage<'a>(
    provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pools: Vec<Pools>,
) -> Result<SwapGraph, CustomError<'a>> {
    let mut pool_data: HashMap<TokenPair, TokenData> = HashMap::with_capacity(pools.len());
    let mut edges = Vec::with_capacity(pools.len());

    debug_time!("pool_data()", {
        pools.iter().for_each(|pool| {
            let (pair, data) = pool.to_key_value();
            pool_data.insert(pair, data);
        })
    });

    debug_time!("pool_data_abstraction()", {
        for (pair, data) in pool_data.iter_mut() {
            let reserves = debug_time!("get_reserves()", {
                get_reserves(provider.clone(), data.pool).await?
            });
            debug_time!("update_reserves()", { data.update_reserves(reserves) });
            debug_time!("calc_slippage()", { data.calc_slippage() });

            debug_time!("edges()", {
                edges.push((pair.token_a, pair.token_b, data.pool, data.slippage))
            });
        }
    });

    let graph = debug_time!("build_bidirectional_graph()", {
        build_bidirectional_graph(&edges)
    });

    Ok(graph)
}
