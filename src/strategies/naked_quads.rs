use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::Index;
use crate::strategies::{Strategy, StrategyResult};
use crate::value::ValueBitSet;
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes Naked Quads.
///
/// ## Example
/// A naked quad is a set of four cells in a peer group whose combined
/// candidate set has cardinality exactly four. Each of the four cells
/// holds two, three, or four of those candidates.
///
/// Given the cells `{1, 2}`, `{2, 3}`, `{3, 4}`, `{1, 4}` in a row,
/// the four cells form a Naked Quad on `{1, 2, 3, 4}`. Those four
/// values can be eliminated from every other cell of the row.
pub struct NakedQuads {
    enabled: bool,
}

impl NakedQuads {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for NakedQuads {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Naked quads")
    }
}

impl Strategy for NakedQuads {
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
        let mut quads_to_apply: Vec<Quad> = Vec::default();

        for group in groups.iter().filter(|g| g.group_type == group_type) {
            // Collect cells in this group whose candidate count is between two
            // and four. A cell with a single candidate is solved already; a
            // cell with five or more cannot be part of a four-value cover.
            let candidates: Vec<(Index, ValueBitSet)> = group
                .iter_indexes()
                .map(|i| (i, state.get_at_index(i).to_bitset()))
                .filter(|(_, v)| {
                    let n = v.len();
                    (2..=4).contains(&n)
                })
                .collect();

            if candidates.len() < 4 {
                continue;
            }

            let n = candidates.len();
            for i in 0..n {
                for j in (i + 1)..n {
                    let union_ij = candidates[i].1.with_union(candidates[j].1);
                    if union_ij.len() > 4 {
                        continue;
                    }

                    for k in (j + 1)..n {
                        let union_ijk = union_ij.with_union(candidates[k].1);
                        if union_ijk.len() > 4 {
                            continue;
                        }

                        for l in (k + 1)..n {
                            let union_ijkl = union_ijk.with_union(candidates[l].1);
                            if union_ijkl.len() != 4 {
                                continue;
                            }

                            // Detect inconsistency: more than four cells with
                            // candidates entirely contained in this 4-value
                            // union cannot coexist.
                            for (m, (_, values)) in candidates.iter().enumerate() {
                                if m == i || m == j || m == k || m == l {
                                    continue;
                                }
                                if union_ijkl.contains_all(*values) {
                                    return Err(InvalidGameState {});
                                }
                            }

                            let quad = [
                                candidates[i].0,
                                candidates[j].0,
                                candidates[k].0,
                                candidates[l].0,
                            ];
                            trace!(
                                "Identified Naked Quad in {group_type:?} at {a:?}, {b:?}, {c:?}, {d:?}: {values:?}",
                                group_type = group_type,
                                a = quad[0],
                                b = quad[1],
                                c = quad[2],
                                d = quad[3],
                                values = union_ijkl
                            );

                            quads_to_apply.push(Quad {
                                indexes: quad,
                                group_indexes: *group.indexes(),
                                values: union_ijkl,
                            });
                        }
                    }
                }
            }
        }

        if quads_to_apply.is_empty() {
            trace!(
                "No Naked Quads could be applied in {group_type:?}",
                group_type = group_type
            );
            return Ok(StrategyResult::NoChange);
        }

        // Apply collected eliminations only after the detection pass has
        // completed for this group_type, so detection always reads a
        // consistent snapshot of candidate sets.
        let mut applied_some = false;
        for quad in quads_to_apply {
            let mut applied_here = false;
            for index in quad.group_indexes.into_iter() {
                if quad.indexes.contains(&index) {
                    continue;
                }
                applied_here |= state.forget_many_at_index(index, quad.values);
            }

            if applied_here {
                debug!(
                    "Applied Naked Quad at {a:?}, {b:?}, {c:?}, {d:?}: {values:?}",
                    a = quad.indexes[0],
                    b = quad.indexes[1],
                    c = quad.indexes[2],
                    d = quad.indexes[3],
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

struct Quad {
    indexes: [Index; 4],
    group_indexes: crate::index::IndexBitSet,
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

        // Row 0: four cells confined to {1, 2, 3, 4}.
        restrict(&state, 0, 0, &[Value::ONE, Value::TWO]);
        restrict(&state, 1, 0, &[Value::TWO, Value::THREE]);
        restrict(&state, 2, 0, &[Value::THREE, Value::FOUR]);
        restrict(&state, 3, 0, &[Value::ONE, Value::FOUR]);

        let strat = NakedQuads { enabled: true };
        let res = strat
            .apply_in_group(&state, &groups, CellGroupType::StandardRow)
            .unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // 1/2/3/4 must be gone from the rest of row 0.
        let quad =
            ValueBitSet::from([Value::ONE, Value::TWO, Value::THREE, Value::FOUR].as_slice());
        for x in 4u8..9 {
            let cell = state.get_at_xy(x, 0).to_bitset();
            assert!(
                !cell.contains_some(quad),
                "row peer ({x},0) still carries quad candidates: {cell:?}",
                x = x,
                cell = cell
            );
        }

        // Quad cells themselves are untouched.
        assert_eq!(state.get_at_xy(0, 0).len(), 2);
        assert_eq!(state.get_at_xy(1, 0).len(), 2);
        assert_eq!(state.get_at_xy(2, 0).len(), 2);
        assert_eq!(state.get_at_xy(3, 0).len(), 2);
    }

    #[test]
    fn detects_inconsistency_when_five_cells_share_four_values() {
        let groups = standard_groups();
        let state = GameState::new();

        // Five cells in row 0 confined to {1, 2, 3, 4}: impossible.
        restrict(&state, 0, 0, &[Value::ONE, Value::TWO]);
        restrict(&state, 1, 0, &[Value::TWO, Value::THREE]);
        restrict(&state, 2, 0, &[Value::THREE, Value::FOUR]);
        restrict(&state, 3, 0, &[Value::ONE, Value::FOUR]);
        restrict(
            &state,
            4,
            0,
            &[Value::ONE, Value::TWO, Value::THREE, Value::FOUR],
        );

        let strat = NakedQuads { enabled: true };
        let res = strat.apply_in_group(&state, &groups, CellGroupType::StandardRow);
        assert!(res.is_err());
    }

    #[test]
    fn no_change_when_no_quad_present() {
        let groups = standard_groups();
        let state = GameState::new();

        let strat = NakedQuads { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }
}
