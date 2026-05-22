use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::Index;
use crate::strategies::{Strategy, StrategyResult};
use crate::value::ValueBitSet;
use crate::Coordinate;
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes the Unique Rectangle (Type 1) strategy.
///
/// A Unique Rectangle is built from four cells at the corners of a rectangle
/// spanning exactly two rows, two columns, and two block groups. In a Type 1
/// pattern three of those corners are bivalue and share the same candidate
/// pair `{x, y}`; the fourth corner ("roof") contains `{x, y}` plus at least
/// one extra candidate.
///
/// If the roof cell ended up holding `x` or `y`, the rectangle could be
/// completed in two mutually exclusive ways - the puzzle would then have two
/// solutions, contradicting the uniqueness assumption. The extra candidate(s)
/// at the roof are therefore forced, and `x` and `y` are eliminated from it.
///
/// # Uniqueness assumption
///
/// This strategy is unsound on puzzles that do not have exactly one solution.
/// User-supplied or fuzz-generated boards may violate uniqueness; running this
/// strategy on such boards can silently corrupt the solver. The strategy is
/// therefore opt-in via [`crate::default_solver::DefaultSolverConfig::unique_rectangle`]
/// and defaults to disabled.
pub struct UniqueRectangle {
    enabled: bool,
}

impl UniqueRectangle {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for UniqueRectangle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unique Rectangle (Type 1)")
    }
}

impl Strategy for UniqueRectangle {
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
        let mut applied_some = false;

