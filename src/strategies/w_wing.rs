use crate::cell_group::{CellGroupType, CellGroups, CollectIndexes};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::Index;
use crate::strategies::{Strategy, StrategyResult};
use crate::value::{Value, ValueBitSet};
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes the W-Wing strategy.
///
/// A W-Wing is built from two non-peer bivalue cells that share the same
/// candidate set `{x, y}`, connected by a strong link on `x`:
/// - `bivalue_a` with candidates `{x, y}`,
/// - `bivalue_b` with candidates `{x, y}` (not a peer of `bivalue_a`),
/// - a group (row, column, or block) in which `x` appears in exactly two
///   candidate cells `link_a` and `link_b`, where `link_a` is a peer of
///   `bivalue_a` and `link_b` is a peer of `bivalue_b` (or vice versa) and
///   neither link cell coincides with either bivalue cell.
///
/// Whichever bivalue ends up holding `x`, the strong link forces the other
/// bivalue to hold `y`. Every cell that sees **both** bivalue cells therefore
/// cannot contain `y`.
///
/// Registered after the XY-Wing strategy in the default pipeline.
pub struct WWing {
    enabled: bool,
}

impl WWing {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for WWing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "W-Wing")
    }
}

impl Strategy for WWing {
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn always_continue(&self) -> bool {
        false
    }

