use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::strategies::{Strategy, StrategyResult};
use crate::{Coordinate, Value};
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes the H-pattern strategy.
///
/// The H-pattern targets a single digit at a time and inspects a "chute"
/// (a horizontal band or vertical stack of three standard 3x3 boxes).
/// Within the chute it cross-hatches the digit's existing placements
/// and remaining candidates to find:
///
/// - **Pointing**: when a box's candidates for the digit are confined to a
///   single row (or column) of the band, the digit cannot appear in that
///   row (or column) outside the box.
/// - **Claiming** (box/line reduction): when a row (or column) of the band
///   only has candidates for the digit in a single box, the digit cannot
///   appear in the other rows (or columns) of that box.
///
/// The remaining open cells often visually form an "H" or inverted-H
/// shape, hence the name.
///
/// ## Applicability
/// Only triggers when the puzzle has standard 3x3 box constraints
/// ([`CellGroupType::StandardBlock`]). For non-standard layouts such as
/// Nonomino, the strategy is a no-op.
pub struct HPattern {
    enabled: bool,
}

impl HPattern {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for HPattern {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "H-Pattern")
    }
}

impl Strategy for HPattern {
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
        if !has_standard_blocks(groups) {
            return Ok(StrategyResult::NoChange);
        }

        let mut applied_some = false;

