use crate::cell_group::CellGroups;
use crate::index::Index;
use crate::prelude::GameState;

pub struct DefaultSolver {
    groups: CellGroups,
}

#[derive(Debug, thiserror::Error)]
#[error("The game is unsolvable")]
pub struct Unsolvable {}

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
    pub fn new(groups: CellGroups) -> Self {
        Self { groups }
    }

    pub fn solve(&self, state: GameState) -> Result<GameState, Unsolvable> {
        let mut stack = vec![state.clone()];
        'stack: while let Some(state) = stack.pop() {
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
            let forked = state.clone();
            forked.place_at_index(fork_index, fork_value, &self.groups);

            // In the current version of the board, simply forget the picked option.
            state.forget_at_index(fork_index, fork_value);
            stack.push(state.clone());

            // Emplace the forked version after that so that we try with fewer
            // candidates next. If it is inconsistent, skip it.
            if forked.is_consistent(&self.groups) {
                stack.push(forked);
            }
        }

        Err(Unsolvable {})
    }

    fn pick_index_to_fork_from(&self, state: &GameState) -> Option<Index> {
        // Identify the group with the fewest candidates.
        // Within that, identify the cell with the fewest options in that group.
        let mut smallest = SmallestIndex::default();

        for i in 0..81 {
            let mut group_size = 0;
            let mut group_smallest = SmallestIndex::default();
            for group in self.groups.get_at_index(Index::new(i)).iter().flatten() {
                for index in group.iter_indexes() {
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
}
