mod cell_group;
mod coordinate;
mod default_solver;
pub mod example_games;
mod game;
mod game_cell;
mod game_state;
mod index;
mod value;
pub mod visualization;

pub mod prelude {
    pub use crate::cell_group::{CellGroup, CellGroups, OverlappingGroups};
    pub use crate::coordinate::Coordinate;
    pub use crate::default_solver::{DefaultSolver, Unsolvable};
    pub use crate::game::Game;
    pub use crate::game_cell::{GameCell, IndexedGameCell};
    pub use crate::game_state::GameState;
    pub use crate::index::Index;
    pub use crate::index::IndexBitSet;
    pub use crate::value::Value;
    pub use crate::value::ValueBitSet;
    pub use crate::value::ValueOption;
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
