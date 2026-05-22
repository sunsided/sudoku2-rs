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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_initializes_root_entry() {
        let stack = StateStack::new_with(GameState::new());
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.max_depth(), 1);
        assert_eq!(stack.num_forks(), 0);
    }

    #[test]
    fn push_grows_stack_and_tracks_forks_and_max_depth() {
        let mut stack = StateStack::new_with(GameState::new());
        stack.push(GameState::new());
        stack.push(GameState::new());
        assert_eq!(stack.len(), 3);
        assert_eq!(stack.num_forks(), 2);
        assert_eq!(stack.max_depth(), 3);

        let _ = stack.pop();
        // max_depth must not shrink after a pop.
        assert_eq!(stack.max_depth(), 3);
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn pop_returns_entries_in_lifo_order_then_none() {
        let mut stack = StateStack::new_with(GameState::new());
        stack.push(GameState::new());
        stack.push(GameState::new());

        let top = stack.pop().expect("two entries pushed");
        assert_eq!(format!("{}", top.branch_id), "#2");

        let mid = stack.pop().expect("one entry pushed");
        assert_eq!(format!("{}", mid.branch_id), "#1");

        let root = stack.pop().expect("root entry");
        assert_eq!(format!("{}", root.branch_id), "#0");

        assert!(stack.pop().is_none());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn branch_id_debug_and_display_match() {
        let id = BranchId(7);
        assert_eq!(format!("{}", id), "#7");
        assert_eq!(format!("{:?}", id), "#7");
    }
}
