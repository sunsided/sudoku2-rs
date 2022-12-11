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
    state.set_at_xy(0, 0, Value::FIVE);
    state.set_at_xy(1, 0, Value::THREE);
    state.set_at_xy(4, 0, Value::SEVEN);

    state.set_at_xy(0, 1, Value::SIX);
    state.set_at_xy(3, 1, Value::ONE);
    state.set_at_xy(4, 1, Value::NINE);
    state.set_at_xy(5, 1, Value::FIVE);

    state.set_at_xy(1, 2, Value::NINE);
    state.set_at_xy(2, 2, Value::EIGHT);
    state.set_at_xy(7, 2, Value::SIX);

    state.set_at_xy(0, 3, Value::EIGHT);
    state.set_at_xy(4, 3, Value::SIX);
    state.set_at_xy(8, 3, Value::THREE);

    state.set_at_xy(0, 4, Value::FOUR);
    state.set_at_xy(3, 4, Value::EIGHT);
    state.set_at_xy(5, 4, Value::THREE);
    state.set_at_xy(8, 4, Value::ONE);

    state.set_at_xy(0, 5, Value::SEVEN);
    state.set_at_xy(4, 5, Value::TWO);
    state.set_at_xy(8, 5, Value::SIX);

    state.set_at_xy(1, 6, Value::SIX);
    state.set_at_xy(6, 6, Value::TWO);
    state.set_at_xy(7, 6, Value::EIGHT);

    state.set_at_xy(3, 7, Value::FOUR);
    state.set_at_xy(4, 7, Value::ONE);
    state.set_at_xy(5, 7, Value::NINE);
    state.set_at_xy(8, 7, Value::FIVE);

    state.set_at_xy(4, 8, Value::EIGHT);
    state.set_at_xy(7, 8, Value::SEVEN);
    state.set_at_xy(8, 8, Value::NINE);

    // Solution
    let solution = build_example_solution();

    Game {
        initial_state: state,
        groups,
        expected_solution: Some(solution),
    }
}

/// Produces an example Sudoku game (possibly unsolvable) with naked twins.
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
///     · · ·   · · ·   · · ·
///     · · ·   · · ·   · · ·
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
pub fn example_sudoku_naked_twins() -> Game {
    let groups = CellGroups::default()
        .with_default_sudoku_blocks()
        .with_default_rows_and_columns();

    let state = GameState::new();
    state.set_at_xy(0, 0, Value::FIVE);
    state.set_at_xy(1, 0, Value::THREE);
    state.set_at_xy(2, 0, Value::FOUR);
    state.set_at_xy(3, 0, Value::SIX);
    state.set_at_xy(4, 0, Value::SEVEN);
    state.set_at_xy(5, 0, Value::EIGHT);
    state.set_at_xy(6, 0, Value::NINE);
    state.set_at_xy(7, 0, Value::ONE);
    state.set_at_xy(8, 0, Value::TWO);

    state.set_at_xy(0, 1, Value::SIX);
    state.set_at_xy(1, 1, Value::SEVEN);
    state.set_at_xy(2, 1, Value::TWO);
    state.set_at_xy(3, 1, Value::ONE);
    state.set_at_xy(4, 1, Value::NINE);
    state.set_at_xy(5, 1, Value::FIVE);
    state.set_at_xy(6, 1, Value::THREE);
    state.set_at_xy(7, 1, Value::FOUR);
    state.set_at_xy(8, 1, Value::EIGHT);
    // the board until here produces naked triples

    state.set_at_xy(0, 2, Value::ONE);
    state.set_at_xy(1, 2, Value::NINE);
    state.set_at_xy(2, 2, Value::EIGHT);
    state.set_at_xy(3, 2, Value::THREE);
    state.set_at_xy(4, 2, Value::FOUR);
    state.set_at_xy(5, 2, Value::TWO);
    state.set_at_xy(6, 2, Value::FIVE);
    state.set_at_xy(7, 2, Value::SIX);
    state.set_at_xy(8, 2, Value::SEVEN);

    state.set_at_xy(0, 3, Value::EIGHT);
    state.set_at_xy(1, 3, Value::FIVE);
    state.set_at_xy(2, 3, Value::NINE);
    state.set_at_xy(3, 3, Value::SEVEN);
    state.set_at_xy(4, 3, Value::SIX);
    state.set_at_xy(5, 3, Value::ONE);
    state.set_at_xy(6, 3, Value::FOUR);
    state.set_at_xy(7, 3, Value::TWO);
    state.set_at_xy(8, 3, Value::THREE);

    state.set_at_xy(0, 4, Value::FOUR);
    state.set_at_xy(1, 4, Value::TWO);
    state.set_at_xy(2, 4, Value::SIX);
    state.set_at_xy(3, 4, Value::EIGHT);
    state.set_at_xy(4, 4, Value::FIVE);
    state.set_at_xy(5, 4, Value::THREE);
    state.set_at_xy(6, 4, Value::SEVEN);
    state.set_at_xy(7, 4, Value::NINE);
    state.set_at_xy(8, 4, Value::ONE);

    state.set_at_xy(0, 5, Value::SEVEN);
    state.set_at_xy(1, 5, Value::ONE);
    state.set_at_xy(2, 5, Value::THREE);
    state.set_at_xy(3, 5, Value::NINE);
    state.set_at_xy(4, 5, Value::TWO);
    state.set_at_xy(5, 5, Value::FOUR);
    state.set_at_xy(6, 5, Value::EIGHT);
    state.set_at_xy(7, 5, Value::FIVE);
    state.set_at_xy(8, 5, Value::SIX);

    state.set_at_xy(0, 6, Value::NINE);
    state.set_at_xy(1, 6, Value::SIX);
    state.set_at_xy(2, 6, Value::ONE);
    state.set_at_xy(3, 6, Value::FIVE);
    state.set_at_xy(4, 6, Value::THREE);
    state.set_at_xy(5, 6, Value::SEVEN);
    state.set_at_xy(6, 6, Value::TWO);
    state.set_at_xy(7, 6, Value::EIGHT);
    state.set_at_xy(8, 6, Value::FOUR);
    // at this stage, naked twins are available

    // Solution
    let solution = build_example_solution();

    Game {
        initial_state: state,
        groups,
        expected_solution: Some(solution),
    }
}

