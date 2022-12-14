use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::{Index, IndexBitSet};
use crate::strategies::{Strategy, StrategyResult};
use crate::value::ValueBitSet;
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes naked twins.
///
/// ## Example
/// A naked twin is a pair of cells that share the same values.
///
/// Given three cells with the values `3 5`, `3 4` and `3 4`,
/// `3 4` are the naked twins. Since they must appear in the last two
/// cells, the `3` can be removed from the first cell.
pub struct NakedTwins {
    enabled: bool,
}

impl NakedTwins {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for NakedTwins {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Naked twins")
    }
}

impl Strategy for NakedTwins {
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
        let mut twins_to_remove = Vec::default();
        let mut observed_twins = IndexBitSet::empty();

        for cell_under_test in state.iter_indexed() {
            if !observed_twins.try_insert(cell_under_test.index) {
                continue;
            }

            // Only consider cells that have two possible candidates.
            if cell_under_test.len() != 2 {
                continue;
            }

            let mut possible_twins = Vec::default();

            // Find all possible twin candidates.
            for index in groups.get_peer_indexes(cell_under_test.index, group_type) {
                if observed_twins.contains(index) {
                    continue;
                }

                let cell = state.get_at_index(index);
                if cell.len() != 2 {
                    continue;
                }

                if cell.to_bitset().eq(cell_under_test.as_bitset()) {
                    possible_twins.push(cell.into_indexed(index));
                }
            }

            // At least one other cell is required for a twin pair.
            if possible_twins.len() < 1 {
                continue;
            }

            // More than two "twins" are an error.
            if possible_twins.len() > 1 {
                return Err(InvalidGameState {});
            }

            debug_assert_eq!(possible_twins.len(), 1);
            let other_twin = possible_twins.iter().next().unwrap();

            // Eliminate twin values in other cells.
            observed_twins
                .insert(cell_under_test.index)
                .insert(other_twin.index);

            trace!(
                "Identified Naked Twin pair in {group_type:?} at {a:?} and {b:?}: {values:?}",
                group_type = group_type,
                a = cell_under_test.index.min(other_twin.index),
                b = cell_under_test.index.max(other_twin.index),
                values = other_twin.to_bitset()
            );
            twins_to_remove.push(TwinPair {
                smaller: cell_under_test.index.min(other_twin.index),
                larger: cell_under_test.index.max(other_twin.index),
                values: other_twin.to_bitset(),
            });
        }

        if twins_to_remove.is_empty() {
            return Ok(StrategyResult::NoChange);
        }

        // Iterate the detected twins, find their groups and eliminate the values.
        let mut applied_some = false;
        for twin in twins_to_remove.into_iter() {
            // The choice of the smaller or larger index here doesn't matter as they
            // are in the same group.
            let mut applied_twin = false;
            for index in groups
                .get_peer_indexes(twin.smaller, group_type)
                .filter(|&x| x != twin.smaller && x != twin.larger)
            {
                applied_twin |= state.forget_many_at_index(index, twin.values);
            }

            if applied_twin {
                debug!(
                    "Applied Naked Twin at {a:?} and {b:?}: {values:?}",
                    a = twin.smaller,
                    b = twin.larger,
                    values = twin.values
                );
            }

            applied_some |= applied_twin;
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            trace!(
                "No Naked Twins could be applied in {group_type:?}",
                group_type = group_type
            );
            Ok(StrategyResult::NoChange)
        }
    }
}

struct TwinPair {
    smaller: Index,
    larger: Index,
    values: ValueBitSet,
}
