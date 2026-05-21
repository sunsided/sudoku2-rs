#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use sudoku2::{CellGroups, CollectIndexes, GameState, Index, Value};

fn sudoku_groups() -> &'static CellGroups {
    static GROUPS: OnceLock<CellGroups> = OnceLock::new();
    GROUPS.get_or_init(|| {
        CellGroups::default()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns()
    })
}

fn to_value(byte: u8) -> Option<Value> {
    Value::try_from((byte % 9) + 1).ok()
}

fn to_index(byte: u8) -> Index {
    Index::new(byte % 81)
}

fuzz_target!(|data: &[u8]| {
    let groups = sudoku_groups();
    let state = GameState::new();

    for chunk in data.chunks_exact(3) {
        let op = chunk[0];
        let index = to_index(chunk[1]);
        let value = match to_value(chunk[2]) {
            Some(v) => v,
            None => continue,
        };

        match op % 3 {
            0 => {
                let cell = state.get_at_index(index);
                if !cell.contains(value) || cell.is_solved() {
                    continue;
                }
                state.place_and_propagate_at_index(index, value, groups);

                let after = state.get_at_index(index);
                assert!(after.is_solved());
                assert!(after.contains(value));

                let peers = groups
                    .get_peers_at_index(index, CollectIndexes::ExcludeSelf)
                    .expect("peers must exist for valid index");
                for peer in peers.iter() {
                    let peer_cell = state.get_at_index(peer);
                    assert!(
                        !peer_cell.contains(value),
                        "propagation failed: peer still contains placed value"
                    );
                }
            }
            1 => {
                let cell = state.get_at_index(index);
                if !cell.contains(value) {
                    continue;
                }
                state.forget_at_index(index, value);
                assert!(!state.get_at_index(index).contains(value));
            }
            2 => {
                let _ = state.is_consistent(groups);
                let _ = state.is_solved(groups);
            }
            _ => unreachable!(),
        }

        for idx in Index::range() {
            let cell = state.get_at_index(idx);
            assert!(cell.len() <= 9);
        }
    }
});