    fn apply(
        &self,
        state: &GameState,
        groups: &CellGroups,
    ) -> Result<StrategyResult, InvalidGameState> {
        // Collect every bivalue cell on the board once per invocation.
        let mut bivalues: Vec<Bivalue> = Vec::with_capacity(32);
        for cell in state.iter_indexed() {
            if cell.len() == 2 {
                bivalues.push(Bivalue {
                    index: cell.index,
                    values: cell.to_bitset(),
                });
            }
        }

        // A W-Wing requires at least two bivalue cells.
        if bivalues.len() < 2 {
            return Ok(StrategyResult::NoChange);
        }

        // Enumerate the strong links on each candidate value. A strong link is
        // a group in which `value` is a candidate in exactly two cells.
        let mut strong_links: Vec<StrongLink> = Vec::with_capacity(32);
        for value in Value::range() {
            for group in groups.iter() {
                let mut endpoints = [Index::default(); 2];
                let mut count = 0usize;
                for index in group.iter_indexes() {
                    if state.get_at_index(index).contains(value) {
                        if count < 2 {
                            endpoints[count] = index;
                        }
                        count += 1;
                        if count > 2 {
                            break;
                        }
                    }
                }
                if count == 2 {
                    strong_links.push(StrongLink {
                        value,
                        a: endpoints[0],
                        b: endpoints[1],
                    });
                }
            }
        }

        if strong_links.is_empty() {
            return Ok(StrategyResult::NoChange);
        }

        let mut hits: Vec<WWingHit> = Vec::default();

        for (i, biv_a) in bivalues.iter().enumerate() {
            let peers_a = groups
                .get_peers_at_index(biv_a.index, CollectIndexes::ExcludeSelf)
                .expect("group missing for bivalue cell");

            for biv_b in bivalues.iter().skip(i + 1) {
                // The two bivalue cells must share the same candidate pair.
                if biv_a.values != biv_b.values {
                    continue;
                }

                // W-Wing requires non-peer bivalue cells; otherwise the pair
                // is a naked pair handled by other strategies.
                if peers_a.contains(biv_b.index) {
                    continue;
                }

                let peers_b = groups
                    .get_peers_at_index(biv_b.index, CollectIndexes::ExcludeSelf)
                    .expect("group missing for bivalue cell");

                for &x in &[
                    biv_a.values.iter().next().unwrap(),
                    biv_a.values.iter().nth(1).unwrap(),
                ] {
                    // `y` is the other candidate, to be eliminated.
                    let mut y_bits = biv_a.values;
                    y_bits.remove(x);
                    let y = match y_bits.as_single_value() {
                        Some(v) => v,
                        None => continue,
                    };

                    for link in strong_links.iter().filter(|l| l.value == x) {
                        // Strong-link endpoints must be distinct from the
                        // bivalue cells. If one of them coincides, the
                        // pattern degenerates and is covered elsewhere.
                        if link.a == biv_a.index
                            || link.a == biv_b.index
                            || link.b == biv_a.index
                            || link.b == biv_b.index
                        {
                            continue;
                        }

                        let a_sees_la = peers_a.contains(link.a);
                        let b_sees_lb = peers_b.contains(link.b);
                        let a_sees_lb = peers_a.contains(link.b);
                        let b_sees_la = peers_b.contains(link.a);

                        let matched = (a_sees_la && b_sees_lb) || (a_sees_lb && b_sees_la);
                        if !matched {
                            continue;
                        }

                        trace!(
                            "Identified W-Wing on candidate {x:?} between {ba:?} and {bb:?} via link {la:?}-{lb:?}, eliminating {y:?}",
                            x = x,
                            ba = biv_a.index,
                            bb = biv_b.index,
                            la = link.a,
                            lb = link.b,
                            y = y,
                        );

                        hits.push(WWingHit {
                            bivalue_a: biv_a.index,
                            bivalue_b: biv_b.index,
                            y,
                        });
                    }
                }
            }
        }

        if hits.is_empty() {
            return Ok(StrategyResult::NoChange);
        }

        let mut applied_some = false;
        for hit in hits {
            let peers_a = groups
                .get_peers_at_index(hit.bivalue_a, CollectIndexes::ExcludeSelf)
                .expect("group missing for bivalue cell");
            let peers_b = groups
                .get_peers_at_index(hit.bivalue_b, CollectIndexes::ExcludeSelf)
                .expect("group missing for bivalue cell");

            let common = peers_a.intersect(&peers_b);

            let mut applied = false;
            for idx in common.iter() {
                applied |= state.forget_at_index(idx, hit.y);
            }

            applied_some |= applied;
            if applied {
                debug!(
                    "Applied W-Wing between {ba:?} and {bb:?} eliminating {y:?}",
                    ba = hit.bivalue_a,
                    bb = hit.bivalue_b,
                    y = hit.y,
                );
            }
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            trace!("No W-Wing eliminations applied");
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

#[derive(Copy, Clone)]
struct Bivalue {
    index: Index,
    values: ValueBitSet,
}

#[derive(Copy, Clone)]
struct StrongLink {
    value: Value,
    a: Index,
    b: Index,
}

struct WWingHit {
    bivalue_a: Index,
    bivalue_b: Index,
    y: Value,
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

    /// Forces `(x, y)` to hold exactly the listed candidate set.
    fn set_candidates(state: &GameState, x: u8, y: u8, keep: &[Value]) {
        let index = Coordinate::new(x, y).into_index();
        for v in Value::range() {
            if !keep.contains(&v) {
                state.forget_at_index(index, v);
            }
        }
    }

    /// Removes the supplied candidate at `(x, y)`.
    fn forget(state: &GameState, x: u8, y: u8, value: Value) {
        let index = Coordinate::new(x, y).into_index();
        state.forget_at_index(index, value);
    }

    /// Bivalues at (0,0) and (8,8) both {1,2}. They are non-peers.
    /// Row 4 has value 1 confined to (0,4) and (8,4) - a strong link on 1.
    /// (0,4) is a peer of (0,0) via column 0; (8,4) is a peer of (8,8) via
    /// column 8. The W-Wing therefore eliminates 2 from any cell that sees
    /// both bivalue cells.
    #[test]
    fn w_wing_eliminates_y_at_common_peer() {
        let groups = standard_groups();
        let state = GameState::new();

        set_candidates(&state, 0, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 8, 8, &[Value::ONE, Value::TWO]);

        // Confine value 1 in row 4 to columns 0 and 8 only.
        for x in [1u8, 2, 3, 4, 5, 6, 7] {
            forget(&state, x, 4, Value::ONE);
        }

        let strat = WWing { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // (0,8) and (8,0) each see both bivalues (column + row) and must
        // drop the `y` candidate (2).
        assert!(!state.get_at_xy(0, 8).contains(Value::TWO));
        assert!(!state.get_at_xy(8, 0).contains(Value::TWO));

        // The bivalue cells themselves keep both candidates.
        assert!(state.get_at_xy(0, 0).contains(Value::ONE));
        assert!(state.get_at_xy(0, 0).contains(Value::TWO));
        assert!(state.get_at_xy(8, 8).contains(Value::ONE));
        assert!(state.get_at_xy(8, 8).contains(Value::TWO));

        // Strong-link endpoints keep the linked candidate.
        assert!(state.get_at_xy(0, 4).contains(Value::ONE));
        assert!(state.get_at_xy(8, 4).contains(Value::ONE));
    }

    /// Without a strong link on either shared candidate, two bivalue cells
    /// with the same pair must not trigger a W-Wing.
    #[test]
    fn no_strong_link_no_w_wing() {
        let groups = standard_groups();
        let state = GameState::new();

        set_candidates(&state, 0, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 8, 8, &[Value::ONE, Value::TWO]);

        let strat = WWing { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }

    /// Two bivalue cells sharing a peer group form a naked pair, not a
    /// W-Wing - the strategy must abstain.
    #[test]
    fn peer_bivalues_are_not_a_w_wing() {
        let groups = standard_groups();
        let state = GameState::new();

        set_candidates(&state, 0, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 1, 0, &[Value::ONE, Value::TWO]);

        // Even with a strong link present elsewhere, peer bivalues must not
        // qualify as a W-Wing.
        for x in [1u8, 2, 3, 4, 5, 6, 7] {
            forget(&state, x, 4, Value::ONE);
        }

        let strat = WWing { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }

    #[test]
    fn no_change_on_pristine_board() {
        let groups = standard_groups();
        let state = GameState::new();
        let strat = WWing { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }
}
