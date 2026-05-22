use crate::*;

/// Produces an example state that exercises the XY-Wing strategy.
///
/// The board itself is otherwise unconstrained: only three bivalue cells are
/// pre-seeded so the XY-Wing pattern can be observed in isolation.
///
/// Pattern:
/// - Pivot at `(0, 0)` carries `{1, 2}`.
/// - Pincer A at `(5, 0)` (shares row 0 with the pivot) carries `{1, 3}`.
/// - Pincer B at `(0, 5)` (shares column 0 with the pivot) carries `{2, 3}`.
///
/// XY-Wing then eliminates `3` from any cell that sees both pincers - most
/// visibly from `(5, 5)`.
pub fn example_sudoku() -> Game {
    let groups = CellGroups::default()
        .with_default_sudoku_blocks()
        .with_default_rows_and_columns();

    let state = GameState::new();

    let pivot = Coordinate::new(0, 0).into_index();
    let pincer_a = Coordinate::new(5, 0).into_index();
    let pincer_b = Coordinate::new(0, 5).into_index();

    for v in Value::range() {
        if v != Value::ONE && v != Value::TWO {
            state.forget_at_index(pivot, v);
        }
        if v != Value::ONE && v != Value::THREE {
            state.forget_at_index(pincer_a, v);
        }
        if v != Value::TWO && v != Value::THREE {
            state.forget_at_index(pincer_b, v);
        }
    }

    Game {
        initial_state: state,
        groups,
        expected_solution: None,
    }
}
