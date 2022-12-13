use crate::cell_group::{CellGroups, CollectIndexes};
use crate::game_state::InvalidGameState;
use crate::index::Index;
use crate::strategies::{HiddenSingles, NakedSingles, NakedTwins, Strategy, StrategyResult, XWing};
use crate::GameState;
use log::debug;

type PrintFn = fn(state: &GameState) -> ();

pub struct DefaultSolver {
    groups: CellGroups,
    print_fn: Option<PrintFn>,
    strategies: Vec<Box<dyn Strategy>>,
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
        let strategies: Vec<Box<dyn Strategy>> = vec![
            XWing::new_box(),
            NakedSingles::new_box(),
            HiddenSingles::new_box(),
            NakedTwins::new_box(),
        ];

        Self {
            groups: groups.as_ref().clone(),
            print_fn: None,
            strategies,
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

            if self.apply_strategies(&state).is_err() {
                debug!("Applying strategies resulted in inconsistent state - ignoring branch");
                self.print_state(&state);
                continue 'stack;
            }

            debug_assert!(state.is_consistent(&self.groups));

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

    /// Applies different strategies for solving the board without branching.
    fn apply_strategies(&self, state: &GameState) -> Result<(), InvalidGameState> {
        'solving: loop {
            'next_strategy: for strategy in self.strategies.iter() {
                match strategy.apply(&state, &self.groups) {
                    Err(e) => return Err(e),
                    Ok(outcome) => {
                        #[cfg(debug_assertions)]
                        {
                            if !state.is_consistent(&self.groups) {
                                debug!(
                                    "{strategy:?} resulted in inconsistent state - ignoring branch",
                                    strategy = strategy
                                );
                                self.print_state(&state);
                                return Err(InvalidGameState {});
                            }
                        }

                        // Some strategies do not require a restart.
                        if strategy.always_continue() {
                            continue 'next_strategy;
                        }

                        // Assuming that strategies are ordered by complexity,
                        // restarting with the easiest one should result in
                        // fastest gains. Because of that, when changes were applied
                        // we start over until all strategies report no change.
                        match outcome {
                            StrategyResult::AppliedChange => continue 'solving,
                            StrategyResult::NoChange => continue 'next_strategy,
                        }
                    }
                }
            }

            // No more strategies.
            break;
        }

        if state.is_consistent(&self.groups) {
            Ok(())
        } else {
            return Err(InvalidGameState {});
        }
    }

    fn pick_index_to_fork_from(&self, state: &GameState) -> Option<Index> {
        // Identify the group with the fewest candidates.
        // Within that, identify the cell with the fewest options in that group.
        let mut smallest = SmallestIndex::default();

        for index_under_test in Index::range() {
            let mut group_size = 0;
            let mut group_smallest = SmallestIndex::default();
            for index in self
                .groups
                .get_peers_at_index(index_under_test, CollectIndexes::IncludeSelf)
                .unwrap()
                .iter()
            {
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
    fn solving_sudoku_with_hidden_singles() {
        let game = crate::example_games::sudoku2::example_sudoku();
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

    #[test]
    fn solving_nonomino() {
        let game = crate::example_games::nonomino::example_nonomino();
        let solver = DefaultSolver::new(&game);
        let result = solver.solve(&game);
        assert!(result.is_ok());

        let solution = result.unwrap();
        assert!(solution.is_consistent(&game.groups));
        assert!(solution.is_solved(&game.groups));
    }

    #[test]
    fn solving_hypersudoku() {
        let game = crate::example_games::hypersudoku::example_hypersudoku();
        let solver = DefaultSolver::new(&game);
        let result = solver.solve(&game);
        assert!(result.is_ok());

        let solution = result.unwrap();
        assert!(solution.is_consistent(&game.groups));
        assert!(solution.is_solved(&game.groups));
    }
}
