use crate::cell_group::{CellGroupType, CellGroups, CollectIndexes};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::{Index, IndexBitSet};
use crate::strategies::{Strategy, StrategyResult};
use log::debug;
use std::fmt::{Debug, Formatter};

/// Identifies and realizes naked singles.
///
/// ## Notes
/// Playing this strategy is required because other strategies may
/// collapse the candidate space of a cell into a singular value. This
/// however does not automatically manifest the move, i.e. the value
/// is not propagated to the board. This strategy does just that: Identify
/// singles and ensure they are correctly propagated.
#[derive(Default)]
pub struct NakedSingles {}

impl NakedSingles {
    pub fn new_box() -> Box<Self> {
        Box::new(Self::default())
    }
}

impl Debug for NakedSingles {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Naked singles")
    }
}

impl Strategy for NakedSingles {
    fn always_continue(&self) -> bool {
        true
    }

    fn apply(
        &self,
        state: &GameState,
        groups: &CellGroups,
    ) -> Result<StrategyResult, InvalidGameState> {
        let mut observed_singles = IndexBitSet::empty();
        let mut removed_some = false;

        for index_under_test in Index::range() {
            if !observed_singles.try_insert(index_under_test) {
                continue;
            }

            // Only consider cells that have exactly one value.
            let cell_under_test = state.get_at_index(index_under_test);
            if !cell_under_test.is_solved() {
                continue;
            }

            // Find all peers candidates.
            let mut applied_single = false;
            for index in groups
                .get_peers_at_index(index_under_test, CollectIndexes::ExcludeSelf)
                .unwrap()
                .into_iter()
            {
                debug_assert_ne!(index, index_under_test);
                applied_single |= state.forget_many_at_index(index, cell_under_test.to_bitset());
            }

            removed_some |= applied_single;
            if applied_single {
                debug!(
                    "Applied Naked Single {value:?} at {iut:?}",
                    value = cell_under_test.to_bitset(),
                    iut = index_under_test
                );
            }
        }

        if removed_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            Ok(StrategyResult::NoChange)
        }
    }

    fn apply_in_group(
        &self,
        _state: &GameState,
        _groups: &CellGroups,
        _group_type: CellGroupType,
    ) -> Result<StrategyResult, InvalidGameState> {
        unimplemented!("This strategy is not group aware")
    }
}
