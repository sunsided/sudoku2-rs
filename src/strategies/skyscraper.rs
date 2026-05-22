use crate::board_stats::BoardStatsCache;
use crate::cell_group::{CellGroupType, CellGroups, CollectIndexes};
use crate::game_state::{GameState, InvalidGameState};
use crate::index::Index;
use crate::strategies::{Strategy, StrategyResult};
use crate::{Coordinate, Value};
use log::{debug, trace};
use std::fmt::{Debug, Formatter};

/// Identifies and realizes the Skyscraper strategy.
///
/// A Skyscraper is a single-digit pattern built on two strong links in the
/// same group orientation (two rows, or two columns) where the digit `d`
/// appears as a candidate in exactly two cells per line. The two lines share
/// **one** cross-axis coordinate (the "base"); the other two cells (the
/// "roof") sit at different cross-axis coordinates. Any cell that sees both
/// roof cells must not be the digit `d`.
///
/// Compared with X-Wing, Skyscraper requires only a single shared cross-axis
/// (X-Wing requires both) and therefore catches eliminations X-Wing does not,
/// at a tighter scan cost. It is registered before X-Wing in the default
/// pipeline.
pub struct Skyscraper {
    enabled: bool,
}

impl Skyscraper {
    pub fn new_box(enabled: bool) -> Box<Self> {
        Box::new(Self { enabled })
    }
}

impl Debug for Skyscraper {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Skyscraper")
    }
}

impl Strategy for Skyscraper {
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
        let mut hits: Vec<SkyscraperHit> = Vec::default();

        // Per-value candidate positions are built lazily on first access.
        let stats_ref = stats.get();
        for value in Value::range() {
            let v_idx = (value.get() - 1) as usize;
            let indexes = stats_ref.per_value_unsolved[v_idx];

            // A Skyscraper needs four candidate cells across two lines.
            if indexes.len() < 4 {
                continue;
            }

            // Row-oriented Skyscrapers: two rows with exactly two
            // `value`-candidate cells each, sharing exactly one column.
            let mut row_pairs: Vec<LinePair> = Vec::with_capacity(9);
            for row in 0..9u8 {
                let mut cols = [0u8; 2];
                let mut count = 0usize;
                for col in 0..9u8 {
                    if indexes.contains_xy(col, row) {
                        if count < 2 {
                            cols[count] = col;
                        }
                        count += 1;
                    }
                }
                if count == 2 {
                    row_pairs.push(LinePair {
                        line: row,
                        a: cols[0],
                        b: cols[1],
                    });
                }
            }
            collect_skyscraper_hits(&row_pairs, value, LineOrientation::Rows, &mut hits);

            // Column-oriented Skyscrapers (mirror case).
            let mut col_pairs: Vec<LinePair> = Vec::with_capacity(9);
            for col in 0..9u8 {
                let mut rows = [0u8; 2];
                let mut count = 0usize;
                for row in 0..9u8 {
                    if indexes.contains_xy(col, row) {
                        if count < 2 {
                            rows[count] = row;
                        }
                        count += 1;
                    }
                }
                if count == 2 {
                    col_pairs.push(LinePair {
                        line: col,
                        a: rows[0],
                        b: rows[1],
                    });
                }
            }
            collect_skyscraper_hits(&col_pairs, value, LineOrientation::Columns, &mut hits);
        }

        if hits.is_empty() {
            return Ok(StrategyResult::NoChange);
        }