        for value in Value::range() {
            for band in 0..3u8 {
                applied_some |= apply_horizontal(state, value, band);
            }
            for stack in 0..3u8 {
                applied_some |= apply_vertical(state, value, stack);
            }
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            trace!("No H-Pattern deductions could be applied");
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

fn has_standard_blocks(groups: &CellGroups) -> bool {
    groups
        .iter()
        .any(|g| g.group_type == CellGroupType::StandardBlock)
}

/// Applies H-pattern deductions to one horizontal band of three 3x3 boxes.
///
/// `band` is the band index 0..3, covering rows `band*3..band*3+3`.
fn apply_horizontal(state: &GameState, value: Value, band: u8) -> bool {
    let y_base = band * 3;
    let mut applied = false;

    // For each of the three boxes in the band, which of the three rows
    // still have the value as an unsolved candidate.
    let mut box_rows = [0u8; 3];
    // For each of the three rows in the band, which of the three boxes
    // still have the value as an unsolved candidate.
    let mut row_boxes = [0u8; 3];
    // Whether the value is already solved (placed) inside the given box.
    let mut solved_in_box = [false; 3];

    for bx in 0..3u8 {
        for dy in 0..3u8 {
            for dx in 0..3u8 {
                let x = bx * 3 + dx;
                let y = y_base + dy;
                let cell = state.get_at_xy(x, y);
                if !cell.contains(value) {
                    continue;
                }
                // Track both solved and unsolved presence so that a value
                // newly solved (and not yet propagated) in row Y still
                // contributes to that row's box set. Without this, a stale
                // candidate in another box of the same band could falsely
                // appear as the only home for the value in the row.
                box_rows[bx as usize] |= 1 << dy;
                row_boxes[dy as usize] |= 1 << bx;
                if cell.is_solved() {
                    solved_in_box[bx as usize] = true;
                }
            }
        }
    }

    // Collect eliminations from the snapshot before mutating, so later
    // deductions in the same band don't see stale (just-cleared) cells
    // as candidate positions.
    let mut to_forget: Vec<Coordinate> = Vec::new();

    // Pointing: box -> row.
    for bx in 0..3u8 {
        if solved_in_box[bx as usize] {
            continue;
        }
        let rows = box_rows[bx as usize];
        if rows.count_ones() != 1 {
            continue;
        }
        let dy = rows.trailing_zeros() as u8;
        let y = y_base + dy;
        for x in 0..9u8 {
            if x / 3 == bx {
                continue;
            }
            to_forget.push(Coordinate::new(x, y));
        }
    }

    // Claiming: row -> box.
    for dy in 0..3u8 {
        let boxes = row_boxes[dy as usize];
        if boxes.count_ones() != 1 {
            continue;
        }
        let bx = boxes.trailing_zeros() as u8;
        if solved_in_box[bx as usize] {
            continue;
        }
        for ddy in 0..3u8 {
            if ddy == dy {
                continue;
            }
            let y = y_base + ddy;
            for dx in 0..3u8 {
                let x = bx * 3 + dx;
                to_forget.push(Coordinate::new(x, y));
            }
        }
    }

    for coord in to_forget {
        let index = coord.into_index();
        // Skip already-solved (or impossible) cells. The strategy queues
        // eliminations from a chute-level snapshot, and within a single
        // apply() call NakedSingles has not yet re-propagated cells that
        // earlier H-pattern deductions reduced to a single candidate;
        // stripping the placed digit from such a cell would turn it
        // empty. Any genuine inconsistency that this guard masks is
        // re-detected by the next consistency check on the branch.
        if state.get_at_index(index).len() <= 1 {
            continue;
        }
        if state.forget_at_index(index, value) {
            applied = true;
            trace!(
                "H-Pattern eliminated {value:?} at {coord:?}",
                value = value,
                coord = coord
            );
        }
    }

    if applied {
        debug!(
            "Applied H-Pattern in horizontal band {band} for value {value:?}",
            band = band,
            value = value
        );
    }
    applied
}

/// Applies H-pattern deductions to one vertical stack of three 3x3 boxes.
///
/// `stack` is the stack index 0..3, covering columns `stack*3..stack*3+3`.
fn apply_vertical(state: &GameState, value: Value, stack: u8) -> bool {
    let x_base = stack * 3;
    let mut applied = false;

    let mut box_cols = [0u8; 3];
    let mut col_boxes = [0u8; 3];
    let mut solved_in_box = [false; 3];

    for by in 0..3u8 {
        for dx in 0..3u8 {
            for dy in 0..3u8 {
                let x = x_base + dx;
                let y = by * 3 + dy;
                let cell = state.get_at_xy(x, y);
                if !cell.contains(value) {
                    continue;
                }
                // See horizontal case: solved cells must also contribute
                // so an un-propagated placement doesn't make a stale
                // candidate look like the sole option.
                box_cols[by as usize] |= 1 << dx;
                col_boxes[dx as usize] |= 1 << by;
                if cell.is_solved() {
                    solved_in_box[by as usize] = true;
                }
            }
        }
    }

    let mut to_forget: Vec<Coordinate> = Vec::new();

    // Pointing: box -> column.
    for by in 0..3u8 {
        if solved_in_box[by as usize] {
            continue;
        }
        let cols = box_cols[by as usize];
        if cols.count_ones() != 1 {
            continue;
        }
        let dx = cols.trailing_zeros() as u8;
        let x = x_base + dx;
        for y in 0..9u8 {
            if y / 3 == by {
                continue;
            }
            to_forget.push(Coordinate::new(x, y));
        }
    }

    // Claiming: column -> box.
    for dx in 0..3u8 {
        let boxes = col_boxes[dx as usize];
        if boxes.count_ones() != 1 {
            continue;
        }
        let by = boxes.trailing_zeros() as u8;
        if solved_in_box[by as usize] {
            continue;
        }
        for ddx in 0..3u8 {
            if ddx == dx {
                continue;
            }
            let x = x_base + ddx;
            for dy in 0..3u8 {
                let y = by * 3 + dy;
                to_forget.push(Coordinate::new(x, y));
            }
        }
    }

    for coord in to_forget {
        let index = coord.into_index();
        // Skip already-solved (or impossible) cells. The strategy queues
        // eliminations from a chute-level snapshot, and within a single
        // apply() call NakedSingles has not yet re-propagated cells that
        // earlier H-pattern deductions reduced to a single candidate;
        // stripping the placed digit from such a cell would turn it
        // empty. Any genuine inconsistency that this guard masks is
        // re-detected by the next consistency check on the branch.
        if state.get_at_index(index).len() <= 1 {
            continue;
        }
        if state.forget_at_index(index, value) {
            applied = true;
            trace!(
                "H-Pattern eliminated {value:?} at {coord:?}",
                value = value,
                coord = coord
            );
        }
    }

    if applied {
        debug!(
            "Applied H-Pattern in vertical stack {stack} for value {value:?}",
            stack = stack,
            value = value
        );
    }
    applied
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell_group::CellGroups;
    use crate::default_solver::{DefaultSolver, DefaultSolverConfig};
    use crate::GameState;

    fn standard_groups() -> CellGroups {
        CellGroups::default()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns()
    }

    #[test]
    fn does_not_eliminate_solution_candidate_on_hardest() {
        let game = crate::example_games::sudoku2::example_sudoku_hardest();
        let config_no_h = DefaultSolverConfig {
            hidden_singles: true,
            naked_twins: true,
            hidden_twins: true,
            h_pattern: false,
            xwings: true,
        };
        let solver = DefaultSolver::new_with(&game.groups, &config_no_h);
        let solution = solver
            .solve(&game.initial_state)
            .expect("hardest should have a solution");

        let state = game.initial_state.clone();
        for index in crate::Index::range() {
            let cell = state.get_at_index(index);
            if cell.is_solved() {
                let v = cell.iter_candidates().next().unwrap();
                state.place_and_propagate_at_index(index, v, &game.groups);
            }
        }
        let strat = HPattern { enabled: true };
        let _ = strat.apply(&state, &game.groups).unwrap();

        for index in crate::Index::range() {
            let solved = solution
                .get_at_index(index)
                .iter_candidates()
                .next()
                .unwrap();
            let after = state.get_at_index(index);
            assert!(
                after.contains(solved),
                "H-Pattern wrongly eliminated {solved:?} at {coord:?}",
                solved = solved,
                coord = index.into_coordinate()
            );
        }
    }

    #[test]
    fn pointing_box_to_row_in_band() {
        // Hand-craft a state so that exactly box 0 confines value 1 to
        // its top row. Other boxes in the band still have 1-candidates
        // spread across multiple rows, and band 1/2 are untouched so
        // vertical analysis can't fire on value 1 either.
        let groups = standard_groups();
        let state = GameState::new();

        // Cells in band 0 we want to strip value 1 from.
        let strip: &[(u8, u8)] = &[
            // Box 0 rows 1, 2 (so box 0 is confined to row 0).
            (0, 1),
            (1, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
            // Box 1: keep 1 only at (3,0), (4,1), (5,2) (spread, not confined).
            (4, 0),
            (5, 0),
            (3, 1),
            (5, 1),
            (3, 2),
            (4, 2),
            // Box 2: keep 1 only at (6,0), (7,1), (8,2) (spread, not confined).
            (7, 0),
            (8, 0),
            (6, 1),
            (8, 1),
            (6, 2),
            (7, 2),
        ];
        for &(x, y) in strip {
            state.forget_at_index(Coordinate::new(x, y).into_index(), Value::ONE);
        }

        let strat = HPattern { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // Pointing fires for box 0 -> row 0, clearing 1 from (3,0) and
        // (6,0) (the remaining row-0 candidates in boxes 1 and 2).
        assert!(!state.get_at_xy(3, 0).contains(Value::ONE));
        assert!(!state.get_at_xy(6, 0).contains(Value::ONE));
        // Box 0 row 0 still has 1 as a candidate.
        for col in 0u8..=2 {
            assert!(
                state.get_at_xy(col, 0).contains(Value::ONE),
                "1 must remain at ({col},0)",
                col = col
            );
        }
    }

    #[test]
    fn does_not_empty_solved_cell_in_target_row() {
        // Simulate a stale snapshot: box 0 still has 1 as a candidate
        // confined to row 0, but row 0 also has 1 already placed at a
        // solved cell in box 2. Pointing would normally target that
        // solved cell; the guard must prevent emptying it.
        let groups = standard_groups();
        let state = GameState::new();

        // Strip value 1 such that:
        // - Box 0 row 0 keeps 1-candidates (columns 0..3).
        // - Box 0 rows 1, 2 lose 1 (so box 0 is confined to row 0).
        // - Box 1 spreads 1 across rows 0, 1, 2 (not confined).
        // - Box 2: cell (8,0) is the *only* cell with 1, and we then
        //   collapse it to exactly { 1 } so it counts as solved.
        let strip: &[(u8, u8)] = &[
            (0, 1),
            (1, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
            (4, 0),
            (5, 0),
            (3, 1),
            (5, 1),
            (3, 2),
            (4, 2),
            (6, 0),
            (7, 0),
            (6, 1),
            (7, 1),
            (8, 1),
            (6, 2),
            (7, 2),
            (8, 2),
        ];
        for &(x, y) in strip {
            state.forget_at_index(Coordinate::new(x, y).into_index(), Value::ONE);
        }
        // Collapse (8,0) so it is solved with value 1.
        for v in [
            Value::TWO,
            Value::THREE,
            Value::FOUR,
            Value::FIVE,
            Value::SIX,
            Value::SEVEN,
            Value::EIGHT,
            Value::NINE,
        ] {
            state.forget_at_index(Coordinate::new(8, 0).into_index(), v);
        }
        assert!(state.get_at_xy(8, 0).is_solved());

        let strat = HPattern { enabled: true };
        // Should run without panicking and without emptying (8,0).
        let _ = strat.apply(&state, &groups).unwrap();
        assert!(state.get_at_xy(8, 0).contains(Value::ONE));
        assert!(state.get_at_xy(8, 0).is_solved());
    }

    #[test]
    fn skips_when_no_standard_blocks() {
        // Without StandardBlock groups (e.g. nonomino), H-pattern must
        // be a no-op regardless of board contents.
        let groups = CellGroups::default().with_default_rows_and_columns();
        let state = GameState::new();
        let strat = HPattern { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }
}
