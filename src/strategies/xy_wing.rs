use crate::board_stats::BoardStatsCache;
use crate::cell_group::{CellGroupType, CellGroups, CollectIndexes};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::Index;
use crate::strategies::{Strategy, StrategyResult};
use crate::value::Value;
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes the XY-Wing (a.k.a. Y-Wing) strategy.
///
/// An XY-Wing is built from three bivalue cells:
/// - a `pivot` with candidates `{x, y}`,
/// - a `pincer_a` that sees the pivot with candidates `{x, z}`,
/// - a `pincer_b` that sees the pivot with candidates `{y, z}`.
///
/// The two pincers need not see each other. Whichever of `x` or `y` the
/// pivot ends up holding, one of the pincers is forced to `z`. Every cell
/// that sees **both** pincers therefore cannot contain `z`.
///
/// Registered after the X-Wing strategy in the default pipeline.
pub struct XYWing {
    enabled: bool,
}

impl XYWing {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for XYWing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "XY-Wing")
    }
}

impl Strategy for XYWing {
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
        stats: &BoardStatsCache,
    ) -> Result<StrategyResult, InvalidGameState> {
        // Bivalues come from the shared lazy cache.
        let stats_ref = stats.get();
        if stats_ref.bivalues.len() < 3 {
            return Ok(StrategyResult::NoChange);
        }
        let bivalues: &[crate::board_stats::Bivalue] = &stats_ref.bivalues;

        let mut hits: Vec<XYWingHit> = Vec::default();

        for (pivot_pos, pivot) in bivalues.iter().enumerate() {
            // Pincers must see the pivot. Take the union of peer indexes over
            // every group type the pivot participates in.
            let pivot_peers = groups
                .get_peers_at_index(pivot.index, CollectIndexes::ExcludeSelf)
                .expect("group missing for pivot cell");

            // Restrict to bivalue peers, sorted by their position in the
            // `bivalues` vector so we can deduplicate pairs with `i < j`.
            let peer_positions: Vec<usize> = bivalues
                .iter()
                .enumerate()
                .filter(|(pos, b)| *pos != pivot_pos && pivot_peers.contains(b.index))
                .map(|(pos, _)| pos)
                .collect();

            for (k, &i) in peer_positions.iter().enumerate() {
                let pa = bivalues[i];

                // pivot ∩ pa must be exactly one value (the shared `x` or `y`).
                let shared_a = pivot.values.with_intersection(pa.values);
                if shared_a.len() != 1 {
                    continue;
                }

                // The non-shared candidate of pa is the `z` we hope to eliminate.
                let mut z_a_bits = pa.values;
                z_a_bits.remove_many(shared_a);
                let z = match z_a_bits.as_single_value() {
                    Some(v) => v,
                    None => continue,
                };

                // z must not equal either of the pivot's candidates.
                if pivot.values.contains(z) {
                    continue;
                }

                for &j in &peer_positions[k + 1..] {
                    let pb = bivalues[j];

                    let shared_b = pivot.values.with_intersection(pb.values);
                    if shared_b.len() != 1 {
                        continue;
                    }
                    // The two pincers must cover both pivot candidates between
                    // them, not the same one twice.
                    if shared_b == shared_a {
                        continue;
                    }

                    let mut z_b_bits = pb.values;
                    z_b_bits.remove_many(shared_b);
                    let z_b = match z_b_bits.as_single_value() {
                        Some(v) => v,
                        None => continue,
                    };
                    if z_b != z {
                        continue;
                    }

                    trace!(
                        "Identified XY-Wing pivot {pv:?} pincers {pa:?},{pb:?} z={z:?}",
                        pv = pivot.index,
                        pa = pa.index,
                        pb = pb.index,
                        z = z,
                    );

                    hits.push(XYWingHit {
                        pivot: pivot.index,
                        pincer_a: pa.index,
                        pincer_b: pb.index,
                        z,
                    });
                }
            }
        }

        if hits.is_empty() {
            return Ok(StrategyResult::NoChange);
        }

        let mut applied_some = false;
        for hit in hits {
            let peers_a = groups
                .get_peers_at_index(hit.pincer_a, CollectIndexes::ExcludeSelf)
                .expect("group missing for pincer cell");
            let peers_b = groups
                .get_peers_at_index(hit.pincer_b, CollectIndexes::ExcludeSelf)
                .expect("group missing for pincer cell");

            // Cells that see both pincers, minus the pivot. The pincers
            // themselves are already excluded by each peer set's `ExcludeSelf`.
            let common = peers_a.intersect(&peers_b).without_index(hit.pivot);

            let mut applied = false;
            for idx in common.iter() {
                applied |= state.forget_at_index(idx, hit.z);
            }

            applied_some |= applied;
            if applied {
                debug!(
                    "Applied XY-Wing pivot {pv:?} pincers {pa:?},{pb:?} eliminating {z:?}",
                    pv = hit.pivot,
                    pa = hit.pincer_a,
                    pb = hit.pincer_b,
                    z = hit.z,
                );
            }
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            trace!("No XY-Wing eliminations applied");
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

struct XYWingHit {
    pivot: Index,
    pincer_a: Index,
    pincer_b: Index,
    z: Value,
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

    /// Pivot at (0,0) = {1,2}; pincers at (5,0) = {1,3} and (0,5) = {2,3}.
    /// Cell (5,5) sees both pincers and must drop 3.
    #[test]
    fn xy_wing_eliminates_z_at_common_peer() {
        let groups = standard_groups();
        let state = GameState::new();

        set_candidates(&state, 0, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 5, 0, &[Value::ONE, Value::THREE]);
        set_candidates(&state, 0, 5, &[Value::TWO, Value::THREE]);

        let strat = XYWing { enabled: true };
        let res = strat
            .apply(&state, &groups, &BoardStatsCache::new(&state))
            .unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // Common peer of the two pincers (other than the pivot) loses 3.
        assert!(!state.get_at_xy(5, 5).contains(Value::THREE));

        // Pattern cells retain their candidates.
        assert!(state.get_at_xy(0, 0).contains(Value::ONE));
        assert!(state.get_at_xy(0, 0).contains(Value::TWO));
        assert!(state.get_at_xy(5, 0).contains(Value::THREE));
        assert!(state.get_at_xy(0, 5).contains(Value::THREE));
    }

    #[test]
    fn no_xy_wing_on_pristine_board() {
        let groups = standard_groups();
        let state = GameState::new();
        let strat = XYWing { enabled: true };
        let res = strat
            .apply(&state, &groups, &BoardStatsCache::new(&state))
            .unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }

    /// Three bivalue cells with the same candidate set {1,2} are a naked
    /// triple, not an XY-Wing. The strategy must not fire.
    #[test]
    fn naked_pair_is_not_an_xy_wing() {
        let groups = standard_groups();
        let state = GameState::new();

        set_candidates(&state, 0, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 5, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 0, 5, &[Value::ONE, Value::TWO]);

        let strat = XYWing { enabled: true };
        let res = strat
            .apply(&state, &groups, &BoardStatsCache::new(&state))
            .unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }
}