        let mut applied_some = false;
        for hit in hits {
            debug_assert_ne!(hit.roof_a, hit.roof_b);
            debug_assert_ne!(hit.base_a, hit.base_b);

            // Cells that see both roof cells = intersection of their peer sets,
            // minus the four pattern cells themselves.
            let peers_a = groups
                .get_peers_at_index(hit.roof_a, CollectIndexes::ExcludeSelf)
                .expect("group missing for roof cell");
            let peers_b = groups
                .get_peers_at_index(hit.roof_b, CollectIndexes::ExcludeSelf)
                .expect("group missing for roof cell");
            let common = peers_a
                .intersect(&peers_b)
                .without_index(hit.base_a)
                .without_index(hit.base_b)
                .without_index(hit.roof_a)
                .without_index(hit.roof_b);

            let mut applied = false;
            for idx in common.iter() {
                applied |= state.forget_at_index(idx, hit.value);
            }

            applied_some |= applied;
            if applied {
                debug!(
                    "Applied Skyscraper for value {value:?} bases {ba:?},{bb:?} roofs {ra:?},{rb:?}",
                    value = hit.value,
                    ba = hit.base_a,
                    bb = hit.base_b,
                    ra = hit.roof_a,
                    rb = hit.roof_b,
                );
            }
        }

