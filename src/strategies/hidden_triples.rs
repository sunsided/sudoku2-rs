use crate::board_stats::BoardStatsCache;
use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::IndexBitSet;
use crate::strategies::{Strategy, StrategyResult};
use crate::value::{Value, ValueBitSet};
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes Hidden Triples.
///
/// ## Example
/// A hidden triple is a set of three digits whose only candidate positions
/// inside a peer group lie in exactly three cells. Those cells may carry
/// additional candidates, all of which can be eliminated.
///
/// Suppose a row contains the cells `{1, 2, 3, 5, 6}`, `{1, 2, 3}` and
/// `{5, 6, 9}`, and the digits `2`, `3` and `5` do not appear as
/// candidates anywhere else in that row. The union of their candidate
/// positions is exactly these three cells, so the three cells must hold
/// `{2, 3, 5}` in some order. The other candidates are eliminated,
/// leaving `{2, 3, 5}`, `{2, 3}` and `{5}` respectively.
pub struct HiddenTriples {
    enabled: bool,
}

impl HiddenTriples {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for HiddenTriples {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hidden triples")
    }
}

impl Strategy for HiddenTriples {
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
        let mut triples_to_apply: Vec<HiddenTriple> = Vec::default();

        for group in groups.iter().filter(|g| g.group_type == group_type) {
            // Read each cell in the group once and fan its candidates out into
            // per-value position sets. The earlier shape iterated values in the
            // outer loop and re-read every cell nine times; this inversion cuts
            // the cell-reads-per-group from 81 to 9 and reaches the same data.
            //
            // A digit that fits in 1, 2, or 3 cells is eligible for a hidden
            // triple - allowing 1-cell digits means we do not depend on
            // `HiddenSingles` having run beforehand. Digits with more than
            // three positions cannot be confined to a 3-cell cover.
            let mut positions: [IndexBitSet; 9] = Default::default();
            let mut counts: [u8; 9] = [0; 9];
            for i in group.iter_indexes() {
                let cell = state.get_at_index(i);
                if cell.len() <= 1 {
                    continue;
                }
                for value in cell.to_bitset().iter() {
                    let v_idx = (value.get() - 1) as usize;
                    positions[v_idx].insert(i);
                    counts[v_idx] += 1;
                }
            }

            let mut candidate_values: Vec<(Value, IndexBitSet)> = Vec::with_capacity(9);
            for (v_idx, value) in Value::range().enumerate() {
                let count = counts[v_idx];
                if (1..=3).contains(&count) {
                    candidate_values.push((value, positions[v_idx]));
                }
            }

            if candidate_values.len() < 3 {
                continue;
            }

            let n = candidate_values.len();
            for i in 0..n {
                for j in (i + 1)..n {
                    let pos_ij = candidate_values[i].1.with_union(&candidate_values[j].1);
                    if pos_ij.len() > 3 {
                        continue;
                    }

                    for k in (j + 1)..n {
                        let pos_ijk = pos_ij.with_union(&candidate_values[k].1);
                        if pos_ijk.len() != 3 {
                            continue;
                        }

                        let values = ValueBitSet::empty()
                            .with_value(candidate_values[i].0)
                            .with_value(candidate_values[j].0)
                            .with_value(candidate_values[k].0);

                        trace!(
                            "Identified Hidden Triple in {group_type:?}: values {values:?} confined to 3 cells",
                            group_type = group_type,
                            values = values
                        );

                        triples_to_apply.push(HiddenTriple {
                            indexes: pos_ijk,
                            values,
                        });
                    }
                }
            }
        }

        if triples_to_apply.is_empty() {
            trace!(
                "No Hidden Triples could be applied in {group_type:?}",
                group_type = group_type
            );
            return Ok(StrategyResult::NoChange);
        }

