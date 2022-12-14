use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::{CollectIndexBitSet, Index};
use crate::strategies::{Strategy, StrategyResult};
use crate::{Coordinate, Value};
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes the X-Wing strategy.
pub struct XWing {
    enabled: bool,
}

impl XWing {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for XWing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "X-Wing")
    }
}

impl Strategy for XWing {
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
        let mut xwings: Vec<XWingCoords> = Vec::default();

        for value in Value::range() {
            // Identify all the cells that are not solved and contain the value under test.
            let indexes = state
                .iter_indexed()
                .filter(|&cell| !cell.is_solved() && cell.contains(value))
                .map(|cell| cell.index)
                .collect_bitset();

            // For the X-Wing to work, we need at least four matching cells
            // in order to form a single rectangle.
            if indexes.len() < 4 {
                continue;
            }

            // For each matching cell, scan for rectangles.
            for tl in indexes {
                let tl = tl.into_coordinate();

                for x in (tl.x + 1)..9 {
                    let tr = Coordinate::new(x, tl.y);
                    let has_tr = indexes.contains_coord(tr);
                    if !has_tr {
                        continue;
                    }

                    for y in (tl.y + 1)..9 {
                        let bl = Coordinate::new(tl.x, y);
                        let br = Coordinate::new(x, y);

                        let has_bl = indexes.contains_coord(bl);
                        let has_br = indexes.contains_coord(br);

                        // Ensure we found a rectangle.
                        if !(has_bl && has_br) {
                            continue;
                        }

                        // Ensure that only two matches exist in both rows OR both columns.
                        let mut top_count = 0;
                        let mut bottom_count = 0;
                        let mut left_count = 0;
                        let mut right_count = 0;
                        for xy in 0..9 {
                            top_count += indexes.contains_xy(xy, tr.y) as u32;
                            bottom_count += indexes.contains_xy(xy, br.y) as u32;
                            left_count += indexes.contains_xy(tl.x, xy) as u32;
                            right_count += indexes.contains_xy(br.x, xy) as u32;
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
                            top_left: tl.into_index(),
                            top_right: tr.into_index(),
                            bottom_left: bl.into_index(),
                            bottom_right: br.into_index(),
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
            for index in groups
                .get_peer_indexes(xwing.top_left, CellGroupType::StandardRow)
                .chain(groups.get_peer_indexes(xwing.bottom_left, CellGroupType::StandardRow))
                .chain(groups.get_peer_indexes(xwing.top_left, CellGroupType::StandardColumn))
                .chain(groups.get_peer_indexes(xwing.top_right, CellGroupType::StandardColumn))
                .filter(|idx| !xwing.eq(idx))
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
            trace!("No X-Wings could be applied");
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

impl PartialEq<Index> for XWingCoords {
    #[inline]
    fn eq(&self, other: &Index) -> bool {
        self.top_left == *other
            || self.top_right == *other
            || self.bottom_left == *other
            || self.bottom_right == *other
    }
}