        if applied_some {
            Ok(StrategyResult::AppliedChange)
        } else {
            trace!("No Skyscrapers could be applied");
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

#[derive(Copy, Clone)]
struct LinePair {
    /// The row (for row-oriented pairs) or column (for column-oriented pairs)
    /// that this pair lives on.
    line: u8,
    /// The first cross-axis coordinate (column for rows, row for columns).
    a: u8,
    /// The second cross-axis coordinate.
    b: u8,
}

#[derive(Copy, Clone)]
enum LineOrientation {
    Rows,
    Columns,
}

struct SkyscraperHit {
    value: Value,
    base_a: Index,
    base_b: Index,
    roof_a: Index,
    roof_b: Index,
}

fn collect_skyscraper_hits(
    pairs: &[LinePair],
    value: Value,
    orientation: LineOrientation,
    hits: &mut Vec<SkyscraperHit>,
) {
    for i in 0..pairs.len() {
        for j in (i + 1)..pairs.len() {
            let p = pairs[i];
            let q = pairs[j];

            // Find the single shared cross-axis coordinate.
            // If two are shared, the pattern is an X-Wing and is handled by
            // the X-Wing strategy instead.
            let (base_cross, roof_cross_p, roof_cross_q) = if p.a == q.a {
                (p.a, p.b, q.b)
            } else if p.a == q.b {
                (p.a, p.b, q.a)
            } else if p.b == q.a {
                (p.b, p.a, q.b)
            } else if p.b == q.b {
                (p.b, p.a, q.a)
            } else {
                continue;
            };

            // Reject the X-Wing rectangle case (both cross-axis coords shared).
            if roof_cross_p == roof_cross_q {
                continue;
            }

            let (base_a, base_b, roof_a, roof_b) = match orientation {
                LineOrientation::Rows => (
                    Coordinate::new(base_cross, p.line).into_index(),
                    Coordinate::new(base_cross, q.line).into_index(),
                    Coordinate::new(roof_cross_p, p.line).into_index(),
                    Coordinate::new(roof_cross_q, q.line).into_index(),
                ),
                LineOrientation::Columns => (
                    Coordinate::new(p.line, base_cross).into_index(),
                    Coordinate::new(q.line, base_cross).into_index(),
                    Coordinate::new(p.line, roof_cross_p).into_index(),
                    Coordinate::new(q.line, roof_cross_q).into_index(),
                ),
            };

            trace!(
                "Identified Skyscraper for value {value:?} bases {ba:?},{bb:?} roofs {ra:?},{rb:?}",
                value = value,
                ba = base_a,
                bb = base_b,
                ra = roof_a,
                rb = roof_b,
            );
            hits.push(SkyscraperHit {
                value,
                base_a,
                base_b,
                roof_a,
                roof_b,
            });
        }
    }
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

    /// Removes the supplied candidate at `(x, y)`.
    fn forget(state: &GameState, x: u8, y: u8, value: Value) {
        let index = Coordinate::new(x, y).into_index();
        state.forget_at_index(index, value);
    }

    #[test]
    fn row_skyscraper_eliminates_seeing_cells() {
        let groups = standard_groups();
        let state = GameState::new();

        // Confine value 7 in row 0 to columns 0 and 3, and in row 4 to columns
        // 0 and 5. Shared column = 0 (base). Roof cells = (3,0) and (5,4).
        // Cells (3,4) and (5,0) see both roofs and must drop 7.
        for x in [1u8, 2, 4, 5, 6, 7, 8] {
            forget(&state, x, 0, Value::SEVEN);
        }
        for x in [1u8, 2, 3, 4, 6, 7, 8] {
            forget(&state, x, 4, Value::SEVEN);
        }

        let strat = Skyscraper { enabled: true };
        let res = strat
            .apply(&state, &groups, &BoardStatsCache::new(&state))
            .unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        // Both roof-seeing cells must have lost 7 as a candidate.
        assert!(!state.get_at_xy(3, 4).contains(Value::SEVEN));
        assert!(!state.get_at_xy(5, 0).contains(Value::SEVEN));

        // Base and roof cells must keep 7 as a candidate.
        assert!(state.get_at_xy(0, 0).contains(Value::SEVEN));
        assert!(state.get_at_xy(3, 0).contains(Value::SEVEN));
        assert!(state.get_at_xy(0, 4).contains(Value::SEVEN));
        assert!(state.get_at_xy(5, 4).contains(Value::SEVEN));
    }

    #[test]
    fn column_skyscraper_eliminates_seeing_cells() {
        let groups = standard_groups();
        let state = GameState::new();

        // Mirror of the row test, on columns 0 and 4.
        for y in [1u8, 2, 4, 5, 6, 7, 8] {
            forget(&state, 0, y, Value::SEVEN);
        }
        for y in [1u8, 2, 3, 4, 6, 7, 8] {
            forget(&state, 4, y, Value::SEVEN);
        }

        let strat = Skyscraper { enabled: true };
        let res = strat
            .apply(&state, &groups, &BoardStatsCache::new(&state))
            .unwrap();
        assert_eq!(res, StrategyResult::AppliedChange);

        assert!(!state.get_at_xy(4, 3).contains(Value::SEVEN));
        assert!(!state.get_at_xy(0, 5).contains(Value::SEVEN));

        assert!(state.get_at_xy(0, 0).contains(Value::SEVEN));
        assert!(state.get_at_xy(0, 3).contains(Value::SEVEN));
        assert!(state.get_at_xy(4, 0).contains(Value::SEVEN));
        assert!(state.get_at_xy(4, 5).contains(Value::SEVEN));
    }

    #[test]
    fn xwing_rectangle_is_not_a_skyscraper() {
        let groups = standard_groups();
        let state = GameState::new();

        // Confine value 7 in row 0 and row 4 to the *same* two columns
        // (0 and 3). This is an X-Wing, not a Skyscraper. Skyscraper must
        // not claim a change.
        for x in [1u8, 2, 4, 5, 6, 7, 8] {
            forget(&state, x, 0, Value::SEVEN);
        }
        for x in [1u8, 2, 4, 5, 6, 7, 8] {
            forget(&state, x, 4, Value::SEVEN);
        }

        // Suppress all column-oriented matches: any column where 7 still
        // appears in exactly two cells could form a column Skyscraper, so
        // make sure no spurious column Skyscraper sneaks in by leaving the
        // remaining rows untouched (each still has 9 candidates of 7).

        let strat = Skyscraper { enabled: true };
        let res = strat
            .apply(&state, &groups, &BoardStatsCache::new(&state))
            .unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }

    #[test]
    fn no_change_on_pristine_board() {
        let groups = standard_groups();
        let state = GameState::new();

        let strat = Skyscraper { enabled: true };
        let res = strat
            .apply(&state, &groups, &BoardStatsCache::new(&state))
            .unwrap();
        assert_eq!(res, StrategyResult::NoChange);
    }
}
