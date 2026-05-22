use crate::board_stats::BoardStatsCache;
use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::Index;
use crate::strategies::{Strategy, StrategyResult};
use crate::value::ValueBitSet;
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes naked twins.
///
/// ## Example
/// A naked twin is a pair of cells that share the same values.
///
/// Given three cells with the values `3 5`, `3 4` and `3 4`,
/// `3 4` are the naked twins. Since they must appear in the last two
/// cells, the `3` can be removed from the first cell.
pub struct NakedTwins {
    enabled: bool,
}

impl NakedTwins {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for NakedTwins {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Naked twins")
    }
}

impl Strategy for NakedTwins {
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
        let mut twins_to_remove: Vec<TwinPair> = Vec::default();

        for group in groups.iter().filter(|g| g.group_type == group_type) {
            // Collect every bivalue cell in this group with its candidate set.
            // Naked twins are two bivalue cells sharing the same candidate set
            // - the inverted form of Hidden Twins' two values sharing the
            // same 2-cell footprint.
            let mut bivalues: [(Index, ValueBitSet); 9] = Default::default();
            let mut bivalue_len = 0usize;
            for i in group.iter_indexes() {
                let cell = state.get_at_index(i);
                if cell.len() != 2 {
                    continue;
                }
                bivalues[bivalue_len] = (i, cell.to_bitset());
                bivalue_len += 1;
            }

            if bivalue_len < 2 {
                continue;
            }

            for (i, &(idx_a, bits_a)) in bivalues.iter().take(bivalue_len).enumerate() {
                let mut partner: Option<Index> = None;
                for &(idx_b, bits_b) in bivalues.iter().take(bivalue_len).skip(i + 1) {
                    if bits_b != bits_a {
                        continue;
                    }
                    if partner.is_some() {
                        // Three cells in the same group sharing the same
                        // bivalue pair is inconsistent: only two of them can
                        // hold those two digits.
                        return Err(InvalidGameState {});
                    }
                    partner = Some(idx_b);
                }

                let Some(idx_b) = partner else {
                    continue;
                };

                let (smaller, larger) = if idx_a < idx_b {
                    (idx_a, idx_b)
                } else {
                    (idx_b, idx_a)
                };

                trace!(
                    "Identified Naked Twin pair in {group_type:?} at {a:?} and {b:?}: {values:?}",
                    group_type = group_type,
                    a = smaller,
                    b = larger,
                    values = bits_a
                );
                twins_to_remove.push(TwinPair {
                    smaller,
                    larger,
                    values: bits_a,
                });
            }
        }

        if twins_to_remove.is_empty() {
            return Ok(StrategyResult::NoChange);
        }

        let mut applied_some = false;
        for twin in twins_to_remove {
            // Smaller or larger index doesn't matter - both belong to the
            // same `group_type` peer set.
            let mut applied_twin = false;
            for index in groups
                .get_peer_indexes(twin.smaller, group_type)
                .filter(|&x| x != twin.smaller && x != twin.larger)
            {
                applied_twin |= state.forget_many_at_index(index, twin.values);
            }

            if applied_twin {
                debug!(
                    "Applied Naked Twin at {a:?} and {b:?}: {values:?}",
                    a = twin.smaller,
                    b = twin.larger,
                    values = twin.values
                );
            }

            applied_some |= applied_twin;
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            trace!(
                "No Naked Twins could be applied in {group_type:?}",
                group_type = group_type
            );
            Ok(StrategyResult::NoChange)
        }
    }
}

struct TwinPair {
    smaller: Index,
    larger: Index,
    values: ValueBitSet,
}
