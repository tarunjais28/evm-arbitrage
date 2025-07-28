use super::*;

#[derive(Default, Debug, Clone, Copy)]
pub struct Reserves {
    pub reserve0: BigInt,
    pub reserve1: BigInt,
}

pub async fn get_reserves_v2<'a>(
    provider: &SolverProvider,
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
                    reserve0: U256::from(reserves._reserve0).to_big_int(),
                    reserve1: U256::from(reserves._reserve1).to_big_int(),
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
