use crate::cell_group::{CellGroups, CollectIndexes};
use crate::index::Index;
use crate::value::Value;
use crate::GameState;
use rand::seq::SliceRandom;
use rand::Rng;

/// Generates complete, valid, randomly-filled solution grids.
///
/// Uses backtracking with MRV (Minimum Remaining Values) cell selection and
/// randomized candidate ordering: at each step the most-constrained unsolved
/// cell is filled with a randomly-shuffled candidate. Full naked-singles
/// propagation is applied after each placement to prune the search space.
/// Every call with a different RNG state produces a different grid.
///
/// Works with any [`CellGroups`] configuration (standard 9×9, Hypersudoku,
/// Nonomino, etc.) because group membership is fully encapsulated in the
/// groups passed at construction time.
pub struct GridGenerator {
    groups: CellGroups,
}

impl GridGenerator {
    pub fn new(groups: CellGroups) -> Self {
        Self { groups }
    }

    /// Generates a complete valid solution grid.
    ///
    /// # Panics
    ///
    /// Panics if the configured groups make it impossible to fill all 81 cells
    /// (i.e. the group layout itself is inconsistent). Valid Sudoku group
    /// layouts always succeed.
    pub fn generate<R: Rng>(&self, rng: &mut R) -> GameState {
        fill_grid(&GameState::new(), &self.groups, rng)
            .expect("group layout is inconsistent: no complete grid exists")
    }

    /// Attempts to generate a complete valid solution grid, returning `None`
    /// if the configured groups make it impossible to fill all 81 cells.
    pub fn try_generate<R: Rng>(&self, rng: &mut R) -> Option<GameState> {
        fill_grid(&GameState::new(), &self.groups, rng)
    }

    /// Like [`try_generate`] but aborts after `max_nodes` recursive calls,
    /// returning `None` on timeout. Useful for quickly rejecting degenerate
    /// group layouts that have no valid grid or very few valid grids.
    pub fn try_generate_limited<R: Rng>(&self, rng: &mut R, max_nodes: usize) -> Option<GameState> {
        let mut remaining = max_nodes;
        fill_grid_limited(&GameState::new(), &self.groups, rng, &mut remaining)
    }
}

/// Recursively fills the most-constrained unsolved cell (MRV heuristic),
/// returning the first complete grid found or `None` if the current partial
/// assignment has no valid completion.
///
/// MRV (Minimum Remaining Values) picks the cell with the fewest candidates
/// at each step. This dramatically reduces backtracking compared to left-to-right
/// order, especially for irregular group layouts like Nonomino.
fn fill_grid<R: Rng>(state: &GameState, groups: &CellGroups, rng: &mut R) -> Option<GameState> {
    let index = match most_constrained_unsolved(state) {
        Some(idx) => idx,
        None => return Some(state.clone()), // all cells solved
    };

    let cell = state.get_at_index(index);
    let mut candidates: Vec<Value> = cell.iter_candidates().collect();
    candidates.shuffle(rng);

    for value in candidates {
        let branch = state.clone();
        branch.place_and_propagate_at_index(index, value, groups);

        // Propagate naked singles to completion before recursing.
        // place_and_propagate only removes `value` from direct peers; if a
        // peer drops to one candidate, its own peers are not yet updated.
        // Skipping this leaves "ghost" conflicts undetected until is_solved
        // catches them at the top, producing invalid grids.
        if propagate_naked_singles(&branch, groups, index) {
            if let Some(result) = fill_grid(&branch, groups, rng) {
                return Some(result);
            }
        }
    }

    None
}

/// Like [`fill_grid`] but decrements `remaining` at each recursive call and
/// returns `None` immediately when `remaining` reaches zero.
fn fill_grid_limited<R: Rng>(
    state: &GameState,
    groups: &CellGroups,
    rng: &mut R,
    remaining: &mut usize,
) -> Option<GameState> {
    if *remaining == 0 {
        return None;
    }
    *remaining -= 1;

    let index = match most_constrained_unsolved(state) {
        Some(idx) => idx,
        None => return Some(state.clone()),
    };

    let cell = state.get_at_index(index);
    let mut candidates: Vec<Value> = cell.iter_candidates().collect();
    candidates.shuffle(rng);

    for value in candidates {
        let branch = state.clone();
        branch.place_and_propagate_at_index(index, value, groups);
        if propagate_naked_singles(&branch, groups, index) {
            if let Some(result) = fill_grid_limited(&branch, groups, rng, remaining) {
                return Some(result);
            }
        }
    }

    None
}

/// Propagates naked-singles constraints after placing a value at `seed`.
///
/// `place_and_propagate` already removed the placed value from all direct
/// peers of `seed`. Some of those peers may now have dropped to a single
/// candidate (naked single), requiring their own values to be removed from
/// their peers. This function handles that cascade until quiescence.
///
/// Returns `false` if any cell reaches 0 candidates (dead end).
fn propagate_naked_singles(state: &GameState, groups: &CellGroups, seed: Index) -> bool {
    // Seed's value was already propagated by place_and_propagate. The queue
    // starts from solved peers of seed — those are the newly-created naked
    // singles that need their own values removed from their peers.
    let mut queue: Vec<Index> = Vec::new();
    let seed_peers = groups
        .get_peers_at_index(seed, CollectIndexes::ExcludeSelf)
        .expect("CellGroups must have peers for every cell index");
    for peer_index in seed_peers.iter() {
        let peer = state.get_at_index(peer_index);
        if peer.is_impossible() {
            return false;
        }
        if peer.is_solved() {
            queue.push(peer_index);
        }
    }

    while let Some(index) = queue.pop() {
        let cell = state.get_at_index(index);
        if !cell.is_solved() {
            continue;
        }
        let value = cell.iter_candidates().next().unwrap();

        let peers = groups
            .get_peers_at_index(index, CollectIndexes::ExcludeSelf)
            .expect("CellGroups must have peers for every cell index");

        for peer_index in peers.iter() {
            let peer = state.get_at_index(peer_index);
            if !peer.contains(value) {
                continue;
            }
            state.forget_at_index(peer_index, value);
            let peer_after = state.get_at_index(peer_index);
            if peer_after.is_impossible() {
                return false;
            }
            if peer_after.is_solved() {
                queue.push(peer_index);
            }
        }
    }

    true
}

