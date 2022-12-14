use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::Index;
use crate::strategies::{Strategy, StrategyResult};
use log::debug;
use std::fmt::{Debug, Formatter};

/// Identifies and realizes hidden singles.
///
/// ## Example
/// A single is a value that does not appear in any other cell.
/// It is hidden when it appears along other values.
///
/// Given two cells with the values `3 4` and `3 4 7`,
/// `7` is the hidden single. Since it only appears in the second
/// cell, it must be placed there (resulting in a "naked twin" pair of `3 4`).
pub struct HiddenSingles {
    enabled: bool,
}

impl HiddenSingles {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for HiddenSingles {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hidden singles")
    }
}

impl Strategy for HiddenSingles {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn always_continue(&self) -> bool {
        false
    }

    fn apply_in_group(
        &self,
        state: &GameState,
        groups: &CellGroups,
        group_type: CellGroupType,
    ) -> Result<StrategyResult, InvalidGameState> {
        let mut applied_some = false;

        for index_under_test in Index::range() {
            // Hidden singles "hide" behind more than one other
            // possible value; we want to exclude impossible cells
            // and those that are already solved.
            let cell_under_test = state.get_at_index(index_under_test);
            if cell_under_test.len() <= 1 {
                continue;
            }

            // By taking the intersection with each peer, we will isolate
            // values that appear only in this cell and nowhere else.
            let mut values = cell_under_test.to_bitset();

            // Find all peers candidates.
            for index in groups
                .get_peer_indexes(index_under_test, group_type)
                .filter(|&i| i != index_under_test)
            {
                debug_assert_ne!(index, index_under_test);
                values.remove_many(state.get_at_index(index).to_bitset());
            }

            if let Some(value) = values.as_single_value() {
                if state.place_and_propagate_at_index(index_under_test, value, &groups) {
                    debug!(
                        "Applied Hidden Single {value:?} at {iut:?}",
                        value = value,
                        iut = index_under_test
                    );
                    applied_some = true;
                }
            }
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            Ok(StrategyResult::NoChange)
        }
    }
}
