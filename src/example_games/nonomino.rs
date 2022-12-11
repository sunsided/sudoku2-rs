use crate::prelude::*;

/// Produces an example Nonomino game.
///
/// Unlike regular Sudokus, Nonominos trade the blocks with
/// nine irregularly shaped regions of nine cells each.
///
/// ## Initial state
/// ```plain
///     3 · ·   · · ·   · · 4
///     · · 2   · 6 ·   1 · ·
///     · 1 ·   9 · 8   · 2 ·
///
///     · · 5   · · ·   6 · ·
///     · 2 ·   · · ·   · 1 ·
///     · · 9   · · ·   8 · ·
///
///     · 8 ·   3 · 4   · 6 ·
///     · · 4   · 1 ·   9 · ·
///     5 · ·   · · ·   · · 7
/// ```
///
/// ## Expected solution
/// ```plain
///     3 5 8   1 9 6   2 7 4
///     4 9 2   5 6 7   1 3 8
///     6 1 3   9 7 8   4 2 5
///
///     1 7 5   8 4 2   6 9 3
///     8 2 6   4 5 3   7 1 9
///     2 4 9   7 3 1   8 5 6
///
///     9 8 7   3 2 4   5 6 1
///     7 3 4   6 1 5   9 8 2
///     5 6 1   2 8 9   3 4 7
/// ```
#[rustfmt::skip]
pub fn example_nonomino() -> Game {
    let mut groups = CellGroups::default()
        .with_group(CellGroup::from_u8_slice(&[0, 1, 2, 9, 10, 11, 18, 27, 28]))
        .with_group(CellGroup::from_u8_slice(&[3, 12, 13, 14, 23, 24, 25, 34, 35]))
        .with_group(CellGroup::from_u8_slice(&[4, 5, 6, 7, 8, 15, 16, 17, 26]))
        .with_group(CellGroup::from_u8_slice(&[19, 20, 21, 22, 29, 36, 37, 38, 39]))
        .with_group(CellGroup::from_u8_slice(&[30, 31, 32, 33, 40, 47, 48, 49, 50]))
        .with_group(CellGroup::from_u8_slice(&[41, 42, 43, 44, 51, 58, 59, 60, 61]))
        .with_group(CellGroup::from_u8_slice(&[45, 46, 55, 56, 57, 66, 67, 68, 77]))
        .with_group(CellGroup::from_u8_slice(&[54, 63, 64, 65, 72, 73, 74, 75, 76]))
        .with_group(CellGroup::from_u8_slice(&[52, 53, 62, 69, 70, 71, 78, 79, 80]))
        .with_default_rows_and_columns();

    let x = 0u8;
    let state = GameState::new_from([
        3, x, x,   x, x, x,   x, x, 4,
        x, x, 2,   x, 6, x,   1, x, x,
        x, 1, x,   9, x, 8,   x, 2, x,

        x, x, 5,   x, x, x,   6, x, x,
        x, 2, x,   x, x, x,   x, 1, x,
        x, x, 9,   x, x, x,   8, x, x,

        x, 8, x,   3, x, 4,   x, 6, x,
        x, x, 4,   x, 1, x,   9, x, x,
        5, x, x,   x, x, x,   x, x, 7,
    ],);

    let solution = GameState::new_from([
        3, 5, 8,   1, 9, 6,   2, 7, 4,
        4, 9, 2,   5, 6, 7,   1, 3, 8,
        6, 1, 3,   9, 7, 8,   4, 2, 5,

        1, 7, 5,   8, 4, 2,   6, 9, 3,
        8, 2, 6,   4, 5, 3,   7, 1, 9,
        2, 4, 9,   7, 3, 1,   8, 5, 6,

        9, 8, 7,   3, 2, 4,   5, 6, 1,
        7, 3, 4,   6, 1, 5,   9, 8, 2,
        5, 6, 1,   2, 8, 9,   3, 4, 7,
    ],);

    Game {
        initial_state: state,
        groups,
        expected_solution: Some(solution),
    }
}
