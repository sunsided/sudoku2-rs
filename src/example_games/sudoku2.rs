use crate::*;

/// Produces an example Sudoku game.
///
/// ## Initial state
/// ```plain
///     · 2 8   · · 7   · · ·
///     · 1 6   · 8 3   · 7 ·
///     · · ·   · 2 ·   8 5 1
///
///     1 3 7   2 9 ·   · · ·
///     · · ·   7 3 ·   · · ·
///     · · ·   · 4 6   3 · 7
///
///     2 9 ·   · 7 ·   · · ·
///     · · ·   8 6 ·   1 4 ·
///     · · ·   3 · ·   7 · ·
/// ```
//noinspection DuplicatedCode
#[rustfmt::skip]
pub fn example_sudoku() -> Game {
    let groups = CellGroups::default()
        .with_default_sudoku_blocks()
        .with_default_rows_and_columns();

    // Hidden single is at 3 x 2, value 6.

    let x = 0u8;
    let state = GameState::new_from([
        x, 2, 8,   x, x, 7,   x, x, x,
        x, 1, 6,   x, 8, 3,   x, 7, x,
        x, x, x,   x, 2, x,   8, 5, 1,

        1, 3, 7,   2, 9, x,   x, x, x,
        x, x, x,   7, 3, x,   x, x, x,
        x, x, x,   x, 4, 6,   3, x, 7,

        2, 9, x,   x, 7, x,   x, x, x,
        x, x, x,   8, 6, x,   1, 4, x,
        x, x, x,   3, x, x,   7, x, x,
    ]);

    Game {
        initial_state: state,
        groups,
        expected_solution: None,
    }
}
