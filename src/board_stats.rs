use crate::game_state::GameState;
use crate::index::{Index, IndexBitSet};
use crate::value::ValueBitSet;
use std::cell::{Ref, RefCell};

/// Board-wide summary built once per pipeline iteration and consumed by
/// strategies that would otherwise re-scan the board.
///
/// Strategies such as X-Wing, Skyscraper, W-Wing, and XY-Wing all need the
/// same shape of data - "which cells still hold value `v`" and "where are the
/// bivalue cells" - so computing it once in the solver and passing it through
/// avoids the redundant per-strategy board scans.
#[derive(Debug, Clone)]
pub struct BoardStats {
    /// For each value `v` (0-indexed), the set of unsolved cell indexes that
    /// still hold `v` as a candidate. Solved cells are excluded - a placed
    /// value cannot participate in fish / wing patterns.
    pub per_value_unsolved: [IndexBitSet; 9],
    /// All unsolved cells with exactly two candidates, captured with the
    /// candidate pair so strategies can avoid re-reading the cell.
    pub bivalues: Vec<Bivalue>,
}

#[derive(Debug, Clone, Copy)]
pub struct Bivalue {
    pub index: Index,
    pub values: ValueBitSet,
}

impl BoardStats {
    /// Build the stats with a single pass over the board.
    pub fn from_state(state: &GameState) -> Self {
        let mut per_value_unsolved: [IndexBitSet; 9] = Default::default();
        let mut bivalues: Vec<Bivalue> = Vec::with_capacity(16);

        for cell in state.iter_indexed() {
            let len = cell.len();
            if len <= 1 {
                continue;
            }
            let values = cell.to_bitset();
            for value in values.iter() {
                let v_idx = (value.get() - 1) as usize;
                per_value_unsolved[v_idx].insert(cell.index);
            }
            if len == 2 {
                bivalues.push(Bivalue {
                    index: cell.index,
                    values,
                });
            }
        }

        Self {
            per_value_unsolved,
            bivalues,
        }
    }
}

/// Lazy cache for `BoardStats`. The pipeline restart loop creates one cache
/// per quiescent run, invalidating it whenever a strategy mutates state.
/// Strategies that don't consult stats never trigger the build, so simple
/// puzzles never pay the scan cost.
pub struct BoardStatsCache<'a> {
    state: &'a GameState,
    inner: RefCell<Option<BoardStats>>,
}

impl<'a> BoardStatsCache<'a> {
    #[inline]
    pub fn new(state: &'a GameState) -> Self {
        Self {
            state,
            inner: RefCell::new(None),
        }
    }

    /// Returns a borrow of the stats, building them on the first call after
    /// construction or `invalidate`.
    pub fn get(&self) -> Ref<'_, BoardStats> {
        if self.inner.borrow().is_none() {
            *self.inner.borrow_mut() = Some(BoardStats::from_state(self.state));
        }
        Ref::map(self.inner.borrow(), |opt| {
            opt.as_ref().expect("just populated")
        })
    }

    /// Drops the cached stats; the next `get` will rebuild from the current
    /// state.
    #[inline]
    pub fn invalidate(&self) {
        *self.inner.borrow_mut() = None;
    }
}
