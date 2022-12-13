use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::{Index, IndexBitSet};
use crate::strategies::{Strategy, StrategyResult};
use crate::{Coordinate, Value};
use log::{debug, trace};
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
        let mut xwings: Vec<XWingCoords> = Vec::default();

        for value in Value::range() {
            // Identify all the cells that are not solved and contain the value under test.
            let indexes = IndexBitSet::from_iter(Index::range().filter(|&index| {
                let cell = state.get_at_index(index);
                !cell.is_solved() && cell.contains(value)
            }));

            // For the X-Wing to work, we need at least four matching cells
            // in order to form a single rectangle.
            if indexes.len() < 4 {
                return Ok(StrategyResult::NoChange);
            }

            // For each matching cell, scan for rectangles.
            for tl in indexes {
                let tl: Coordinate = tl.into();

                for x in (tl.x + 1)..9 {
                    for y in (tl.y + 1)..9 {
                        let tr = Coordinate::new(x, tl.y);
                        let bl = Coordinate::new(tl.x, y);
                        let br = Coordinate::new(x, y);

                        let has_tl = indexes.contains(tr.into());
                        let has_bl = indexes.contains(bl.into());
                        let has_br = indexes.contains(br.into());

                        // Ensure we found a rectangle.
                        if !(has_tl && has_bl && has_br) {
                            continue;
                        }

                        // Ensure that only two matches exist in both rows OR both columns.
                        let mut top_count = 0;
                        let mut bottom_count = 0;
                        let mut left_count = 0;
                        let mut right_count = 0;
                        for x in 0..9 {
                            if indexes.contains(Coordinate::new(x, tr.y).into()) {
                                top_count += 1;
                            }
                            if indexes.contains(Coordinate::new(x, br.y).into()) {
                                bottom_count += 1;
                            }
                        }
                        for y in 0..9 {
                            if indexes.contains(Coordinate::new(tl.x, y).into()) {
                                left_count += 1;
                            }
                            if indexes.contains(Coordinate::new(br.x, y).into()) {
                                right_count += 1;
                            }
                        }

                        if !(left_count == 2 && right_count == 2)
                            && !(top_count == 2 && bottom_count == 2)
                        {
                            continue;
                        }

                        trace!(
                            "Identified X-Wing for value {value:?} at {tl:?}, {tr:?}, {bl:?}, {br:?}",
                            value = value,
                            tl = tl,
                            tr = tr,
                            bl = bl,
                            br = br
                        );
                        xwings.push(XWingCoords {
                            value,
                            top_left: tl.into(),
                            top_right: tr.into(),
                            bottom_left: bl.into(),
                            bottom_right: br.into(),
                        })
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

            let mut applied_xwing = false;

            // Forget top row.
            for index in groups
                .get_peer_indexes(xwing.top_left, CellGroupType::StandardRow)
                .filter(|&idx| idx != xwing.top_left && idx != xwing.top_right)
            {
                applied_xwing |= state.forget_at_index(index, xwing.value);
            }

            // Forget in bottom row.
            for index in groups
                .get_peer_indexes(xwing.bottom_left, CellGroupType::StandardRow)
                .filter(|&idx| idx != xwing.bottom_left && idx != xwing.bottom_right)
            {
                applied_xwing |= state.forget_at_index(index, xwing.value);
            }

            // Forget left column.
            for index in groups
                .get_peer_indexes(xwing.top_left, CellGroupType::StandardColumn)
                .filter(|&idx| idx != xwing.top_left && idx != xwing.bottom_left)
            {
                applied_xwing |= state.forget_at_index(index, xwing.value);
            }

            // Forget right column.
            for index in groups
                .get_peer_indexes(xwing.top_right, CellGroupType::StandardColumn)
                .filter(|&idx| idx != xwing.top_right && idx != xwing.bottom_right)
            {
                applied_xwing |= state.forget_at_index(index, xwing.value);
            }

            applied_some |= applied_xwing;

            if applied_xwing {
                debug!(
                    "Applied X-Wing for value {value:?} at {tl:?}, {tr:?}, {bl:?}, {br:?}",
                    value = xwing.value,
                    tl = xwing.top_left,
                    tr = xwing.top_right,
                    bl = xwing.bottom_left,
                    br = xwing.bottom_right
                );
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
