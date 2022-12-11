use crate::prelude::*;

/// Produces an example Sudoku game.
///
/// ## Initial state
/// ```plain
///     5 3 ·   · 7 ·   · · ·
///     6 · ·   1 9 5   · · ·
///     · 9 8   · · ·   · 6 ·
///
///     8 · ·   · 6 ·   · · 3
///     4 · ·   8 · 3   · · 1
///     7 · ·   · 2 ·   · · 6
///
///     · 6 ·   · · ·   2 8 ·
///     · · ·   4 1 9   · · 5
///     · · ·   · 8 ·   · 7 9
/// ```
///
/// ## Expected solution
/// ```plain
///     5 3 4   6 7 8   9 1 2
///     6 7 2   1 9 5   3 4 8
///     1 9 8   3 4 2   5 6 7
///
///     8 5 9   7 6 1   4 2 3
///     4 2 6   8 5 3   7 9 1
///     7 1 3   9 2 4   8 5 6
///
///     9 6 1   5 3 7   2 8 4
///     2 8 7   4 1 9   6 3 5
///     3 4 5   2 8 6   1 7 9
/// ```
pub fn example_sudoku() -> Game {
    let groups = CellGroups::default()
        .with_default_sudoku_blocks()
        .with_default_rows_and_columns();

    let state = GameState::new();
    state.place_at_xy(0, 0, Value::FIVE, &groups);
    state.place_at_xy(1, 0, Value::THREE, &groups);
    // state.place_at_xy(2, 0, Value::FOUR, &groups);
    // state.place_at_xy(3, 0, Value::SIX, &groups);
    state.place_at_xy(4, 0, Value::SEVEN, &groups);
    // state.place_at_xy(5, 0, Value::EIGHT, &groups);
    // state.place_at_xy(6, 0, Value::NINE, &groups);
    // state.place_at_xy(7, 0, Value::ONE, &groups);
    // state.place_at_xy(8, 0, Value::TWO, &groups);

    state.place_at_xy(0, 1, Value::SIX, &groups);
    // state.place_at_xy(1, 1, Value::SEVEN, &groups);
    // state.place_at_xy(2, 1, Value::TWO, &groups);
    state.place_at_xy(3, 1, Value::ONE, &groups);
    state.place_at_xy(4, 1, Value::NINE, &groups);
    state.place_at_xy(5, 1, Value::FIVE, &groups);
    // state.place_at_xy(6, 1, Value::THREE, &groups);
    // state.place_at_xy(7, 1, Value::FOUR, &groups);
    // state.place_at_xy(8, 1, Value::EIGHT, &groups);
    // the board until here produces naked triples

    // state.place_at_xy(0, 2, Value::ONE, &groups);
    state.place_at_xy(1, 2, Value::NINE, &groups);
    state.place_at_xy(2, 2, Value::EIGHT, &groups);
    // state.place_at_xy(3, 2, Value::THREE, &groups);
    // state.place_at_xy(4, 2, Value::FOUR, &groups);
    // state.place_at_xy(5, 2, Value::TWO, &groups);
    // state.place_at_xy(6, 2, Value::FIVE, &groups);
    state.place_at_xy(7, 2, Value::SIX, &groups);
    // state.place_at_xy(8, 2, Value::SEVEN, &groups);

    state.place_at_xy(0, 3, Value::EIGHT, &groups);
    // state.place_at_xy(1, 3, Value::FIVE, &groups);
    // state.place_at_xy(2, 3, Value::NINE, &groups);
    // state.place_at_xy(3, 3, Value::SEVEN, &groups);
    state.place_at_xy(4, 3, Value::SIX, &groups);
    // state.place_at_xy(5, 3, Value::ONE, &groups);
    // state.place_at_xy(6, 3, Value::FOUR, &groups);
    // state.place_at_xy(7, 3, Value::TWO, &groups);
    state.place_at_xy(8, 3, Value::THREE, &groups);

    state.place_at_xy(0, 4, Value::FOUR, &groups);
    // state.place_at_xy(1, 4, Value::TWO, &groups);
    // state.place_at_xy(2, 4, Value::SIX, &groups);
    state.place_at_xy(3, 4, Value::EIGHT, &groups);
    // state.place_at_xy(4, 4, Value::FIVE, &groups);
    state.place_at_xy(5, 4, Value::THREE, &groups);
    // state.place_at_xy(6, 4, Value::SEVEN, &groups);
    // state.place_at_xy(7, 4, Value::NINE, &groups);
    state.place_at_xy(8, 4, Value::ONE, &groups);

    state.place_at_xy(0, 5, Value::SEVEN, &groups);
    // state.place_at_xy(1, 5, Value::ONE, &groups);
    // state.place_at_xy(2, 5, Value::THREE, &groups);
    // state.place_at_xy(3, 5, Value::NINE, &groups);
    state.place_at_xy(4, 5, Value::TWO, &groups);
    // state.place_at_xy(5, 5, Value::FOUR, &groups);
    // state.place_at_xy(6, 5, Value::EIGHT, &groups);
    // state.place_at_xy(7, 5, Value::FIVE, &groups);
    state.place_at_xy(8, 5, Value::SIX, &groups);

    // state.place_at_xy(0, 6, Value::NINE, &groups);
    state.place_at_xy(1, 6, Value::SIX, &groups);
    // state.place_at_xy(2, 6, Value::ONE, &groups);
    // state.place_at_xy(3, 6, Value::FIVE, &groups);
    // state.place_at_xy(4, 6, Value::THREE, &groups);
    // state.place_at_xy(5, 6, Value::SEVEN, &groups);
    state.place_at_xy(6, 6, Value::TWO, &groups);
    state.place_at_xy(7, 6, Value::EIGHT, &groups);
    // state.place_at_xy(8, 6, Value::FOUR, &groups);
    // at this stage, naked twins are available

    // state.place_at_xy(0, 7, Value::TWO, &groups);
    // state.place_at_xy(1, 7, Value::EIGHT, &groups);
    // state.place_at_xy(2, 7, Value::SEVEN, &groups);
    state.place_at_xy(3, 7, Value::FOUR, &groups);
    state.place_at_xy(4, 7, Value::ONE, &groups);
    state.place_at_xy(5, 7, Value::NINE, &groups);
    // state.place_at_xy(6, 7, Value::SIX, &groups);
    // state.place_at_xy(7, 7, Value::THREE, &groups);
    state.place_at_xy(8, 7, Value::FIVE, &groups);
    // at this point, the board is solved. The remaining row is only for reference.

    // state.place_at_xy(0, 8, Value::THREE, &groups);
    // state.place_at_xy(1, 8, Value::FOUR, &groups);
    // state.place_at_xy(2, 8, Value::FIVE, &groups);
    // state.place_at_xy(3, 8, Value::TWO, &groups);
    state.place_at_xy(4, 8, Value::EIGHT, &groups);
    // state.place_at_xy(5, 8, Value::SIX, &groups);
    // state.place_at_xy(6, 8, Value::ONE, &groups);
    state.place_at_xy(7, 8, Value::SEVEN, &groups);
    state.place_at_xy(8, 8, Value::NINE, &groups);

    // Solution
    let solution = GameState::new();
    solution.place_at_xy(0, 0, Value::FIVE, &groups);
    solution.place_at_xy(1, 0, Value::THREE, &groups);
    solution.place_at_xy(2, 0, Value::FOUR, &groups);
    solution.place_at_xy(3, 0, Value::SIX, &groups);
    solution.place_at_xy(4, 0, Value::SEVEN, &groups);
    solution.place_at_xy(5, 0, Value::EIGHT, &groups);
    solution.place_at_xy(6, 0, Value::NINE, &groups);
    solution.place_at_xy(7, 0, Value::ONE, &groups);
    solution.place_at_xy(8, 0, Value::TWO, &groups);

    solution.place_at_xy(0, 1, Value::SIX, &groups);
    solution.place_at_xy(1, 1, Value::SEVEN, &groups);
    solution.place_at_xy(2, 1, Value::TWO, &groups);
    solution.place_at_xy(3, 1, Value::ONE, &groups);
    solution.place_at_xy(4, 1, Value::NINE, &groups);
    solution.place_at_xy(5, 1, Value::FIVE, &groups);
    solution.place_at_xy(6, 1, Value::THREE, &groups);
    solution.place_at_xy(7, 1, Value::FOUR, &groups);
    solution.place_at_xy(8, 1, Value::EIGHT, &groups);

    solution.place_at_xy(0, 2, Value::ONE, &groups);
    solution.place_at_xy(1, 2, Value::NINE, &groups);
    solution.place_at_xy(2, 2, Value::EIGHT, &groups);
    solution.place_at_xy(3, 2, Value::THREE, &groups);
    solution.place_at_xy(4, 2, Value::FOUR, &groups);
    solution.place_at_xy(5, 2, Value::TWO, &groups);
    solution.place_at_xy(6, 2, Value::FIVE, &groups);
    solution.place_at_xy(7, 2, Value::SIX, &groups);
    solution.place_at_xy(8, 2, Value::SEVEN, &groups);

    solution.place_at_xy(0, 3, Value::EIGHT, &groups);
    solution.place_at_xy(1, 3, Value::FIVE, &groups);
    solution.place_at_xy(2, 3, Value::NINE, &groups);
    solution.place_at_xy(3, 3, Value::SEVEN, &groups);
    solution.place_at_xy(4, 3, Value::SIX, &groups);
    solution.place_at_xy(5, 3, Value::ONE, &groups);
    solution.place_at_xy(6, 3, Value::FOUR, &groups);
    solution.place_at_xy(7, 3, Value::TWO, &groups);
    solution.place_at_xy(8, 3, Value::THREE, &groups);

    solution.place_at_xy(0, 4, Value::FOUR, &groups);
    solution.place_at_xy(1, 4, Value::TWO, &groups);
    solution.place_at_xy(2, 4, Value::SIX, &groups);
    solution.place_at_xy(3, 4, Value::EIGHT, &groups);
    solution.place_at_xy(4, 4, Value::FIVE, &groups);
    solution.place_at_xy(5, 4, Value::THREE, &groups);
    solution.place_at_xy(6, 4, Value::SEVEN, &groups);
    solution.place_at_xy(7, 4, Value::NINE, &groups);
    solution.place_at_xy(8, 4, Value::ONE, &groups);

    solution.place_at_xy(0, 5, Value::SEVEN, &groups);
    solution.place_at_xy(1, 5, Value::ONE, &groups);
    solution.place_at_xy(2, 5, Value::THREE, &groups);
    solution.place_at_xy(3, 5, Value::NINE, &groups);
    solution.place_at_xy(4, 5, Value::TWO, &groups);
    solution.place_at_xy(5, 5, Value::FOUR, &groups);
    solution.place_at_xy(6, 5, Value::EIGHT, &groups);
    solution.place_at_xy(7, 5, Value::FIVE, &groups);
    solution.place_at_xy(8, 5, Value::SIX, &groups);

    solution.place_at_xy(0, 6, Value::NINE, &groups);
    solution.place_at_xy(1, 6, Value::SIX, &groups);
    solution.place_at_xy(2, 6, Value::ONE, &groups);
    solution.place_at_xy(3, 6, Value::FIVE, &groups);
    solution.place_at_xy(4, 6, Value::THREE, &groups);
    solution.place_at_xy(5, 6, Value::SEVEN, &groups);
    solution.place_at_xy(6, 6, Value::TWO, &groups);
    solution.place_at_xy(7, 6, Value::EIGHT, &groups);
    solution.place_at_xy(8, 6, Value::FOUR, &groups);

    solution.place_at_xy(0, 7, Value::TWO, &groups);
    solution.place_at_xy(1, 7, Value::EIGHT, &groups);
    solution.place_at_xy(2, 7, Value::SEVEN, &groups);
    solution.place_at_xy(3, 7, Value::FOUR, &groups);
    solution.place_at_xy(4, 7, Value::ONE, &groups);
    solution.place_at_xy(5, 7, Value::NINE, &groups);
    solution.place_at_xy(6, 7, Value::SIX, &groups);
    solution.place_at_xy(7, 7, Value::THREE, &groups);
    solution.place_at_xy(8, 7, Value::FIVE, &groups);

    solution.place_at_xy(0, 8, Value::THREE, &groups);
    solution.place_at_xy(1, 8, Value::FOUR, &groups);
    solution.place_at_xy(2, 8, Value::FIVE, &groups);
    solution.place_at_xy(3, 8, Value::TWO, &groups);
    solution.place_at_xy(4, 8, Value::EIGHT, &groups);
    solution.place_at_xy(5, 8, Value::SIX, &groups);
    solution.place_at_xy(6, 8, Value::ONE, &groups);
    solution.place_at_xy(7, 8, Value::SEVEN, &groups);
    solution.place_at_xy(8, 8, Value::NINE, &groups);

    Game {
        initial_state: state,
        groups,
        expected_solution: Some(solution),
    }
}
