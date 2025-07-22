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
    if zero_for_one {
        let idx = ticks
            .iter()
            .rev()
            .position(|t| t.index.le(&current_tick))
            .ok_or_else(|| Error::NoTickDataError)?;

        for i in (0..=idx).rev() {
            let tick = ticks[i];
            if tick.is_init {
                return Ok(tick);
            }
        }
    } else {
        let idx = ticks
            .iter()
            .position(|t| t.index.gt(&current_tick))
            .ok_or_else(|| Error::NoTickDataError)?;

        for i in idx..ticks.len() {
            let tick = ticks[i];
            if tick.is_init {
                return Ok(tick);
            }
        }
    }

    Err(Error::NoTickDataError)
}
