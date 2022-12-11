use crate::cell_group::{CellGroupType, CellGroups};
use crate::index::{Index, IndexBitSet};
use crate::prelude::{GameState, ValueBitSet};
use log::debug;

type PrintFn = fn(state: &GameState) -> ();

pub struct DefaultSolver {
    groups: CellGroups,
    print_fn: Option<PrintFn>,
}

#[derive(Debug, thiserror::Error)]
#[error("The game is unsolvable")]
pub struct Unsolvable(pub GameState);

#[derive(Debug)]
struct SmallestIndex {
    pub index: Index,
    pub size: usize,
}

impl Default for SmallestIndex {
    fn default() -> Self {
        Self {
            index: Index::default(),
            size: usize::MAX,
        }
    }
}

impl DefaultSolver {
    pub fn new<G: AsRef<CellGroups>>(groups: G) -> Self {
        Self {
            groups: groups.as_ref().clone(),
            print_fn: None,
        }
    }

    pub fn set_print_fn(&mut self, print_fn: PrintFn) {
        self.print_fn = Some(print_fn);
    }

    pub fn solve<S: AsRef<GameState>>(&self, state: S) -> Result<GameState, Unsolvable> {
        // We keep the last seen state as a reference to return when the board is unsolvable.
        let mut last_seen_state = state.as_ref().clone();

        let mut stack = vec![last_seen_state.clone()];
        'stack: while let Some(state) = stack.pop() {
            last_seen_state = state.clone();

            debug!("Taking state from stack ...");
            self.print_state(&state);

            if state.is_solved(&self.groups) {
                return Ok(state);
            }

            // Early exit the branch if needed.
            if !state.is_consistent(&self.groups) {
                debug!("Branch is inconsistent - ignoring");
                continue;
            }

            'solving: loop {
                match self.play_lonely_singles(&state) {
                    Err(_) => {
                        // continue with previous stack frame
                        continue;
                    }
                    Ok(_) => {
                        #[cfg(debug_assertions)]
                        {
                            if !state.is_consistent(&self.groups) {
                                debug!(
                                "Lonely singles resulted in inconsistent state - ignoring branch"
                            );
                                self.print_state(&state);
                                continue 'stack;
                            }
                        }

                        // always continue.
                    }
                }

                match self.play_hidden_singles(&state) {
                    Err(_) => {
                        // continue with previous stack frame
                        continue;
                    }
                    Ok(applied) => {
                        #[cfg(debug_assertions)]
                        {
                            if !state.is_consistent(&self.groups) {
                                debug!(
                                "Hidden singles resulted in inconsistent state - ignoring branch"
                            );
                                self.print_state(&state);
                                continue 'stack;
                            }
                        }

                        if applied {
                            continue 'solving;
                        }
                    }
                }

                match self.play_naked_twins(&state, CellGroupType::StandardColumn) {
                    Err(_) => {
                        // continue with previous stack frame
                        continue;
                    }
                    Ok(applied) => {
                        #[cfg(debug_assertions)]
                        {
                            if !state.is_consistent(&self.groups) {
                                debug!(
                                    "Naked twins on columns resulted in inconsistent state - ignoring branch"
                                );
                                self.print_state(&state);
                                continue 'stack;
                            }
                        }

                        if applied {
                            continue 'solving;
                        }
                    }
                }

                match self.play_naked_twins(&state, CellGroupType::StandardRow) {
                    Err(_) => {
                        // continue with previous stack frame
                        continue;
                    }
                    Ok(applied) => {
                        #[cfg(debug_assertions)]
                        {
                            if !state.is_consistent(&self.groups) {
                                debug!(
                                    "Naked twins on rows resulted in inconsistent state - ignoring branch"
                                );
                                self.print_state(&state);
                                continue 'stack;
                            }
                        }

                        if applied {
                            continue 'solving;
                        }
                    }
                }

                match self.play_naked_twins(&state, CellGroupType::StandardBlock) {
                    Err(_) => {
                        // continue with previous stack frame
                        continue;
                    }
                    Ok(applied) => {
                        #[cfg(debug_assertions)]
                        {
                            if !state.is_consistent(&self.groups) {
                                debug!(
                                    "Naked twins on blocks resulted in inconsistent state - ignoring branch"
                                );
                                self.print_state(&state);
                                continue 'stack;
                            }
                        }

                        if applied {
                            continue 'solving;
                        }
                    }
                }

                // No more strategies.
                break;
            }

