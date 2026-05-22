use crate::cell_group::{CellGroup, CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::IndexBitSet;
use crate::strategies::{Strategy, StrategyResult};
use crate::{Coordinate, Value};
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes the H-pattern strategy (Locked Candidates).
///
/// For every block group (a `StandardBlock` or a `Custom` region such as a
/// Nonomino tile or a Hypersudoku window) the strategy cross-hatches each
/// digit's candidate positions against every standard row and column:
///
/// - **Pointing**: if a block's candidates for the digit are confined to a
///   single row (or column), the digit cannot appear in that row (or
///   column) outside the block.
/// - **Claiming** (box/line reduction): if a row (or column) has the
///   digit's candidates confined to a single block, the digit cannot
///   appear in the rest of that block.
///
/// The remaining open cells in the classical 3x3 case often form an "H"
/// or inverted-H shape, hence the name. The same logic applies to
/// arbitrary block shapes.
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
        let mut applied_some = false;

        // Fast path: classical 3x3 boxes aligned with the standard band
        // and stack layout. Identified once via the presence of any
        // `StandardBlock` group.
        let has_standard_blocks = groups
            .iter()
            .any(|g| g.group_type == CellGroupType::StandardBlock);
        if has_standard_blocks {
            for value in Value::range() {
                for band in 0..3u8 {
                    applied_some |= apply_horizontal(state, value, band);
                }
                for stack in 0..3u8 {
                    applied_some |= apply_vertical(state, value, stack);
                }
            }
        }

        // Generic path: every block-shaped group that isn't covered by
        // the fast path (Nonomino tiles, Hypersudoku windows, etc.) is
        // intersected with every standard row and column.
        applied_some |= apply_custom_blocks(state, groups);

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

/// Applies H-pattern deductions to one horizontal band of three 3x3 boxes.
///
/// `band` is the band index 0..3, covering rows `band*3..band*3+3`.
fn apply_horizontal(state: &GameState, value: Value, band: u8) -> bool {
    let y_base = band * 3;
    let mut applied = false;

    let mut box_rows = [0u8; 3];
    let mut row_boxes = [0u8; 3];
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
                box_rows[bx as usize] |= 1 << dy;
                row_boxes[dy as usize] |= 1 << bx;
                if cell.is_solved() {
                    solved_in_box[bx as usize] = true;
                }
            }
        }
    }

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
        // See `eliminate` for the rationale behind this guard.
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

/// Generic path for arbitrary block-shaped groups (Nonomino tiles,
/// Hypersudoku windows, etc.). Standard 3x3 blocks are handled by the
/// faster band/stack code above.
fn apply_custom_blocks(state: &GameState, groups: &CellGroups) -> bool {
    // Bail out before any heap allocation when the puzzle has no
    // custom block groups (classic Sudoku, the hot bench path).
    if !groups.iter().any(|g| g.group_type == CellGroupType::Custom) {
        return false;
    }

    let mut blocks: Vec<&CellGroup> = Vec::with_capacity(9);
    let mut lines: Vec<&CellGroup> = Vec::with_capacity(18);
    for g in groups.iter() {
        match g.group_type {
            CellGroupType::Custom => blocks.push(g),
            CellGroupType::StandardRow | CellGroupType::StandardColumn => lines.push(g),
            CellGroupType::StandardBlock => {}
        }
    }
    if blocks.is_empty() || lines.is_empty() {
        return false;
    }

    let block_data: Vec<GroupCandidates> = blocks
        .iter()
        .map(|g| GroupCandidates::collect(state, g))
        .collect();
    let line_data: Vec<GroupCandidates> = lines
        .iter()
        .map(|g| GroupCandidates::collect(state, g))
        .collect();

    let mut applied = false;
    for value in Value::range() {
        let vi = (value.get() - 1) as usize;
        let value_bit = 1u16 << vi;

        for (block, b_data) in blocks.iter().zip(&block_data) {
            let b_cands = b_data.per_value[vi];
            if b_cands.is_empty() {
                continue;
            }
            let block_indexes = block.indexes();
            let block_has_solved = b_data.solved_mask & value_bit != 0;

            for (line, l_data) in lines.iter().zip(&line_data) {
                let l_cands = l_data.per_value[vi];
                if l_cands.is_empty() {
                    continue;
                }
                let line_indexes = line.indexes();
                if !block_indexes.overlaps_with(line_indexes) {
                    continue;
                }

                if !block_has_solved && b_cands.is_subset_of(line_indexes) {
                    let targets = line_indexes.subtract(block_indexes);
                    applied |= eliminate(state, value, &targets);
                }

                let line_has_solved = l_data.solved_mask & value_bit != 0;
                if !line_has_solved && l_cands.is_subset_of(block_indexes) {
                    let targets = block_indexes.subtract(line_indexes);
                    applied |= eliminate(state, value, &targets);
                }
            }
        }
    }
    applied
}

