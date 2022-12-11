use crate::cell_group::CellGroups;
use crate::game_state::{GameState, InvalidGameState};
use crate::index::{Index, IndexBitSet};
use crate::strategies::Strategy;
use log::debug;

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

impl Strategy for NakedSingles {
    fn always_continue(&self) -> bool {
        true
    }

    fn apply(&self, state: &GameState, groups: &CellGroups) -> Result<bool, InvalidGameState> {
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
            for index in groups.get_at_index(index_under_test, false).unwrap().iter() {
                debug_assert_ne!(index, index_under_test);

                let cell = state.get_at_index(index);
                if cell.as_bitset().contains_all(cell_under_test.as_bitset()) {
                    debug!(
                        "Removing naked single {value:?} at {index:?} (single at {iut:?})",
                        value = cell_under_test.as_bitset(),
                        index = index,
                        iut = index_under_test
                    );
                    removed_some = true;
                }

                state.forget_many_at_index(index, cell_under_test.as_bitset());
            }
        }

        return Ok(removed_some);
    }
}
