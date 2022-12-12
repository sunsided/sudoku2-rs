use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::{Index, IndexBitSet};
use crate::strategies::{Strategy, StrategyResult};
use crate::value::ValueBitSet;
use log::debug;
use std::fmt::{Debug, Formatter};

/// Identifies and realizes naked twins.
///
/// ## Example
/// A naked twin is a pair of cells that share the same values.
///
/// Given three cells with the values `3 5`, `3 4` and `3 4`,
/// `3 4` are the naked twins. Since they must appear in the last two
/// cells, the `3` can be removed from the first cell.
#[derive(Default)]
pub struct NakedTwins {}

impl NakedTwins {
    pub fn new_box() -> Box<Self> {
        Box::new(Self::default())
    }
}

impl Debug for NakedTwins {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Naked twins")
    }
}

impl Strategy for NakedTwins {
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

        for index_under_test in Index::range() {
            if !observed_twins.try_insert(index_under_test) {
                continue;
            }

            // Only consider cells that have two possible candidates.
            let cell_under_test = state.get_at_index(index_under_test);
            if cell_under_test.len() != 2 {
                continue;
            }

            let mut possible_twins = Vec::default();

            // Find all possible twin candidates.
            for group in groups
                .get_groups_at_index(index_under_test)
                .unwrap()
                .iter()
                .filter(|g| g.group_type == group_type)
            {
                for index in group.iter_indexes() {
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
                .insert(index_under_test)
                .insert(other_twin.index);

            debug!(
                "Twin pair detected in {group_type:?} at {a:?} and {b:?}: {values:?}",
                group_type = group_type,
                a = index_under_test.min(other_twin.index),
                b = index_under_test.max(other_twin.index),
                values = other_twin.to_bitset()
            );
            twins_to_remove.push(TwinPair {
                smaller: index_under_test.min(other_twin.index),
                larger: index_under_test.max(other_twin.index),
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
            for index in groups
                .get_groups_at_index(twin.smaller)
                .unwrap()
                .iter()
                .filter(|g| g.group_type == group_type)
                .flat_map(|g| g.iter_indexes())
                .filter(|&x| x != twin.smaller && x != twin.larger)
            {
                applied_some |= state.forget_many_at_index(index, twin.values);
            }
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            Ok(StrategyResult::NoChange)
        }
    }
}

struct TwinPair {
    smaller: Index,
    larger: Index,
    values: ValueBitSet,
}
