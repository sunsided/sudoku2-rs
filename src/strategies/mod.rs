mod hidden_singles;
mod naked_singles;
mod naked_twins;

use crate::cell_group::CellGroups;
use crate::game_state::{GameState, InvalidGameState};
use std::fmt::Debug;

pub use hidden_singles::HiddenSingles;
pub use naked_singles::NakedSingles;
pub use naked_twins::NakedTwins;

pub enum StrategyResult {
    AppliedChange,
    NoChange,
}

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
    ) -> Result<StrategyResult, InvalidGameState>;
}