        // Apply eliminations after detection so the detection pass reads a
        // consistent candidate snapshot. Restrict each cell to the
        // intersection of its current candidates with the triple values:
        // simply assigning `triple.values` could re-introduce candidates
        // earlier strategies had eliminated, since a cell may only carry a
        // proper subset of the triple.
        //
        // Overlapping triples detected in the same pass can shrink a cell
        // mid-loop. If a later triple's intersection turns out to be empty
        // the position-set snapshot is no longer consistent with the
        // current board, so we surface that as an invalid state and let
        // the solver back out of the branch.
        let mut applied_some = false;
        for triple in triples_to_apply {
            let mut applied_here = false;
            for index in triple.indexes.iter() {
                let current = state.get_at_index(index).to_bitset();
                let kept = current.with_intersection(triple.values);
                if kept.is_empty() {
                    return Err(InvalidGameState {});
                }
                if !current.eq(&kept) {
                    state.set_many_at_index(index, kept);
                    applied_here = true;
                }
            }

            if applied_here {
                debug!(
                    "Applied Hidden Triple at {indexes:?}: {values:?}",
                    indexes = triple.indexes,
                    values = triple.values
                );
                applied_some = true;
            }
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            Ok(StrategyResult::NoChange)
        }
    }
}

struct HiddenTriple {
    indexes: IndexBitSet,
    values: ValueBitSet,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell_group::CellGroups;
    use crate::value::Value;
    use crate::{Coordinate, GameState};

    fn standard_groups() -> CellGroups {
        CellGroups::default()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns()
    }

    /// Removes the supplied candidates at `(x, y)`.
    fn forget(state: &GameState, x: u8, y: u8, drop: &[Value]) {
        let index = Coordinate::new(x, y).into_index();
        for v in drop {
            state.forget_at_index(index, *v);
        }
    }

    /// Keeps only the supplied candidates at `(x, y)`.
    fn restrict(state: &GameState, x: u8, y: u8, keep: &[Value]) {
        let index = Coordinate::new(x, y).into_index();
        let keep_set = ValueBitSet::from(keep);
        for v in Value::range() {
            if !keep_set.contains(v) {
                state.forget_at_index(index, v);
            }
        }
    }

    #[test]
    fn applies_in_row() {
        let groups = standard_groups();
        let state = GameState::new();

        // Confine values {2, 5, 7} to cells (0,0), (1,0), (2,0) in row 0
        // by removing them from cells (3,0) .. (8,0). The triple cells keep
        // their full candidate sets so the hidden triple is masked by other
        // candidates.
        for x in 3u8..9 {
            forget(&state, x, 0, &[Value::TWO, Value::FIVE, Value::SEVEN]);
        }

        let strat = HiddenTriples { enabled: true };
        let res = strat
            .apply_in_group(
                &state,
                &groups,
                &BoardStatsCache::new(&state),
                CellGroupType::StandardRow,
            )
            .unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // The three cells must now hold only {2, 5, 7}.
        let triple = ValueBitSet::from([Value::TWO, Value::FIVE, Value::SEVEN].as_slice());
        for x in 0u8..3 {
            let cell = state.get_at_xy(x, 0).to_bitset();
            assert_eq!(
                cell,
                triple,
                "triple cell ({x},0) should hold exactly the triple: {cell:?}",
                x = x,
                cell = cell
            );
        }
    }

    #[test]
    fn no_change_when_no_triple_present() {
        let groups = standard_groups();
        let state = GameState::new();

        let strat = HiddenTriples { enabled: true };
        let res = strat
            .apply(&state, &groups, &BoardStatsCache::new(&state))
            .unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }

    #[test]
    fn skips_when_value_already_solved() {
        let groups = standard_groups();
        let state = GameState::new();

        // Place a solved cell that should be ignored by the search and ensure
        // no spurious triple is reported on an otherwise pristine board.
        restrict(&state, 0, 0, &[Value::ONE]);

        let strat = HiddenTriples { enabled: true };
        let res = strat
            .apply_in_group(
                &state,
                &groups,
                &BoardStatsCache::new(&state),
                CellGroupType::StandardRow,
            )
            .unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }
}
