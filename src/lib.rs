mod board_stats;
mod cell_group;
mod coordinate;
mod default_solver;
pub mod difficulty_estimator;
pub mod example_games;
mod game;
mod game_cell;
mod game_state;
pub mod generator;
mod index;
pub mod serialization;
mod state_stack;
mod strategies;
mod value;
pub mod visualization;

pub use crate::cell_group::{CellGroup, CellGroupType, CellGroups, CollectIndexes};
pub use crate::coordinate::Coordinate;
pub use crate::default_solver::{DefaultSolver, DefaultSolverConfig, Unsolvable};
pub use crate::difficulty_estimator::{estimate_difficulty, Difficulty};
pub use crate::game::Game;
pub use crate::game_cell::{GameCell, IndexedGameCell};
pub use crate::game_state::GameState;
pub use crate::generator::{
    ClueDigger, GenerationError, NonominoRegionGenerator, Puzzle, PuzzleGenerator,
    PuzzleGeneratorConfig, RemovalStrategy, StoppingCondition, Symmetry, Variant,
};
pub use crate::index::Index;
pub use crate::index::IndexBitSet;
pub use crate::serialization::{ParseError, SudokuSerializer};
pub use crate::value::Value;
pub use crate::value::ValueBitSet;
pub use crate::value::ValueOption;
