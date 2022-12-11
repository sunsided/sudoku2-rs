mod hidden_singles;
mod naked_singles;
mod naked_twins;

use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use std::fmt::Debug;
use std::ops::{BitOr, BitOrAssign};

pub use hidden_singles::HiddenSingles;
pub use naked_singles::NakedSingles;
pub use naked_twins::NakedTwins;

pub trait Strategy: Debug {
    /// Indicates whether the next strategy should always be executed
    /// (if `true`) regardless of the return value of [`Strategy::apply`]
    /// or (if `false`) whether execution should restart with the first registered
    /// strategy if [`Strategy::apply`] indicates success.
    fn always_continue(&self) -> bool;

    /// Applies the strategy to the state.
    fn apply(
        &self,
        state: &GameState,
        groups: &CellGroups,
    ) -> Result<StrategyResult, InvalidGameState> {
        let mut result = StrategyResult::NoChange;
        result |= self.apply_in_group(state, groups, CellGroupType::Custom)?;
        result |= self.apply_in_group(state, groups, CellGroupType::StandardBlock)?;
        result |= self.apply_in_group(state, groups, CellGroupType::StandardRow)?;
        result |= self.apply_in_group(state, groups, CellGroupType::StandardColumn)?;
        Ok(result)
    }

    /// Applies the strategy to the state, restricting
    /// it to a specific group of peers.
    ///
    /// ## Panics
    /// This method is not guaranteed to be implemented for every
    /// strategy and is to be considered an implementation detail.
    /// Solver code should execute [`Strategy::apply`] instead.
    fn apply_in_group(
        &self,
        state: &GameState,
        groups: &CellGroups,
        group_type: CellGroupType,
    ) -> Result<StrategyResult, InvalidGameState>;
}

/// The outcome of a strategy.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum StrategyResult {
    /// The strategy applied some moves or erased some candidates.
    AppliedChange,
    /// No change was applied by this strategy.
    NoChange,
}

impl BitOr for StrategyResult {
    type Output = StrategyResult;

    fn bitor(self, rhs: Self) -> Self::Output {
        if self == StrategyResult::AppliedChange || rhs == StrategyResult::AppliedChange {
            StrategyResult::AppliedChange
        } else {
            StrategyResult::NoChange
        }
    }
}

impl BitOrAssign for StrategyResult {
    fn bitor_assign(&mut self, rhs: Self) {
        if rhs == StrategyResult::AppliedChange {
            *self = StrategyResult::AppliedChange;
        }
    }
}
