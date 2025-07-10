use super::*;

pub fn update_reserve_abs<'a>(
    scanner: ScanData,
    pool_data: &mut v2::PoolData,
) -> Result<(), CustomError<'a>> {
    pool_data
        .data
        .entry(scanner.pool_address)
        .and_modify(|data| data.update_reserves(Reserves::from(scanner)));

    Ok(())
}
