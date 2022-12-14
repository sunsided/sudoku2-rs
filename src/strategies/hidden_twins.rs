use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::Index;
use crate::strategies::{Strategy, StrategyResult};
use crate::ValueBitSet;
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes Hidden Twins.
///
/// ## Example
/// A single is a value that does not appear in any other cell.
/// It is hidden when it appears along other values.
///
/// Given two cells with the values `5 7`, `3 4 5` and `3 4 7`,
/// `3 4` is the hidden twin. Since `3 4` only appear in the
/// second and third cell they must be placed there, eliminating
/// `5` and `7` from those cells.
pub struct HiddenTwins {
    enabled: bool,
}

impl HiddenTwins {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for HiddenTwins {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hidden singles")
    }
}

impl Strategy for HiddenTwins {
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
        let mut twins = Vec::default();
        'next_index: for index_under_test in Index::range() {
            // For the pair to work, we need to ensure that the values under test
            // exist in only two columns. For that, we register all seen values
            // in the below set.
            let mut values_observed = ValueBitSet::empty();

            // Hidden twins "hide" behind more than one other
            // possible value; we want to exclude impossible cells
            // and those that are already solved.
            let cell_under_test = state.get_at_index(index_under_test);
            // A Hidden Twin can pair with a Naked Twin, so we only
            // filter out cells that have less than two values.
            let cell_under_test_len = cell_under_test.len();
            if cell_under_test_len < 2 {
                continue;
            }

            // Find all peers candidates.
            for index in groups
                .get_peer_indexes(index_under_test, group_type)
                .filter(|&i| i > index_under_test)
            {
                // Exactly two values need to be shared for it to be a twin.
                let peer_values = state.get_at_index(index).to_bitset();
                let values = peer_values.with_intersection(cell_under_test.to_bitset());
                if values.len() != 2 {
                    continue;
                }

                // Skip (possible) Naked Twins as they are already covered.
                // TODO: Add configuration option for strictly hidden twins?
                if cell_under_test_len == 2 && peer_values.len() == 2 {
                    trace!(
                        "Skipping Naked Twin in {group_type:?}",
                        group_type = group_type
                    );
                    continue 'next_index;
                }

                // If the values existed in previously observed cells, this
                // is not a proper twin pair.
                if values_observed.contains_some(values) {
                    continue 'next_index;
                }
                values_observed.union(values);

                // Ensure that no other cell in the same group shares any values with the twins.
                if groups
                    .get_peer_indexes(index_under_test, group_type)
                    .filter(|&idx| idx != index_under_test && idx != index)
                    .map(|idx| state.get_at_index(idx))
                    .any(|cell| cell.contains_some(values))
                {
                    continue 'next_index;
                }

                trace!(
                    "Identified Hidden Twins {twins:?} in {lhs:?} at {iut:?} and {rhs:?} at {index:?}",
                    lhs = cell_under_test.as_bitset(),
                    iut = index_under_test,
                    rhs = state.get_at_index(index).to_bitset(),
                    index = index,
                    twins = values
                );

                twins.push(TwinPair {
                    smaller: index_under_test.min(index),
                    larger: index_under_test.max(index),
                    values,
                })
            }
        }

        if twins.is_empty() {
            return Ok(StrategyResult::NoChange);
        }

        let mut applied_some = false;
        for twin in twins {
            let set_smaller = state.set_many_at_index(twin.smaller, twin.values);
            let set_larger = state.set_many_at_index(twin.larger, twin.values);
            if set_smaller || set_larger {
                applied_some = true;
                debug!(
                    "Applied Hidden Twins {twins:?} at {lhs:?} and {rhs:?}",
                    lhs = twin.smaller,
                    rhs = twin.larger,
                    twins = twin.values
                );
            }
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            trace!(
                "No Hidden Twins could be applied in {group_type:?}",
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
