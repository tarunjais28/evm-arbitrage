use super::*;

#[derive(Default, Debug)]
pub struct Reserves {
    pub reserve0: U112,
    pub reserve1: U112,
}

pub async fn get_reserves<'a>(
    provider: FillProvider<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
    pool_address: Address,
) -> Result<Reserves, CustomError<'a>> {
    let contract = IUniswapV2Pair::new(pool_address, provider);
    let reserves = contract.getReserves().call().await?;

    Ok(Reserves {
        reserve0: reserves._reserve0,
        reserve1: reserves._reserve1,
    })
}
