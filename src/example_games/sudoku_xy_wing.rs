use crate::*;

/// Produces a real Sudoku puzzle that exercises advanced strategies including
/// XY-Wing during the solve.
///
/// The grid is Peter Norvig's `hard1` from his "Solving Every Sudoku Puzzle"
/// collection - a uniquely solvable, well-known hard puzzle. Singles and
/// subset strategies will not finish it on their own; the solver advances
/// through fish-style and bivalue strategies (including XY-Wing) before
/// resorting to branching.
///
/// ## Initial state
/// ```plain
///     4 · ·   · · ·   8 · 5
///     · 3 ·   · · ·   · · ·
///     · · ·   7 · ·   · · ·
///
///     · 2 ·   · · ·   · 6 ·
///     · · ·   · 8 ·   4 · ·
///     · · ·   · 1 ·   · · ·
///
///     · · ·   6 · 3   · 7 ·
///     5 · ·   2 · ·   · · ·
///     1 · 4   · · ·   · · ·
/// ```
#[rustfmt::skip]
pub fn example_sudoku() -> Game {
    let groups = CellGroups::default()
        .with_default_sudoku_blocks()
        .with_default_rows_and_columns();

    let x = 0u8;
    let state = GameState::new_from([
        4, x, x,   x, x, x,   8, x, 5,
        x, 3, x,   x, x, x,   x, x, x,
        x, x, x,   7, x, x,   x, x, x,

        x, 2, x,   x, x, x,   x, 6, x,
        x, x, x,   x, 8, x,   4, x, x,
        x, x, x,   x, 1, x,   x, x, x,

        x, x, x,   6, x, 3,   x, 7, x,
        5, x, x,   2, x, x,   x, x, x,
        1, x, 4,   x, x, x,   x, x, x,
    ]);

    Game {
        initial_state: state,
        groups,
        expected_solution: None,
    }
}
