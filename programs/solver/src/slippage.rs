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

    pools.iter().for_each(|pool| {
        let (pair, data) = pool.to_key_value();
        pool_data.insert(pair, data);
    });

    for (pair, data) in pool_data.iter_mut() {
        let reserves = get_reserves(provider.clone(), data.pool).await?;
        data.update_reserves(reserves);
        data.calc_slippage();

        edges.push((pair.token_a, pair.token_b, data.pool, data.slippage));
    }

    let graph = build_bidirectional_graph(&edges);

    Ok(graph)
}