/// Produces an example Sudoku game (possibly unsolvable) with naked triples.
///
/// ## Initial state
/// ```plain
///     5 3 ·   · 7 ·   · · ·
///     6 · ·   1 9 5   · · ·
///     · · ·   · · ·   · · ·
///
///     · · ·   · · ·   · · ·
///     · · ·   · · ·   · · ·
///     · · ·   · · ·   · · ·
///
///     · · ·   · · ·   · · ·
///     · · ·   · · ·   · · ·
///     · · ·   · · ·   · · ·
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
#[allow(dead_code)]
pub(crate) fn example_sudoku_naked_triples() -> Game {
    let groups = CellGroups::default()
        .with_default_sudoku_blocks()
        .with_default_rows_and_columns();

    let state = GameState::new();
    state.set_at_xy(0, 0, Value::FIVE);
    state.set_at_xy(1, 0, Value::THREE);
    state.set_at_xy(2, 0, Value::FOUR);
    state.set_at_xy(3, 0, Value::SIX);
    state.set_at_xy(4, 0, Value::SEVEN);
    state.set_at_xy(5, 0, Value::EIGHT);
    state.set_at_xy(6, 0, Value::NINE);
    state.set_at_xy(7, 0, Value::ONE);
    state.set_at_xy(8, 0, Value::TWO);

    state.set_at_xy(0, 1, Value::SIX);
    state.set_at_xy(1, 1, Value::SEVEN);
    state.set_at_xy(2, 1, Value::TWO);
    state.set_at_xy(3, 1, Value::ONE);
    state.set_at_xy(4, 1, Value::NINE);
    state.set_at_xy(5, 1, Value::FIVE);
    state.set_at_xy(6, 1, Value::THREE);
    state.set_at_xy(7, 1, Value::FOUR);
    state.set_at_xy(8, 1, Value::EIGHT);
    // the board until here produces naked triples

    // Solution
    let solution = build_example_solution();

    Game {
        initial_state: state,
        groups,
        expected_solution: Some(solution),
    }
}

