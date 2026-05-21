#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use sudoku2::{CellGroups, DefaultSolver, GameState, Index, Value};

fn sudoku_groups() -> &'static CellGroups {
    static GROUPS: OnceLock<CellGroups> = OnceLock::new();
    GROUPS.get_or_init(|| {
        CellGroups::default()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns()
    })
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 81 {
        return;
    }

    let groups = sudoku_groups();

    let state = GameState::new();
    for (i, byte) in data.iter().take(81).enumerate() {
        let v = byte % 10;
        if v == 0 {
            continue;
        }
        let value = match Value::try_from(v) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let index = Index::new(i as u8);
        if !state.get_at_index(index).contains(value) {
            continue;
        }
        state.place_and_propagate_at_index(index, value, groups);
        if !state.is_consistent(groups) {
            return;
        }
    }

    if !state.is_consistent(groups) {
        return;
    }

    let solver = DefaultSolver::new(groups);
    if let Ok(solution) = solver.solve(&state) {
        assert!(
            solution.is_consistent(groups),
            "solver produced inconsistent solution"
        );
        assert!(
            solution.is_solved(groups),
            "solver returned Ok but board is not solved"
        );
    }
});
