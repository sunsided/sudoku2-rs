use crate::board_stats::BoardStatsCache;
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
        _stats: &BoardStatsCache,
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
        _stats: &BoardStatsCache,
        _group_type: CellGroupType,
    ) -> Result<StrategyResult, InvalidGameState> {
        unimplemented!("This strategy is not group aware")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::example_games::sudoku::example_sudoku;
    use crate::value::Value;

    #[test]
    fn debug_label_matches() {
        assert_eq!(format!("{:?}", NakedSingles::default()), "Naked singles");
    }

    #[test]
    fn always_continue_is_true() {
        assert!(NakedSingles::default().always_continue());
    }

    #[test]
    #[should_panic(expected = "not group aware")]
    fn apply_in_group_panics() {
        let game = example_sudoku();
        let stats = BoardStatsCache::new(&game.initial_state);
        let _ = NakedSingles::default().apply_in_group(
            &game.initial_state,
            &game.groups,
            &stats,
            CellGroupType::StandardRow,
        );
    }

    #[test]
    fn apply_removes_solved_value_from_peers() {
        // Build a tiny board: only cell (0,0) is solved with value 1.
        // The naked single must wipe candidate 1 from the rest of row 0.
        let state = GameState::new();
        state.set_at_xy(0, 0, Value::ONE);
        let groups = CellGroups::default().with_default_rows_and_columns();
        let stats = BoardStatsCache::new(&state);

        let result = NakedSingles::default()
            .apply(&state, &groups, &stats)
            .expect("apply should not invalidate");
        assert_eq!(result, StrategyResult::AppliedChange);

        // Re-applying must be a no-op now.
        let again = NakedSingles::default()
            .apply(&state, &groups, &stats)
            .expect("apply should not invalidate");
        assert_eq!(again, StrategyResult::NoChange);

        // Peer (1,0) should no longer hold candidate 1.
        assert!(!state.get_at_xy(1, 0).contains(Value::ONE));
    }
}
