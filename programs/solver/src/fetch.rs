use super::*;

#[derive(Default, Debug, Clone, Copy)]
pub struct Reserves {
    pub reserve0: U256,
    pub reserve1: U256,
}

pub async fn get_reserves_v2<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pool_addresses: &[Address],
) -> Result<HashMap<Address, Reserves>, CustomError<'a>> {
    // Create a vector to hold all the futures
    let mut futures = Vec::with_capacity(pool_addresses.len());

    // Clone the provider for each request
    for &address in pool_addresses {
        let provider_clone = provider.clone();
        let fut = async move {
            let contract = IUniswapV2Pair::new(address, provider_clone);
            let reserves = contract.getReserves().call().await?;
            Ok((
                address,
                Reserves {
                    reserve0: U256::from(reserves._reserve0),
                    reserve1: U256::from(reserves._reserve1),
                },
            ))
        };
        futures.push(fut);
    }

    // Execute all futures concurrently
    let results: Vec<Result<(Address, Reserves), CustomError<'a>>> = futures::stream::iter(futures)
        .buffer_unordered(10) // Limit to 10 concurrent requests
        .collect::<Vec<_>>()
        .await;

    // Collect results into a HashMap
    let mut reserves_map = HashMap::with_capacity(pool_addresses.len());
    for result in results {
        let (address, reserves) = result?;
        reserves_map.insert(address, reserves);
    }

    Ok(reserves_map)
}

pub async fn get_reserves_v3<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    reserves_map: &mut HashMap<Address, Reserves>,
    pools: Vec<v2::Pools>,
) -> Result<(), CustomError<'a>> {
    let mut futures = Vec::with_capacity(pools.len());

    for pool in pools {
        let provider_clone = provider.clone();
        let fut = async move {
            let mut contract = ERC20::new(pool.token_a, provider_clone.clone());
            let reserve0 = contract.balanceOf(pool.address).call().await?;

            contract = ERC20::new(pool.token_b, provider_clone);
            let reserve1 = contract.balanceOf(pool.address).call().await?;

            Ok((pool.address, Reserves { reserve0, reserve1 }))
        };
        futures.push(fut);
    }

    // Execute all futures concurrently
    let results: Vec<Result<(Address, Reserves), CustomError<'a>>> = futures::stream::iter(futures)
        .buffer_unordered(10) // Limit to 10 concurrent requests
        .collect::<Vec<_>>()
        .await;

    // Collect results into a HashMap
    for result in results {
        let (address, reserves) = result?;
        reserves_map.insert(address, reserves);
    }

    Ok(())
}

pub async fn get_reserves_v3_single<'a>(
    provider: &FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pool_address: Address,
    token_data: v2::TokenData,
) -> Result<Reserves, CustomError<'a>> {
    let mut contract = ERC20::new(token_data.token_a, provider.clone());
    let reserve0 = contract.balanceOf(pool_address).call().await?;

    contract = ERC20::new(token_data.token_b, provider.clone());
    let reserve1 = contract.balanceOf(pool_address).call().await?;

    Ok(Reserves { reserve0, reserve1 })
}