/// Returns the index of the unsolved cell with the fewest remaining candidates,
/// or `None` if all cells are already solved.
fn most_constrained_unsolved(state: &GameState) -> Option<Index> {
    Index::range()
        .filter_map(|index| {
            let cell = state.get_at_index(index);
            if cell.is_solved() {
                None
            } else {
                Some((index, cell.len()))
            }
        })
        .min_by_key(|(_, len)| *len)
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell_group::CellGroups;
    use crate::index::Index;
    use crate::value::Value;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn standard_groups() -> CellGroups {
        CellGroups::default()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns()
    }

    #[test]
    fn generate_returns_complete_valid_grid() {
        let generator = GridGenerator::new(standard_groups());
        let mut rng = StdRng::seed_from_u64(42);
        let grid = generator.generate(&mut rng);

        assert!(grid.is_solved(&generator.groups));
        assert!(grid.is_consistent(&generator.groups));
    }

    #[test]
    fn generate_produces_different_grids_for_different_seeds() {
        let generator = GridGenerator::new(standard_groups());

        let grid_a = generator.generate(&mut StdRng::seed_from_u64(1));
        let grid_b = generator.generate(&mut StdRng::seed_from_u64(2));

        let cells_a: Vec<_> = grid_a.iter().collect();
        let cells_b: Vec<_> = grid_b.iter().collect();
        assert_ne!(cells_a, cells_b);
    }

    #[test]
    fn generate_same_seed_produces_same_grid() {
        let generator = GridGenerator::new(standard_groups());

        let grid_a = generator.generate(&mut StdRng::seed_from_u64(99));
        let grid_b = generator.generate(&mut StdRng::seed_from_u64(99));

        let cells_a: Vec<_> = grid_a.iter().collect();
        let cells_b: Vec<_> = grid_b.iter().collect();
        assert_eq!(cells_a, cells_b);
    }

    #[test]
    fn generate_hypersudoku_grid() {
        let groups = CellGroups::default()
            .with_hypersudoku_windows()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns();

        let generator = GridGenerator::new(groups);
        let mut rng = StdRng::seed_from_u64(42);
        let grid = generator.generate(&mut rng);

        assert!(grid.is_solved(&generator.groups));
        assert!(grid.is_consistent(&generator.groups));
    }

    // --- propagate_naked_singles unit tests ---

    #[test]
    fn propagate_returns_false_when_direct_peer_is_impossible() {
        // Cell 1 (same row as cell 0) is pre-set to ONE.
        // Placing ONE in cell 0 removes ONE from cell 1 → cell 1 becomes impossible.
        // propagate_naked_singles must detect it immediately and return false.
        let groups = standard_groups();
        let state = GameState::new();
        state.set_at_index(Index::new(1), Value::ONE);
        state.place_and_propagate_at_index(Index::new(0), Value::ONE, &groups);

        assert!(!super::propagate_naked_singles(
            &state,
            &groups,
            Index::new(0)
        ));
    }

    #[test]
    fn propagate_returns_false_when_cascade_creates_impossible_cell() {
        // Cell 1 (row 0, col 1) is solved to FOUR.
        // Cell 10 (row 1, col 1) is also solved to FOUR — same column, so they are peers.
        // Placing FIVE in cell 0 (row 0, col 0) makes cell 1 a solved peer of cell 0.
        // Cascading from cell 1 removes FOUR from cell 10 → cell 10 becomes impossible.
        let groups = standard_groups();
        let state = GameState::new();
        state.set_at_index(Index::new(1), Value::FOUR);
        state.set_at_index(Index::new(10), Value::FOUR);
        state.place_and_propagate_at_index(Index::new(0), Value::FIVE, &groups);

        assert!(!super::propagate_naked_singles(
            &state,
            &groups,
            Index::new(0)
        ));
    }

    #[test]
    fn propagate_returns_true_for_conflict_free_placement() {
        let groups = standard_groups();
        let state = GameState::new();
        state.place_and_propagate_at_index(Index::new(0), Value::ONE, &groups);

        assert!(super::propagate_naked_singles(
            &state,
            &groups,
            Index::new(0)
        ));
    }

    #[test]
    #[ignore = "The example nonomino layout has very few valid complete grids, \
                making generation slow for this specific region configuration. \
                Nonomino generation will be properly tested in the nonomino region \
                generator (#26), which produces layouts with many valid complete grids."]
    fn generate_nonomino_grid() {
        let game = crate::example_games::nonomino::example_nonomino();
        let generator = GridGenerator::new(game.groups);
        let mut rng = StdRng::seed_from_u64(42);
        let grid = generator.generate(&mut rng);

        assert!(grid.is_solved(&generator.groups));
        assert!(grid.is_consistent(&generator.groups));
    }
}
