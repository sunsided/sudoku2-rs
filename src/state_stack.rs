use crate::GameState;
use log::trace;
use std::fmt::{Debug, Display, Formatter};

/// A state stack used for branching solvers that assigns IDs
/// to forked [`GameState`] instances.
pub struct StateStack {
    stack: Vec<StateStackEntry>,
    max_depth: usize,
    forks: usize,
}

pub struct StateStackEntry {
    pub branch_id: BranchId,
    pub state: GameState,
}

pub struct BranchId(usize);

impl StateStack {
    pub fn new_with(state: GameState) -> Self {
        Self {
            stack: vec![StateStackEntry {
                branch_id: BranchId(0),
                state,
            }],
            max_depth: 1,
            forks: 0,
        }
    }

    pub fn push(&mut self, state: GameState) {
        self.forks += 1;
        self.stack.push(StateStackEntry {
            branch_id: BranchId(self.forks),
            state,
        });
        self.max_depth = self.max_depth.max(self.len());
        trace!("Enqueued state as id {id}", id = self.forks);
    }

    pub fn pop(&mut self) -> Option<StateStackEntry> {
        if let Some(entry) = self.stack.pop() {
            trace!("Dequeued state with id {id}", id = entry.branch_id);
            Some(entry)
        } else {
            trace!("Stack is empty");
            None
        }
    }

    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    pub fn num_forks(&self) -> usize {
        self.forks
    }
}

impl Debug for BranchId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl Display for BranchId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}
