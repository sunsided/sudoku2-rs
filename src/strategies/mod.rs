mod hidden_singles;
mod naked_singles;
mod naked_twins;

use crate::cell_group::CellGroups;
use crate::game_state::{GameState, InvalidGameState};

pub use hidden_singles::HiddenSingles;
pub use naked_singles::NakedSingles;
pub use naked_twins::NakedTwins;

pub trait Strategy {
    fn always_continue(&self) -> bool;
    fn apply(&self, state: &GameState, groups: &CellGroups) -> Result<bool, InvalidGameState>;
}