        for r1 in 0..9u8 {
            for r2 in (r1 + 1)..9u8 {
                for c1 in 0..9u8 {
                    for c2 in (c1 + 1)..9u8 {
                        let corners = [
                            Coordinate::new(c1, r1).into_index(),
                            Coordinate::new(c2, r1).into_index(),
                            Coordinate::new(c1, r2).into_index(),
                            Coordinate::new(c2, r2).into_index(),
                        ];

                        let cells = [
                            state.get_at_index(corners[0]),
                            state.get_at_index(corners[1]),
                            state.get_at_index(corners[2]),
                            state.get_at_index(corners[3]),
                        ];

                        // A solved corner cannot be part of a UR; skip early.
                        if cells.iter().any(|c| c.is_solved()) {
                            continue;
                        }

                        // The rectangle must span exactly two block groups -
                        // the "third constraint" that makes the deadly
                        // pattern possible.
                        if !corners_span_two_blocks(groups, &corners) {
                            continue;
                        }

                        // Try each corner as the roof - the unique non-bivalue
                        // cell holding the candidate pair plus extras.
                        for roof_pos in 0..4 {
                            let roof_cell = cells[roof_pos];
                            if roof_cell.len() <= 2 {
                                continue;
                            }

                            let mut shared = ValueBitSet::empty();
                            let mut all_bivalue_match = true;
                            for (i, c) in cells.iter().enumerate() {
                                if i == roof_pos {
                                    continue;
                                }
                                if c.len() != 2 {
                                    all_bivalue_match = false;
                                    break;
                                }
                                let bits = c.to_bitset();
                                if shared.is_empty() {
                                    shared = bits;
                                } else if shared != bits {
                                    all_bivalue_match = false;
                                    break;
                                }
                            }

                            if !all_bivalue_match || shared.len() != 2 {
                                continue;
                            }
                            if !roof_cell.contains_all(shared) {
                                continue;
                            }

                            let roof_index = corners[roof_pos];
                            let mut iter = shared.iter();
                            let x = iter.next().unwrap();
                            let y = iter.next().unwrap();

                            trace!(
                                "Identified UR Type 1 with roof at {roof:?} sharing {{{x:?}, {y:?}}}",
                                roof = roof_index,
                                x = x,
                                y = y,
                            );

                            let mut applied = false;
                            applied |= state.forget_at_index(roof_index, x);
                            applied |= state.forget_at_index(roof_index, y);

                            if applied {
                                debug!(
                                    "Applied UR Type 1 - removed {{{x:?}, {y:?}}} from {roof:?}",
                                    x = x,
                                    y = y,
                                    roof = roof_index,
                                );
                                applied_some = true;
                            }

                            // A rectangle has at most one roof; stop probing
                            // further corners for this rectangle.
                            break;
                        }
                    }
                }
            }
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
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

/// Returns `true` iff the four rectangle corners belong to exactly two
/// distinct standard block groups. Boards without standard blocks always
/// return `false`, suppressing the strategy in non-standard variants.
fn corners_span_two_blocks(groups: &CellGroups, corners: &[Index; 4]) -> bool {
    let mut ids: [usize; 4] = [0; 4];
    let mut count: usize = 0;
    for &idx in corners {
        let block_id = match groups.get_groups_at_index(idx) {
            Ok(gs) => gs
                .into_iter()
                .find(|g| g.group_type == CellGroupType::StandardBlock)
                .and_then(|g| g.id),
            Err(_) => return false,
        };
        let id = match block_id {
            Some(id) => id,
            None => return false,
        };
        if !ids[..count].contains(&id) {
            if count == 2 {
                return false;
            }
            ids[count] = id;
            count += 1;
        }
    }
    count == 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell_group::CellGroups;
    use crate::value::Value;
    use crate::GameState;

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

    /// Type 1 UR with three bivalue corners {1,2} at (0,0), (1,0), (0,1)
    /// and a roof at (1,1) holding {1,2,3}. The rectangle spans rows 0-1,
    /// columns 0-1, and a single block - which is one block, not two.
    /// Pick corners across a block boundary instead.
    #[test]
    fn ur_type_1_eliminates_pair_from_roof() {
        let groups = standard_groups();
        let state = GameState::new();

        // Use rows 0 and 3 (different block rows) and columns 0 and 1
        // (same block column). The four corners then span exactly two
        // blocks - top-left and middle-left.
        set_candidates(&state, 0, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 1, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 0, 3, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 1, 3, &[Value::ONE, Value::TWO, Value::THREE]);

        let strat = UniqueRectangle { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // Roof loses the pair, retains only the extra candidate.
        let roof = state.get_at_xy(1, 3);
        assert!(!roof.contains(Value::ONE));
        assert!(!roof.contains(Value::TWO));
        assert!(roof.contains(Value::THREE));

        // Bivalue corners untouched.
        assert!(state.get_at_xy(0, 0).contains(Value::ONE));
        assert!(state.get_at_xy(0, 0).contains(Value::TWO));
        assert!(state.get_at_xy(1, 0).contains(Value::ONE));
        assert!(state.get_at_xy(1, 0).contains(Value::TWO));
        assert!(state.get_at_xy(0, 3).contains(Value::ONE));
        assert!(state.get_at_xy(0, 3).contains(Value::TWO));
    }

    /// Same candidate pattern but the rectangle spans only one block:
    /// rows 0,1 and columns 0,1 are all inside the top-left block. UR
    /// must abstain because the third "block" constraint is not satisfied.
    #[test]
    fn ur_does_not_fire_within_single_block() {
        let groups = standard_groups();
        let state = GameState::new();

        set_candidates(&state, 0, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 1, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 0, 1, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 1, 1, &[Value::ONE, Value::TWO, Value::THREE]);

        let strat = UniqueRectangle { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }

    /// Rectangle spanning four distinct blocks - rows in different bands
    /// and columns in different bands - must not trigger UR.
    #[test]
    fn ur_does_not_fire_across_four_blocks() {
        let groups = standard_groups();
        let state = GameState::new();

        set_candidates(&state, 0, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 3, 0, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 0, 3, &[Value::ONE, Value::TWO]);
        set_candidates(&state, 3, 3, &[Value::ONE, Value::TWO, Value::THREE]);

        let strat = UniqueRectangle { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }

    #[test]
    fn no_change_on_pristine_board() {
        let groups = standard_groups();
        let state = GameState::new();
        let strat = UniqueRectangle { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }

    #[test]
    fn disabled_strategy_reports_disabled() {
        let strat = UniqueRectangle { enabled: false };
        assert!(!strat.is_enabled());
    }
}