            if !state.is_consistent(&self.groups) {
                debug!("Applying strategies resulted in inconsistent state - ignoring branch");
                self.print_state(&state);
                continue 'stack;
            }

            if state.is_solved(&self.groups) {
                return Ok(state);
            }

            let fork_index = match self.pick_index_to_fork_from(&state) {
                Some(index) => index,
                None => {
                    // Since the state is not a solution but we also cannot fork further,
                    // we need to continue with the next possible outcome.
                    debug_assert!(!state.is_consistent(&self.groups));
                    continue 'stack;
                }
            };
            let fork_cell = state.get_at_index(fork_index);
            debug_assert!(!fork_cell.is_impossible());
            debug_assert!(!fork_cell.is_solved());

            // Pick an arbitrary value to fork the state from.
            let fork_value = fork_cell.iter_candidates().next().unwrap();

            // Fork the board.
            debug!(
                "Forking state at {index:?} with value {value:?}",
                index = fork_index,
                value = fork_value
            );
            let forked = state.clone();
            forked.place_and_propagate_at_index(fork_index, fork_value, &self.groups);

            // In the current version of the board, simply forget the picked option.
            state.forget_at_index(fork_index, fork_value);
            stack.push(state.clone());

            // Emplace the forked version after that so that we try with fewer
            // candidates next. If it is inconsistent, skip it.
            if forked.is_consistent(&self.groups) {
                stack.push(forked);
            } else {
                debug!("Forked state is inconsistent - ignoring.");
            }
        }

        Err(Unsolvable(last_seen_state))
    }

    /// Identifies and realizes naked twins.
    ///
    /// ## Example
    /// A naked twin is a pair of cells that share the same values.
    ///
    /// Given three cells with the values `3 5`, `3 4` and `3 4`,
    /// `3 4` are the naked twins. Since they must appear in the last two
    /// cells, the `3` can be removed from the first cell.
    fn play_lonely_singles(&self, state: &GameState) -> Result<bool, InvalidGameState> {
        let mut observed_singles = IndexBitSet::empty();
        let mut removed_some = false;

        for index_under_test in (0..81).map(Index::new) {
            if !observed_singles.try_insert(index_under_test) {
                continue;
            }

            // Only consider cells that have exactly one value.
            let cell_under_test = state.get_at_index(index_under_test);
            if !cell_under_test.is_solved() {
                continue;
            }

            // Find all peers candidates.
            for index in self.groups.get_at_index(index_under_test).unwrap().iter() {
                if index == index_under_test {
                    continue;
                }

                let cell = state.get_at_index(index);
                if cell.as_bitset().contains_all(cell_under_test.as_bitset()) {
                    debug!(
                        "Removing lonely single {value:?} at {index:?} (from {iut:?})",
                        value = cell_under_test.as_bitset(),
                        index = index,
                        iut = index_under_test
                    );
                    removed_some = true;
                }

                state.forget_many_at_index(index, cell_under_test.as_bitset());
            }
        }

        return Ok(removed_some);
    }

    /// Identifies and realizes hidden singles.
    ///
    /// ## Example
    /// A single is a value that does not appear in any other cell.
    /// It is hidden when it appears along other values.
    ///
    /// Given two cells with the values `3 4` and `3 4 7`,
    /// `7` is the hidden single. Since it only appears in the second
    /// cell, it must be placed there (resulting in a "naked twin" pair of `3 4`).
    fn play_hidden_singles(&self, _state: &GameState) -> Result<bool, InvalidGameState> {
        Ok(false)
    }

    /// Identifies and realizes naked twins.
    ///
    /// ## Example
    /// A naked twin is a pair of cells that share the same values.
    ///
    /// Given three cells with the values `3 5`, `3 4` and `3 4`,
    /// `3 4` are the naked twins. Since they must appear in the last two
    /// cells, the `3` can be removed from the first cell.
    fn play_naked_twins(
        &self,
        state: &GameState,
        group_type: CellGroupType,
    ) -> Result<bool, InvalidGameState> {
        let mut twins_to_remove = Vec::default();
        let mut observed_twins = IndexBitSet::empty();

        for index_under_test in (0..81).map(Index::new) {
            if !observed_twins.try_insert(index_under_test) {
                continue;
            }

            // Only consider cells that have two possible candidates.
            let cell_under_test = state.get_at_index(index_under_test);
            if cell_under_test.len() != 2 {
                continue;
            }

            let mut possible_twins = Vec::default();

            // Find all possible twin candidates.
            for group in self
                .groups
                .get_groups_at_index(index_under_test)
                .unwrap()
                .iter()
                .filter(|g| g.group_type == group_type)
            {
                for index in group.iter_indexes() {
                    if observed_twins.contains(index) {
                        continue;
                    }

                    let cell = state.get_at_index(index);
                    if cell.len() != 2 {
                        continue;
                    }

                    if cell.as_bitset().eq(cell_under_test.as_bitset()) {
                        possible_twins.push(cell.into_indexed(index));
                    }
                }
            }

            // At least one other cell is required for a twin pair.
            if possible_twins.len() < 1 {
                continue;
            }

            // More than two "twins" are an error.
            if possible_twins.len() > 1 {
                return Err(InvalidGameState {});
            }

            debug_assert_eq!(possible_twins.len(), 1);
            let other_twin = possible_twins.iter().next().unwrap();

            // Eliminate twin values in other cells.
            observed_twins
                .insert(index_under_test)
                .insert(other_twin.index);

            debug!(
                "Twin pair detected in {group_type:?} at {a:?} and {b:?}: {values:?}",
                group_type = group_type,
                a = index_under_test.min(other_twin.index),
                b = index_under_test.max(other_twin.index),
                values = other_twin.as_bitset()
            );
            twins_to_remove.push(TwinPair {
                smaller: index_under_test.min(other_twin.index),
                larger: index_under_test.max(other_twin.index),
                values: other_twin.as_bitset().clone(),
            });
        }

        if twins_to_remove.is_empty() {
            return Ok(false);
        }

        // Iterate the detected twins, find their groups and eliminate the values.
        let mut applied_some = false;
        for twin in twins_to_remove.into_iter() {
            // The choice of the smaller or larger index here doesn't matter as they
            // are in the same group.
            for index in self
                .groups
                .get_groups_at_index(twin.smaller)
                .unwrap()
                .iter()
                .filter(|g| g.group_type == group_type)
                .flat_map(|g| g.iter_indexes())
                .filter(|&x| x != twin.smaller && x != twin.larger)
            {
                applied_some |= state.forget_many_at_index(index, &twin.values);
            }
        }

        return Ok(applied_some);

        struct TwinPair {
            smaller: Index,
            larger: Index,
            values: ValueBitSet,
        }
    }

    fn pick_index_to_fork_from(&self, state: &GameState) -> Option<Index> {
        // Identify the group with the fewest candidates.
        // Within that, identify the cell with the fewest options in that group.
        let mut smallest = SmallestIndex::default();

        for index_under_test in (0..81).map(Index::new) {
            let mut group_size = 0;
            let mut group_smallest = SmallestIndex::default();
            for index in self.groups.get_at_index(index_under_test).unwrap().iter() {
                let index_size = state.get_at_index(index).len();

                // Ignore solved or invalid cells.
                if index_size <= 1 {
                    continue;
                }

                // Accumulate the group size and keep track of the smallest index
                // within that group.
                group_size += index_size;
                if index_size < group_smallest.size {
                    group_smallest = SmallestIndex {
                        index,
                        size: index_size,
                    }
                }
            }

            if group_size < smallest.size && group_size > 0 {
                smallest = SmallestIndex {
                    index: group_smallest.index,
                    size: group_size,
                };
            }
        }

        if smallest.size != usize::MAX {
            Some(smallest.index)
        } else {
            None
        }
    }

    fn print_state(&self, state: &GameState) {
        if !log::log_enabled!(log::Level::Debug) {
            return;
        }
        if let Some(print_fn) = self.print_fn {
            print_fn(state);
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("An invalid game state was reached")]
struct InvalidGameState {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solving_sudoku_works() {
        let game = crate::example_games::sudoku::example_sudoku();
        let solver = DefaultSolver::new(&game);
        let result = solver.solve(&game);
        assert!(result.is_ok());

        let solution = result.unwrap();
        assert!(solution.is_consistent(&game.groups));
        assert!(solution.is_solved(&game.groups));
    }

    #[test]
    fn solving_sudoku_with_naked_twins() {
        let game = crate::example_games::sudoku::example_sudoku_naked_twins();
        let solver = DefaultSolver::new(&game);
        let result = solver.solve(&game);
        assert!(result.is_ok());

        let solution = result.unwrap();
        assert!(solution.is_consistent(&game.groups));
        assert!(solution.is_solved(&game.groups));
    }
}
