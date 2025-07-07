use super::*;

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV2Pool,
    "../../resources/contracts/uniswapv2_pool_abi.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV2Pair,
    "../../resources/contracts/uniswapv2_pair.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV3Factory,
    "../../resources/contracts/uniswapv3_factory.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IUniswapV3Pool,
    "../../resources/contracts/uniswapv3_pool_abi.json"
);

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    ERC20,
    "../../resources/contracts/erc20_abi.json"
);
