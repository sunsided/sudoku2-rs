use crate::*;

/// Produces an example Sudoku game with an X-Wing configuration.
///
/// ## Initial state
/// ```plain
///     9 · ·   8 6 1   · · 5
///     · 8 7   5 4 2   · · 9
///     · · ·   9 7 3   · · 2
///
///     8 · ·   · · 4   1 · 3
///     · 6 1   · 3 5   9 4 8
///     4 · 3   1 8 ·   · · 7
///
///     5 1 ·   · · 7   · · 6
///     · · ·   · 5 8   2 9 1
///     · · 8   3 1 ·   · · 4
/// ```
#[rustfmt::skip]
pub fn example_sudoku() -> Game {
    let groups = CellGroups::default()
        .with_default_sudoku_blocks()
        .with_default_rows_and_columns();

    let x = 0u8;
    let state = GameState::new_from([
        9, x, x,   8, 6, 1,   x, x, 5,
        x, 8, 7,   5, 4, 2,   x, x, 9,
        x, x, x,   9, 7, 3,   x, x, 2,

        8, x, x,   x, x, 4,   1, x, 3,
        x, 6, 1,   x, 3, 5,   9, 4, 8,
        4, x, 3,   1, 8, x,   x, x, 7,

        5, 1, x,   x, x, 7,   x, x, 6,
        x, x, x,   x, 5, 8,   2, 9, 1,
        x, x, 8,   3, 1, x,   x, x, 4
    ]);

    Game {
        initial_state: state,
        groups,
        expected_solution: None,
    }
}
