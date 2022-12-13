use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::{Index, IndexBitSet};
use crate::strategies::{Strategy, StrategyResult};
use crate::value::ValueBitSet;
use crate::{index, Coordinate, IndexedGameCell, Value};
use log::debug;
use std::fmt::{Debug, Formatter};

/// Identifies and realizes the X-Wing strategy.
#[derive(Default)]
pub struct XWing {}

impl XWing {
    pub fn new_box() -> Box<Self> {
        Box::new(Self::default())
    }
}

impl Debug for XWing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "X-Wing")
    }
}

impl Strategy for XWing {
    fn always_continue(&self) -> bool {
        false
    }

    fn apply(
        &self,
        state: &GameState,
        groups: &CellGroups,
    ) -> Result<StrategyResult, InvalidGameState> {
        let mut xwings = Vec::default();

        for value in Value::range() {
            // Identify all the cells that are not solved and contain the value under test.
            let cells: Vec<_> = Index::range()
                .map(|index| state.get_at_index(index).into_indexed(index))
                .filter(|cell| !cell.is_solved() && cell.contains(value))
                .collect();

            // For the X-Wing to work, we need at least four matching cells
            // in order to form a single rectangle.
            if cells.len() < 4 {
                return Ok(StrategyResult::NoChange);
            }

            // Identify pairs in the same row or column.
            for top_left_coord in cells.iter().map(IndexedGameCell::as_coordinate) {
                // Ignore all cells that lie on the right or bottom edge as they
                // cannot form a rectangle to the bottom right.
                if top_left_coord.x == 8 || top_left_coord.y == 8 {
                    continue;
                }

                // Find all matching cells to the right.
                let top_right_coords: Vec<_> = cells
                    .iter()
                    .map(IndexedGameCell::as_coordinate)
                    .filter(|coord| coord.x > top_left_coord.x && coord.y == top_left_coord.y)
                    .collect();
                if top_right_coords.is_empty() {
                    continue;
                }

                // Find all matching cells to the bottom.
                let bottom_left_coords: Vec<_> = cells
                    .iter()
                    .map(IndexedGameCell::as_coordinate)
                    .filter(|coord| coord.x == top_left_coord.x && coord.y > top_left_coord.y)
                    .collect();
                if bottom_left_coords.is_empty() {
                    continue;
                }

                // Scan down from every peer on the right.
                for &top_right_coord in top_right_coords.iter() {
                    for bottom_right_coord in cells
                        .iter()
                        .map(IndexedGameCell::as_coordinate)
                        .filter(|coord| coord.x == top_right_coord.x && coord.y > top_right_coord.y)
                    {
                        // Test if there is a match on any y coordinate of the peers
                        // on the bottom of the test cell.
                        for &bottom_left_coord in bottom_left_coords
                            .iter()
                            .filter(|coord| coord.y == bottom_right_coord.y)
                        {
                            debug!(
                                "Identified X-Wing for value {value:?} at {tl:?}, {tr:?}, {bl:?}, {br:?}",
                                value = value,
                                tl = top_left_coord,
                                tr = top_right_coord,
                                bl = bottom_left_coord,
                                br = bottom_right_coord
                            );
                            xwings.push(XWingCoords {
                                value,
                                top_left: top_left_coord.into(),
                                top_right: top_right_coord.into(),
                                bottom_left: bottom_left_coord.into(),
                                bottom_right: bottom_right_coord.into(),
                            })
                        }
                    }
                }
            }
        }

        if xwings.is_empty() {
            return Ok(StrategyResult::NoChange);
        }

        let mut applied_some = false;
        for xwing in xwings {
            debug_assert!(xwing.top_left != xwing.top_right);
            debug_assert!(xwing.top_left != xwing.bottom_left);
            debug_assert!(xwing.top_right != xwing.bottom_right);
            debug_assert!(xwing.top_right != xwing.bottom_left);

            // Forget top row.
            for index in groups
                .get_peer_indexes(xwing.top_left, CellGroupType::StandardRow)
                .filter(|&idx| idx != xwing.top_left && idx != xwing.top_right)
            {
                applied_some |= state.forget_at_index(index, xwing.value);
            }

            // Forget in bottom row.
            for index in groups
                .get_peer_indexes(xwing.bottom_left, CellGroupType::StandardRow)
                .filter(|&idx| idx != xwing.bottom_left && idx != xwing.bottom_right)
            {
                applied_some |= state.forget_at_index(index, xwing.value);
            }

            // Forget left column.
            for index in groups
                .get_peer_indexes(xwing.top_left, CellGroupType::StandardColumn)
                .filter(|&idx| idx != xwing.top_left && idx != xwing.bottom_left)
            {
                applied_some |= state.forget_at_index(index, xwing.value);
            }

            // Forget right column.
            for index in groups
                .get_peer_indexes(xwing.top_left, CellGroupType::StandardColumn)
                .filter(|&idx| idx != xwing.top_right && idx != xwing.bottom_right)
            {
                applied_some |= state.forget_at_index(index, xwing.value);
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

struct XWingCoords {
    value: Value,
    top_left: Index,
    top_right: Index,
    bottom_left: Index,
    bottom_right: Index,
}
