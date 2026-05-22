use crate::board_stats::BoardStatsCache;
use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::IndexBitSet;
use crate::strategies::{Strategy, StrategyResult};
use crate::value::Value;
use crate::ValueBitSet;
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes Hidden Twins.
///
/// ## Example
/// A single is a value that does not appear in any other cell.
/// It is hidden when it appears along other values.
///
/// Given two cells with the values `5 7`, `3 4 5` and `3 4 7`,
/// `3 4` is the hidden twin. Since `3 4` only appear in the
/// second and third cell they must be placed there, eliminating
/// `5` and `7` from those cells.
pub struct HiddenTwins {
    enabled: bool,
}

impl HiddenTwins {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for HiddenTwins {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hidden twins")
    }
}

impl Strategy for HiddenTwins {
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
        let mut twins_to_apply: Vec<HiddenTwin> = Vec::default();

        for group in groups.iter().filter(|g| g.group_type == group_type) {
            // Read each cell of the group once and fan its candidates into
            // per-value position sets. A value belongs to a hidden twin only
            // if it appears in exactly two unsolved cells; everything else is
            // skipped before the pair search.
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

            // Collect the values whose candidate footprint is exactly two
            // cells. The hidden-twin pair search compares only these.
            let mut bivalue_positions: Vec<(Value, IndexBitSet)> = Vec::with_capacity(9);
            for (v_idx, value) in Value::range().enumerate() {
                if counts[v_idx] == 2 {
                    bivalue_positions.push((value, positions[v_idx]));
                }
            }

            if bivalue_positions.len() < 2 {
                continue;
            }

            // Two values that share the same 2-cell footprint form a hidden
            // twin: the cells must hold those two digits in some order.
            let n = bivalue_positions.len();
            for i in 0..n {
                for j in (i + 1)..n {
                    if bivalue_positions[i].1 != bivalue_positions[j].1 {
                        continue;
                    }

                    let values = ValueBitSet::empty()
                        .with_value(bivalue_positions[i].0)
                        .with_value(bivalue_positions[j].0);

                    trace!(
                        "Identified Hidden Twin in {group_type:?}: values {values:?} confined to 2 cells",
                        group_type = group_type,
                        values = values
                    );

                    twins_to_apply.push(HiddenTwin {
                        indexes: bivalue_positions[i].1,
                        values,
                    });
                }
            }
        }

        if twins_to_apply.is_empty() {
            trace!(
                "No Hidden Twins could be applied in {group_type:?}",
                group_type = group_type
            );
            return Ok(StrategyResult::NoChange);
        }

        // Apply eliminations after detection so the detection pass reads a
        // consistent candidate snapshot. Restrict each cell to the
        // intersection of its current candidates with the twin values:
        // simply assigning `twin.values` could re-introduce candidates
        // earlier strategies had eliminated, since a cell may only carry a
        // proper subset of the twin.
        //
        // Overlapping twins detected in the same pass can shrink a cell
        // mid-loop. If a later twin's intersection turns out to be empty
        // the position-set snapshot is no longer consistent with the
        // current board, so we surface that as an invalid state and let
        // the solver back out of the branch.
        let mut applied_some = false;
        for twin in twins_to_apply {
            let mut applied_here = false;
            for index in twin.indexes.iter() {
                let current = state.get_at_index(index).to_bitset();
                let kept = current.with_intersection(twin.values);
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
                    "Applied Hidden Twin at {indexes:?}: {values:?}",
                    indexes = twin.indexes,
                    values = twin.values
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

struct HiddenTwin {
    indexes: IndexBitSet,
    values: ValueBitSet,
}
