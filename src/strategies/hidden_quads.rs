use crate::board_stats::BoardStatsCache;
use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::IndexBitSet;
use crate::strategies::{Strategy, StrategyResult};
use crate::value::{Value, ValueBitSet};
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes Hidden Quads.
///
/// ## Example
/// A hidden quad is a set of four digits whose only candidate positions
/// inside a peer group lie in exactly four cells. Those cells may carry
/// additional candidates, all of which can be eliminated.
///
/// If the digits `1`, `2`, `3`, `4` appear as candidates only inside
/// four cells of a row, those four cells must hold `{1, 2, 3, 4}` in
/// some order. Any other candidates in those cells can be removed.
pub struct HiddenQuads {
    enabled: bool,
}

impl HiddenQuads {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for HiddenQuads {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hidden quads")
    }
}

impl Strategy for HiddenQuads {
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
        let mut quads_to_apply: Vec<HiddenQuad> = Vec::default();

        for group in groups.iter().filter(|g| g.group_type == group_type) {
            // Read each cell in the group once and fan its candidates out into
            // per-value position sets. The earlier shape iterated values in the
            // outer loop and re-read every cell nine times; this inversion cuts
            // the cell-reads-per-group from 81 to 9 and reaches the same data.
            //
            // A digit that fits in 1..=4 cells is eligible for a hidden quad -
            // allowing 1-cell digits means we do not depend on `HiddenSingles`
            // having run beforehand. Digits with more than four positions
            // cannot be confined to a 4-cell cover.
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
                if (1..=4).contains(&count) {
                    candidate_values.push((value, positions[v_idx]));
                }
            }

            if candidate_values.len() < 4 {
                continue;
            }

            let n = candidate_values.len();
            for i in 0..n {
                for j in (i + 1)..n {
                    let pos_ij = candidate_values[i].1.with_union(&candidate_values[j].1);
                    if pos_ij.len() > 4 {
                        continue;
                    }

                    for k in (j + 1)..n {
                        let pos_ijk = pos_ij.with_union(&candidate_values[k].1);
                        if pos_ijk.len() > 4 {
                            continue;
                        }

                        for l in (k + 1)..n {
                            let pos_ijkl = pos_ijk.with_union(&candidate_values[l].1);
                            if pos_ijkl.len() != 4 {
                                continue;
                            }

                            let values = ValueBitSet::empty()
                                .with_value(candidate_values[i].0)
                                .with_value(candidate_values[j].0)
                                .with_value(candidate_values[k].0)
                                .with_value(candidate_values[l].0);

                            trace!(
                                "Identified Hidden Quad in {group_type:?}: values {values:?} confined to 4 cells",
                                group_type = group_type,
                                values = values
                            );

                            quads_to_apply.push(HiddenQuad {
                                indexes: pos_ijkl,
                                values,
                            });
                        }
                    }
                }
            }
        }

        if quads_to_apply.is_empty() {
            trace!(
                "No Hidden Quads could be applied in {group_type:?}",
                group_type = group_type
            );
            return Ok(StrategyResult::NoChange);
        }

        // Apply eliminations after detection so the detection pass reads a
        // consistent candidate snapshot. Restrict each cell to the
        // intersection of its current candidates with the quad values:
        // simply assigning `quad.values` could re-introduce candidates
        // earlier strategies had eliminated, since a cell may only carry a
        // proper subset of the quad.
        //
        // Overlapping quads detected in the same pass can shrink a cell
        // mid-loop. If a later quad's intersection turns out to be empty
        // the position-set snapshot is no longer consistent with the
        // current board, so we surface that as an invalid state and let
        // the solver back out of the branch.
        let mut applied_some = false;
        for quad in quads_to_apply {
            let mut applied_here = false;
            for index in quad.indexes.iter() {
                let current = state.get_at_index(index).to_bitset();
                let kept = current.with_intersection(quad.values);
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
                    "Applied Hidden Quad at {indexes:?}: {values:?}",
                    indexes = quad.indexes,
                    values = quad.values
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

struct HiddenQuad {
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

        // Confine values {1, 2, 3, 4} to cells (0,0)..(3,0) in row 0 by
        // removing them from cells (4,0) .. (8,0). The quad cells keep their
        // full candidate sets so the hidden quad is masked by other
        // candidates.
        for x in 4u8..9 {
            forget(
                &state,
                x,
                0,
                &[Value::ONE, Value::TWO, Value::THREE, Value::FOUR],
            );
        }

        let strat = HiddenQuads { enabled: true };
        let res = strat
            .apply_in_group(
                &state,
                &groups,
                &BoardStatsCache::new(&state),
                CellGroupType::StandardRow,
            )
            .unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // The four cells must now hold only {1, 2, 3, 4}.
        let quad =
            ValueBitSet::from([Value::ONE, Value::TWO, Value::THREE, Value::FOUR].as_slice());
        for x in 0u8..4 {
            let cell = state.get_at_xy(x, 0).to_bitset();
            assert_eq!(
                cell,
                quad,
                "quad cell ({x},0) should hold exactly the quad: {cell:?}",
                x = x,
                cell = cell
            );
        }
    }

    #[test]
    fn no_change_when_no_quad_present() {
        let groups = standard_groups();
        let state = GameState::new();

        let strat = HiddenQuads { enabled: true };
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
        // no spurious quad is reported on an otherwise pristine board.
        restrict(&state, 0, 0, &[Value::ONE]);

        let strat = HiddenQuads { enabled: true };
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
