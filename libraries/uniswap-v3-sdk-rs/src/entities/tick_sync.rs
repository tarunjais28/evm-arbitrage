use crate::prelude::*;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct TickSync {
    pub index: i32,
    pub liquidity_gross: u128,
    pub liquidity_net: i128,
    pub is_init: bool,
}

/// Return the next tick that is initialized within a single word
///
/// ## Arguments
///
/// * `tick`: The current tick
/// * `lte`: Whether the next tick should be lte the current tick
///
/// returns: Result<(Self::Index, bool), Error>
pub fn get_next_initialized_tick(
    current_tick: i32,
    ticks: &[TickSync],
    zero_for_one: bool,
) -> Result<TickSync, Error> {
    let idx = ticks
        .binary_search_by_key(&current_tick, |t| t.index)
        .map_err(|_| Error::NoTickDataError)?;

    if zero_for_one {
        for i in (0..=idx).rev() {
            let tick = ticks[i];
            if tick.is_init {
                return Ok(tick);
            }
        }
    } else {
        for i in idx..ticks.len() {
            let tick = ticks[i];
            if tick.is_init {
                return Ok(tick);
            }
        }
    }

    Err(Error::NoTickDataError)
}
