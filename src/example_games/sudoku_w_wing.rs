use crate::*;

/// Produces a real Sudoku puzzle that exercises advanced strategies including
/// W-Wing during the solve.
///
/// The grid is one of Peter Norvig's "hard" puzzles from his "Solving Every
/// Sudoku Puzzle" collection - uniquely solvable, and well known to require
/// bivalue / chain-style strategies after the singles and subset families are
/// exhausted. The solver advances through fish-style, XY-Wing and W-Wing
/// elimination chains before resorting to branching.
///
/// ## Initial state
/// ```plain
///     · 4 ·   · · ·   · 8 ·
///     5 · 3   · · ·   · · ·
///     · · ·   · · 7   · · ·
///
///     · · ·   2 · ·   · · ·
///     6 · ·   · · ·   8 · 4
///     · · ·   · · ·   1 · ·
///
///     · · ·   · 6 ·   3 · 7
///     · 5 ·   · 2 ·   · · ·
///     · · ·   1 · 4   · · ·
/// ```
#[rustfmt::skip]
pub fn example_sudoku() -> Game {
    let groups = CellGroups::default()
        .with_default_sudoku_blocks()
        .with_default_rows_and_columns();

    let x = 0u8;
    let state = GameState::new_from([
        x, 4, x,   x, x, x,   x, 8, x,
        5, x, 3,   x, x, x,   x, x, x,
        x, x, x,   x, x, 7,   x, x, x,

        x, x, x,   2, x, x,   x, x, x,
        6, x, x,   x, x, x,   8, x, 4,
        x, x, x,   x, x, x,   1, x, x,

        x, x, x,   x, 6, x,   3, x, 7,
        x, 5, x,   x, 2, x,   x, x, x,
        x, x, x,   1, x, 4,   x, x, x,
    ]);

    Game {
        initial_state: state,
        groups,
        expected_solution: None,
    }
}