struct GroupCandidates {
    /// For each digit `v` (0-indexed), the set of cells in this group
    /// that hold `v+1` either as a candidate or as their solved value.
    per_value: [IndexBitSet; 9],
    /// Bit `v` set iff the group contains a cell already solved to `v+1`.
    solved_mask: u16,
}

impl GroupCandidates {
    fn collect(state: &GameState, group: &CellGroup) -> Self {
        let mut per_value = [IndexBitSet::empty(); 9];
        let mut solved_mask = 0u16;
        for index in group.iter_indexes() {
            let cell = state.get_at_index(index);
            let solved = cell.is_solved();
            for v in cell.iter_candidates() {
                let vi = (v.get() - 1) as usize;
                per_value[vi].insert(index);
                if solved {
                    solved_mask |= 1 << vi;
                }
            }
        }
        Self {
            per_value,
            solved_mask,
        }
    }
}

fn eliminate(state: &GameState, value: Value, targets: &IndexBitSet) -> bool {
    let mut applied = false;
    for index in targets.iter() {
        // Skip already-solved (or impossible) cells. Within a single
        // apply() call, NakedSingles has not yet re-propagated cells
        // that earlier H-pattern deductions reduced to a single
        // candidate; stripping the placed digit from such a cell
        // would turn it empty. Any genuine inconsistency that this
        // guard masks is re-detected by the next consistency check
        // on the branch.
        if state.get_at_index(index).len() <= 1 {
            continue;
        }
        if state.forget_at_index(index, value) {
            applied = true;
            trace!(
                "H-Pattern eliminated {value:?} at {coord:?}",
                value = value,
                coord = index.into_coordinate()
            );
        }
    }
    if applied {
        debug!("Applied H-Pattern for value {value:?}", value = value);
    }
    applied
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell_group::CellGroups;
    use crate::default_solver::{DefaultSolver, DefaultSolverConfig};
    use crate::{Coordinate, GameState};

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
            naked_triples: true,
            hidden_triples: true,
            h_pattern: false,
            skyscraper: true,
            xwings: true,
            xy_wing: true,
            w_wing: true,
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
    fn skips_when_no_block_groups() {
        // Without StandardBlock or Custom block groups, H-pattern must
        // be a no-op regardless of board contents.
        let groups = CellGroups::default().with_default_rows_and_columns();
        let state = GameState::new();
        let strat = HPattern { enabled: true };
        let res = strat.apply(&state, &groups).unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }

    #[test]
    fn fires_on_nonomino_custom_block() {
        // Nonomino block 0 is { 0, 1, 2, 9, 10, 11, 18, 27, 28 }.
        // Cells (0..3, 0) sit on row 0; cells 9..11 sit on row 1;
        // cells 18, 27, 28 sit on rows 2, 3, 3 respectively.
        //
        // We strip value 1 from every cell of the block except those on
        // row 0, forcing the block's 1-candidates to be confined to row
        // 0 (intersection = the three top cells). Pointing should then
        // remove 1 from the rest of row 0.
        let game = crate::example_games::nonomino::example_nonomino();
        let state = GameState::new();

        let block_cells: [u8; 9] = [0, 1, 2, 9, 10, 11, 18, 27, 28];
        let row0_cells: [u8; 3] = [0, 1, 2];

        for &raw in &block_cells {
            if row0_cells.contains(&raw) {
                continue;
            }
            state.forget_at_index(crate::Index::new(raw), Value::ONE);
        }

        let strat = HPattern { enabled: true };
        let res = strat.apply(&state, &game.groups).unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // Row 0 cells outside the block must lose 1 as a candidate.
        for raw in 3u8..9 {
            let index = crate::Index::new(raw);
            assert!(
                !state.get_at_index(index).contains(Value::ONE),
                "1 should have been eliminated from row 0 at index {raw}",
                raw = raw
            );
        }
        // The three intersecting cells must still hold 1 as a candidate.
        for raw in &row0_cells {
            let index = crate::Index::new(*raw);
            assert!(state.get_at_index(index).contains(Value::ONE));
        }
    }

    #[test]
    fn claiming_fires_on_nonomino_custom_block() {
        // Nonomino block 0 = { 0, 1, 2, 9, 10, 11, 18, 27, 28 } overlaps
        // row 0 in { 0, 1, 2 }. Strip value 5 from row 0 cells 3..=8 so
        // row 0's 5-candidates lie entirely inside block 0. Claiming
        // should then eliminate 5 from block 0's cells outside row 0:
        // { 9, 10, 11, 18, 27, 28 }.
        let game = crate::example_games::nonomino::example_nonomino();
        let state = GameState::new();

        for raw in 3u8..=8 {
            state.forget_at_index(crate::Index::new(raw), Value::FIVE);
        }

        let strat = HPattern { enabled: true };
        let res = strat.apply(&state, &game.groups).unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // Block 0 cells outside row 0 must lose 5 as a candidate.
        for raw in [9u8, 10, 11, 18, 27, 28] {
            let index = crate::Index::new(raw);
            assert!(
                !state.get_at_index(index).contains(Value::FIVE),
                "5 should have been eliminated from block 0 at index {raw}",
                raw = raw
            );
        }
        // The three intersecting row-0 cells must still hold 5.
        for raw in [0u8, 1, 2] {
            let index = crate::Index::new(raw);
            assert!(state.get_at_index(index).contains(Value::FIVE));
        }
    }

    #[test]
    fn fires_on_hypersudoku_window() {
        // Hypersudoku window 0 is { 10, 11, 12, 19, 20, 21, 28, 29, 30 }
        // and overlaps row 1 in { 10, 11, 12 }, row 2 in { 19, 20, 21 },
        // row 3 in { 28, 29, 30 }. Strip 1 everywhere in the window
        // except row 1 cells; window then confines 1 to row 1's slice.
        let game = crate::example_games::hypersudoku::example_hypersudoku();
        let state = GameState::new();

        let window_cells: [u8; 9] = [10, 11, 12, 19, 20, 21, 28, 29, 30];
        let intersecting_row_cells: [u8; 3] = [10, 11, 12];

        for &raw in &window_cells {
            if intersecting_row_cells.contains(&raw) {
                continue;
            }
            state.forget_at_index(crate::Index::new(raw), Value::ONE);
        }

        let strat = HPattern { enabled: true };
        let res = strat.apply(&state, &game.groups).unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // Row 1 outside the window: cells 9, 13, 14, 15, 16, 17.
        for raw in [9u8, 13, 14, 15, 16, 17] {
            let index = crate::Index::new(raw);
            assert!(
                !state.get_at_index(index).contains(Value::ONE),
                "1 should have been eliminated from row 1 at index {raw}",
                raw = raw
            );
        }
        for raw in &intersecting_row_cells {
            let index = crate::Index::new(*raw);
            assert!(state.get_at_index(index).contains(Value::ONE));
        }
    }
}
