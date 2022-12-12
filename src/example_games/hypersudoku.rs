use crate::*;

/// Produces an example Hypersudoku game.
///
/// Hypersudokus behave like regular Sudoku games with the addition
/// of four extra groups (the windows) that overlap with the regular
/// Sudoku blocks.
///
/// ## Initial state
/// ```plain
///     · · ·   · · ·   · 1 ·
///     · · 2   · · ·   · 3 4
///     · · ·   · 5 1   · · ·
///
///     · · ·   · · 6   5 · ·
///     · 7 ·   3 · ·   · 8 ·
///     · · 3   · · ·   · · ·
///
///     · · ·   · 8 ·   · · ·
///     5 8 ·   · · ·   9 · ·
///     6 9 ·   · · ·   · · ·
/// ```
///
/// ## Expected solution
/// ```plain
///     9 4 6   8 3 2   7 1 5
///     1 5 2   6 9 7   8 3 4
///     7 3 8   4 5 1   2 9 6
///
///     8 1 9   7 2 6   5 4 3
///     4 7 5   3 1 9   6 8 2
///     2 6 3   5 4 8   1 7 9
///
///     3 2 7   9 8 5   4 6 1
///     5 8 4   1 6 3   9 2 7
///     6 9 1   2 7 4   3 5 8
/// ```
#[rustfmt::skip]
pub fn example_hypersudoku() -> Game {
    let groups = CellGroups::default()
        .with_hypersudoku_windows()
        .with_default_sudoku_blocks()
        .with_default_rows_and_columns();

    let x = 0u8;
    let state = GameState::new_from([
        x, x, x,  x, x, x,  x, 1, x,
        x, x, 2,  x, x, x,  x, 3, 4,
        x, x, x,  x, 5, 1,  x, x, x,

        x, x, x,  x, x, 6,  5, x, x,
        x, 7, x,  3, x, x,  x, 8, x,
        x, x, 3,  x, x, x,  x, x, x,

        x, x, x,  x, 8, x,  x, x, x,
        5, 8, x,  x, x, x,  9, x, x,
        6, 9, x,  x, x, x,  x, x, x,
    ],);

    let solution = GameState::new_from([
       9, 4, 6,   8, 3, 2,   7, 1, 5,
       1, 5, 2,   6, 9, 7,   8, 3, 4,
       7, 3, 8,   4, 5, 1,   2, 9, 6,

       8, 1, 9,   7, 2, 6,   5, 4, 3,
       4, 7, 5,   3, 1, 9,   6, 8, 2,
       2, 6, 3,   5, 4, 8,   1, 7, 9,

       3, 2, 7,   9, 8, 5,   4, 6, 1,
       5, 8, 4,   1, 6, 3,   9, 2, 7,
       6, 9, 1,   2, 7, 4,   3, 5, 8,
    ],);

    Game {
        initial_state: state,
        groups,
        expected_solution: Some(solution),
    }
}
