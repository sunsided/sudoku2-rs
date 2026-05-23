use crate::board_stats::BoardStatsCache;
use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::Index;
use crate::strategies::{Strategy, StrategyResult};
use crate::value::Value;
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
        _stats: &BoardStatsCache,
        group_type: CellGroupType,
    ) -> Result<StrategyResult, InvalidGameState> {
        let mut applied_some = false;

        for group in groups.iter().filter(|g| g.group_type == group_type) {
            // Read each cell once and fan its candidates into per-value
            // position arrays. A hidden single is any value that appears in
            // exactly one unsolved cell of the group AND is not already
            // resolved in some other cell of the group.
            //
            // Solved cells must still contribute to `counts` (otherwise a
            // value resolved in cell C looks "absent" from the group, and a
            // not-yet-propagated candidate elsewhere could be misread as the
            // unique home for that digit). They never become a placement
            // target, so `single_index` is only set from unsolved cells.
            //
            // The previous shape walked every cell of the group from the
            // perspective of `Index::range()` and re-scanned peers per cell;
            // this inversion cuts the cell-reads-per-group from ~9*8 to 9 and
            // exposes the same answer directly.
            let mut single_index: [Option<Index>; 9] = [None; 9];
            let mut counts: [u8; 9] = [0; 9];
            for i in group.iter_indexes() {
                let cell = state.get_at_index(i);
                let solved = cell.len() == 1;
                for value in cell.to_bitset().iter() {
                    let v_idx = (value.get() - 1) as usize;
                    if !solved && single_index[v_idx].is_none() {
                        single_index[v_idx] = Some(i);
                    }
                    counts[v_idx] = counts[v_idx].saturating_add(1);
                }
            }

            // Apply each value whose unsolved footprint is exactly one cell.
            for (v_idx, value) in Value::range().enumerate() {
                if counts[v_idx] != 1 {
                    continue;
                }
                let Some(index) = single_index[v_idx] else {
                    // The lone contributor was a solved cell - the value is
                    // already placed, no work to do.
                    continue;
                };
                // Guard against over-constrained states: if a preceding
                // placement in this loop already solved this cell to a
                // different value, the board is contradictory.
                let cell = state.get_at_index(index);
                if !cell.contains(value) {
                    return Err(InvalidGameState {});
                }
                if state.place_and_propagate_at_index(index, value, groups) {
                    debug!(
                        "Applied Hidden Single {value:?} at {iut:?}",
                        value = value,
                        iut = index
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board_stats::BoardStatsCache;
    use crate::cell_group::CellGroups;
    use crate::game_state::GameState;
    use crate::Coordinate;

    fn forget_at(state: &GameState, x: u8, y: u8, v: Value) {
        state.forget_at_index(Coordinate::new(x, y).into_index(), v);
    }

    #[test]
    fn contradiction_in_group_returns_err() {
        // Cell (0,0) is the only home for both value 1 and value 2 in row 0.
        // HiddenSingles will place 1 (solving the cell), then find 2 missing
        // from that same cell and must return Err rather than panic.
        let groups = CellGroups::default().with_default_rows_and_columns();
        let state = GameState::new();
        for x in 1u8..9 {
            forget_at(&state, x, 0, Value::ONE);
            forget_at(&state, x, 0, Value::TWO);
        }
        for v in Value::range().skip(2) {
            forget_at(&state, 0, 0, v);
        }
        let stats = BoardStatsCache::new(&state);
        let result = HiddenSingles::new_box(true).apply_in_group(
            &state,
            &groups,
            &stats,
            CellGroupType::StandardRow,
        );
        assert!(result.is_err());
    }
}
