use crate::cell_group::CellGroups;
use crate::prelude::GameState;

/// A Sudoku game.
pub struct Game {
    /// The initial state of the game.
    pub initial_state: GameState,
    /// The groups of the game cells.
    pub groups: CellGroups,
    /// An expected solution, if available, for comparison.
    pub expected_solution: Option<GameState>,
}
