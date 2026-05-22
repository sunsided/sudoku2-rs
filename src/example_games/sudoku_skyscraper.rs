use crate::*;

/// Produces an example Sudoku game that exercises the Skyscraper strategy.
///
/// During solving, the digit `7` forms a Skyscraper on rows `4` and `8`: the
/// digit appears in exactly two cells of each row, the two rows share column
/// `4` (the base), and the remaining "roof" cells sit at `(7, 4)` and
/// `(8, 8)`. Cells visible to both roof cells (but outside the four pattern
/// cells) drop `7` as a candidate, which unblocks the rest of the solve.
///
/// ## Initial state
/// ```plain
///     · 9 ·   · · 2   · · ·
///     · · ·   7 · ·   · 8 ·
///     · 5 4   · 3 ·   7 · ·
///
///     6 · ·   · · ·   · · ·
///     · · ·   · · 1   · · 2
///     · 7 3   · 5 ·   8 · ·
///
///     9 · ·   · · ·   4 · ·
///     8 · ·   · 6 ·   · · ·
///     · 4 6   · · 5   · 1 ·
/// ```
//noinspection DuplicatedCode
#[rustfmt::skip]
pub fn example_sudoku() -> Game {
    let groups = CellGroups::default()
        .with_default_sudoku_blocks()
        .with_default_rows_and_columns();

    let x = 0u8;
    let state = GameState::new_from([
        x, 9, x,   x, x, 2,   x, x, x,
        x, x, x,   7, x, x,   x, 8, x,
        x, 5, 4,   x, 3, x,   7, x, x,

        6, x, x,   x, x, x,   x, x, x,
        x, x, x,   x, x, 1,   x, x, 2,
        x, 7, 3,   x, 5, x,   8, x, x,

        9, x, x,   x, x, x,   4, x, x,
        8, x, x,   x, 6, x,   x, x, x,
        x, 4, 6,   x, x, 5,   x, 1, x
    ]);

    Game {
        initial_state: state,
        groups,
        expected_solution: None,
    }
}
