use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::Index;
use crate::strategies::{Strategy, StrategyResult};
use crate::value::ValueBitSet;
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes Naked Triples.
///
/// ## Example
/// A naked triple is a set of three cells in a peer group whose combined
/// candidate set has cardinality exactly three. Each of the three cells
/// holds two or three of those candidates.
///
/// Given five cells with the values `2 5`, `2 7`, `5 7`, `1 2 5 7` and
/// `3 5 7`, the first three cells form a Naked Triple on `{2, 5, 7}`.
/// Those three values can be removed from the remaining cells of the
/// peer group.
pub struct NakedTriples {
    enabled: bool,
}

impl NakedTriples {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for NakedTriples {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Naked triples")
    }
}

impl Strategy for NakedTriples {
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

        for group in groups.iter().filter(|g| g.group_type == group_type) {
            // Collect cells in this group whose candidate count is two or three.
            let candidates: Vec<(Index, ValueBitSet)> = group
                .iter_indexes()
                .map(|i| (i, state.get_at_index(i).to_bitset()))
                .filter(|(_, v)| {
                    let n = v.len();
                    n == 2 || n == 3
                })
                .collect();

            if candidates.len() < 3 {
                continue;
            }

            let n = candidates.len();
            for i in 0..n {
                for j in (i + 1)..n {
                    let union_ij = candidates[i].1.with_union(candidates[j].1);
                    if union_ij.len() > 3 {
                        continue;
                    }

                    for k in (j + 1)..n {
                        let union_ijk = union_ij.with_union(candidates[k].1);
                        if union_ijk.len() != 3 {
                            continue;
                        }

                        // Detect inconsistency: more than three cells with candidates
                        // entirely contained in this 3-value union cannot coexist.
                        for (m, (_, values)) in candidates.iter().enumerate() {
                            if m == i || m == j || m == k {
                                continue;
                            }
                            if union_ijk.contains_all(*values) {
                                return Err(InvalidGameState {});
                            }
                        }

                        let triple = [candidates[i].0, candidates[j].0, candidates[k].0];
                        trace!(
                            "Identified Naked Triple in {group_type:?} at {a:?}, {b:?}, {c:?}: {values:?}",
                            group_type = group_type,
                            a = triple[0],
                            b = triple[1],
                            c = triple[2],
                            values = union_ijk
                        );

                        let mut applied_here = false;
                        for index in group.iter_indexes() {
                            if triple.contains(&index) {
                                continue;
                            }
                            applied_here |= state.forget_many_at_index(index, union_ijk);
                        }

                        if applied_here {
                            debug!(
                                "Applied Naked Triple at {a:?}, {b:?}, {c:?}: {values:?}",
                                a = triple[0],
                                b = triple[1],
                                c = triple[2],
                                values = union_ijk
                            );
                            applied_some = true;
                        }
                    }
                }
            }
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            trace!(
                "No Naked Triples could be applied in {group_type:?}",
                group_type = group_type
            );
            Ok(StrategyResult::NoChange)
        }
    }
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

        // Row 0: three cells holding only {2,5,7} candidates.
        restrict(&state, 0, 0, &[Value::TWO, Value::FIVE]);
        restrict(&state, 1, 0, &[Value::TWO, Value::SEVEN]);
        restrict(&state, 2, 0, &[Value::FIVE, Value::SEVEN]);

        let strat = NakedTriples { enabled: true };
        let res = strat
            .apply_in_group(&state, &groups, CellGroupType::StandardRow)
            .unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // 2/5/7 must be gone from the rest of row 0.
        let triple = ValueBitSet::from([Value::TWO, Value::FIVE, Value::SEVEN].as_slice());
        for x in 3u8..9 {
            let cell = state.get_at_xy(x, 0).to_bitset();
            assert!(
                !cell.contains_some(triple),
                "row peer ({x},0) still carries triple candidates: {cell:?}",
                x = x,
                cell = cell
            );
        }

        // Triple cells themselves are untouched.
        assert_eq!(state.get_at_xy(0, 0).len(), 2);
        assert_eq!(state.get_at_xy(1, 0).len(), 2);
        assert_eq!(state.get_at_xy(2, 0).len(), 2);
    }

    #[test]
    fn detects_inconsistency_when_four_cells_share_three_values() {
        let groups = standard_groups();
        let state = GameState::new();

        // Four cells in row 0 confined to {2,5,7}: impossible.
        restrict(&state, 0, 0, &[Value::TWO, Value::FIVE]);
        restrict(&state, 1, 0, &[Value::TWO, Value::SEVEN]);
        restrict(&state, 2, 0, &[Value::FIVE, Value::SEVEN]);
        restrict(&state, 3, 0, &[Value::TWO, Value::FIVE, Value::SEVEN]);

        let strat = NakedTriples { enabled: true };
        let res = strat.apply_in_group(&state, &groups, CellGroupType::StandardRow);
        assert!(res.is_err());
    }

    #[test]
    fn no_change_when_no_triple_present() {
        let groups = standard_groups();
        let state = GameState::new();

        let strat = NakedTriples { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }
}
