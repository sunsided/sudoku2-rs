use crate::prelude::*;

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
pub fn example_sudoku() -> Game {
    let groups = CellGroups::default()
        .with_default_sudoku_blocks()
        .with_default_rows_and_columns();

    // Hidden single is at 3 x 2, value 6.

    let state = GameState::new();
    state.set_at_xy(1, 0, Value::TWO);
    state.set_at_xy(2, 0, Value::EIGHT);
    state.set_at_xy(5, 0, Value::SEVEN);

    state.set_at_xy(1, 1, Value::ONE);
    state.set_at_xy(2, 1, Value::SIX);
    state.set_at_xy(4, 1, Value::EIGHT);
    state.set_at_xy(5, 1, Value::THREE);
    state.set_at_xy(7, 1, Value::SEVEN);

    state.set_at_xy(4, 2, Value::TWO);
    state.set_at_xy(6, 2, Value::EIGHT);
    state.set_at_xy(7, 2, Value::FIVE);
    state.set_at_xy(8, 2, Value::ONE);

    state.set_at_xy(0, 3, Value::ONE);
    state.set_at_xy(1, 3, Value::THREE);
    state.set_at_xy(2, 3, Value::SEVEN);
    state.set_at_xy(3, 3, Value::TWO);
    state.set_at_xy(4, 3, Value::NINE);

    state.set_at_xy(3, 4, Value::SEVEN);
    state.set_at_xy(4, 4, Value::THREE);

    state.set_at_xy(4, 5, Value::FOUR);
    state.set_at_xy(5, 5, Value::SIX);
    state.set_at_xy(6, 5, Value::THREE);
    state.set_at_xy(8, 5, Value::SEVEN);

    state.set_at_xy(0, 6, Value::TWO);
    state.set_at_xy(1, 6, Value::NINE);
    state.set_at_xy(4, 6, Value::SEVEN);

    state.set_at_xy(3, 7, Value::EIGHT);
    state.set_at_xy(4, 7, Value::SIX);
    state.set_at_xy(6, 7, Value::ONE);
    state.set_at_xy(7, 7, Value::FOUR);

    state.set_at_xy(3, 8, Value::THREE);
    state.set_at_xy(6, 8, Value::SEVEN);

    Game {
        initial_state: state,
        groups,
        expected_solution: None,
    }
}