fn build_example_solution() -> GameState {
    let solution = GameState::new();
    solution.set_at_xy(0, 0, Value::FIVE);
    solution.set_at_xy(1, 0, Value::THREE);
    solution.set_at_xy(2, 0, Value::FOUR);
    solution.set_at_xy(3, 0, Value::SIX);
    solution.set_at_xy(4, 0, Value::SEVEN);
    solution.set_at_xy(5, 0, Value::EIGHT);
    solution.set_at_xy(6, 0, Value::NINE);
    solution.set_at_xy(7, 0, Value::ONE);
    solution.set_at_xy(8, 0, Value::TWO);

    solution.set_at_xy(0, 1, Value::SIX);
    solution.set_at_xy(1, 1, Value::SEVEN);
    solution.set_at_xy(2, 1, Value::TWO);
    solution.set_at_xy(3, 1, Value::ONE);
    solution.set_at_xy(4, 1, Value::NINE);
    solution.set_at_xy(5, 1, Value::FIVE);
    solution.set_at_xy(6, 1, Value::THREE);
    solution.set_at_xy(7, 1, Value::FOUR);
    solution.set_at_xy(8, 1, Value::EIGHT);

    solution.set_at_xy(0, 2, Value::ONE);
    solution.set_at_xy(1, 2, Value::NINE);
    solution.set_at_xy(2, 2, Value::EIGHT);
    solution.set_at_xy(3, 2, Value::THREE);
    solution.set_at_xy(4, 2, Value::FOUR);
    solution.set_at_xy(5, 2, Value::TWO);
    solution.set_at_xy(6, 2, Value::FIVE);
    solution.set_at_xy(7, 2, Value::SIX);
    solution.set_at_xy(8, 2, Value::SEVEN);

    solution.set_at_xy(0, 3, Value::EIGHT);
    solution.set_at_xy(1, 3, Value::FIVE);
    solution.set_at_xy(2, 3, Value::NINE);
    solution.set_at_xy(3, 3, Value::SEVEN);
    solution.set_at_xy(4, 3, Value::SIX);
    solution.set_at_xy(5, 3, Value::ONE);
    solution.set_at_xy(6, 3, Value::FOUR);
    solution.set_at_xy(7, 3, Value::TWO);
    solution.set_at_xy(8, 3, Value::THREE);

    solution.set_at_xy(0, 4, Value::FOUR);
    solution.set_at_xy(1, 4, Value::TWO);
    solution.set_at_xy(2, 4, Value::SIX);
    solution.set_at_xy(3, 4, Value::EIGHT);
    solution.set_at_xy(4, 4, Value::FIVE);
    solution.set_at_xy(5, 4, Value::THREE);
    solution.set_at_xy(6, 4, Value::SEVEN);
    solution.set_at_xy(7, 4, Value::NINE);
    solution.set_at_xy(8, 4, Value::ONE);

    solution.set_at_xy(0, 5, Value::SEVEN);
    solution.set_at_xy(1, 5, Value::ONE);
    solution.set_at_xy(2, 5, Value::THREE);
    solution.set_at_xy(3, 5, Value::NINE);
    solution.set_at_xy(4, 5, Value::TWO);
    solution.set_at_xy(5, 5, Value::FOUR);
    solution.set_at_xy(6, 5, Value::EIGHT);
    solution.set_at_xy(7, 5, Value::FIVE);
    solution.set_at_xy(8, 5, Value::SIX);

    solution.set_at_xy(0, 6, Value::NINE);
    solution.set_at_xy(1, 6, Value::SIX);
    solution.set_at_xy(2, 6, Value::ONE);
    solution.set_at_xy(3, 6, Value::FIVE);
    solution.set_at_xy(4, 6, Value::THREE);
    solution.set_at_xy(5, 6, Value::SEVEN);
    solution.set_at_xy(6, 6, Value::TWO);
    solution.set_at_xy(7, 6, Value::EIGHT);
    solution.set_at_xy(8, 6, Value::FOUR);

    solution.set_at_xy(0, 7, Value::TWO);
    solution.set_at_xy(1, 7, Value::EIGHT);
    solution.set_at_xy(2, 7, Value::SEVEN);
    solution.set_at_xy(3, 7, Value::FOUR);
    solution.set_at_xy(4, 7, Value::ONE);
    solution.set_at_xy(5, 7, Value::NINE);
    solution.set_at_xy(6, 7, Value::SIX);
    solution.set_at_xy(7, 7, Value::THREE);
    solution.set_at_xy(8, 7, Value::FIVE);

    // The last row is completely defined by now; rows added here only for completeness.
    solution.set_at_xy(0, 8, Value::THREE);
    solution.set_at_xy(1, 8, Value::FOUR);
    solution.set_at_xy(2, 8, Value::FIVE);
    solution.set_at_xy(3, 8, Value::TWO);
    solution.set_at_xy(4, 8, Value::EIGHT);
    solution.set_at_xy(5, 8, Value::SIX);
    solution.set_at_xy(6, 8, Value::ONE);
    solution.set_at_xy(7, 8, Value::SEVEN);
    solution.set_at_xy(8, 8, Value::NINE);
    solution
}
