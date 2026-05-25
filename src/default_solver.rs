use crate::board_stats::BoardStatsCache;
use crate::cell_group::CellGroups;
use crate::game_state::InvalidGameState;
use crate::index::Index;
use crate::state_stack::{StateStack, StateStackEntry};
use crate::strategies::{
    HPattern, HiddenQuads, HiddenSingles, HiddenTriples, HiddenTwins, NakedQuads, NakedSingles,
    NakedTriples, NakedTwins, Skyscraper, Strategy, StrategyResult, UniqueRectangle, WWing, XWing,
    XYWing,
};
use crate::value::Value;
use crate::GameState;
use log::{debug, trace};

type PrintFn = fn(state: &GameState) -> ();

pub struct DefaultSolver {
    groups: CellGroups,
    print_fn: Option<PrintFn>,
    strategies: Vec<Box<dyn Strategy>>,
}

#[derive(Debug, thiserror::Error)]
#[error("The game is unsolvable")]
pub struct Unsolvable(pub GameState);

#[derive(Debug, Clone)]
pub struct SolverStep {
    pub state: GameState,
    pub strategy: String,
    pub index: Option<Index>,
    pub value: Option<Value>,
    pub placed_cells: usize,
    pub eliminated_candidates: usize,
    pub solved: bool,
}

pub struct DefaultSolverConfig {
    pub hidden_singles: bool,
    pub naked_twins: bool,
    pub hidden_twins: bool,
    pub naked_triples: bool,
    pub hidden_triples: bool,
    pub naked_quads: bool,
    pub hidden_quads: bool,
    pub h_pattern: bool,
    pub skyscraper: bool,
    pub xwings: bool,
    pub xy_wing: bool,
    pub w_wing: bool,
    /// Unique Rectangle assumes the puzzle has a single solution. Disabled
    /// by default because user-supplied or fuzz-generated puzzles may
    /// violate uniqueness and silently corrupt the solver.
    pub unique_rectangle: bool,
}

impl Default for DefaultSolverConfig {
    fn default() -> Self {
        Self {
            hidden_singles: true,
            naked_twins: true,
            hidden_twins: true,
            naked_triples: true,
            hidden_triples: true,
            // Quads scan cost outweighs the savings on the puzzles in the
            // benchmark suite. Opt-in.
            naked_quads: false,
            hidden_quads: false,
            h_pattern: true,
            skyscraper: true,
            xwings: true,
            xy_wing: true,
            w_wing: true,
            // Opt-in: requires the puzzle to have exactly one solution.
            unique_rectangle: false,
        }
    }
}

impl DefaultSolver {
    pub fn new<G: AsRef<CellGroups>>(groups: G) -> Self {
        Self::new_with(groups, &DefaultSolverConfig::default())
    }

    pub fn new_with<G: AsRef<CellGroups>>(groups: G, config: &DefaultSolverConfig) -> Self {
        let strategies: Vec<Box<dyn Strategy>> = vec![
            NakedSingles::new_box(),
            HiddenSingles::new_box(config.hidden_singles),
            NakedTwins::new_box(config.naked_twins),
            HiddenTwins::new_box(config.hidden_twins),
            NakedTriples::new_box(config.naked_triples),
            HiddenTriples::new_box(config.hidden_triples),
            NakedQuads::new_box(config.naked_quads),
            HiddenQuads::new_box(config.hidden_quads),
            HPattern::new_box(config.h_pattern),
            Skyscraper::new_box(config.skyscraper),
            XWing::new_box(config.xwings),
            XYWing::new_box(config.xy_wing),
            WWing::new_box(config.w_wing),
            UniqueRectangle::new_box(config.unique_rectangle),
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

        let mut stack = StateStack::new_with(last_seen_state.clone());
        'stack: while let Some(StateStackEntry {
            branch_id: fork_id,
            state,
        }) = stack.pop()
        {
            last_seen_state = state.clone();

            debug!(
                "Processing state {id} (queue depth: {depth}/{max_depth}, num forks: {num_forks}) ...",
                id = fork_id,
                depth = stack.len(),
                max_depth = stack.max_depth(),
                num_forks = stack.num_forks()
            );
            self.print_state(&state);

            if state.is_solved(&self.groups) {
                debug!("Branch {id} is solved", id = fork_id);
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
                debug!("Applying strategies solved branch {id}", id = fork_id);
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

            trace!("Enqueueing modified base branch before fork");
            stack.push(state.clone());

            // Emplace the forked version after that so that we try with fewer
            // candidates next. If it is inconsistent, skip it.
            if forked.is_consistent(&self.groups) {
                trace!("Enqueueing forked branch");
                stack.push(forked);
            } else {
                debug!("Forked state is inconsistent - ignoring.");
            }
        }

        Err(Unsolvable(last_seen_state))
    }

    /// Counts distinct solutions up to `limit`, then stops.
    ///
    /// Passing `limit = 2` is sufficient to distinguish zero / one / many,
    /// which is what uniqueness checking needs during puzzle generation.
    ///
    /// # Warning
    ///
    /// Do not enable [`DefaultSolverConfig::unique_rectangle`] when using this
    /// method. The `UniqueRectangle` strategy assumes the puzzle has exactly one
    /// solution and will eliminate candidates that belong to valid alternative
    /// solutions, producing an incorrect count. It is disabled by default.
    pub fn count_solutions<S: AsRef<GameState>>(&self, state: S, limit: usize) -> usize {
        self.count_solutions_impl(state.as_ref(), limit, None)
    }

    /// Like [`count_solutions`] but guides branching toward `hint` (the known
    /// solution) at each fork. The first DFS path follows the hint, finding the
    /// known solution in O(cells) forks. Alternative branches are then explored
    /// to confirm (or deny) uniqueness; constraint propagation prunes them fast.
    ///
    /// # Warning
    ///
    /// Same caveat as [`count_solutions`]: do not enable
    /// [`DefaultSolverConfig::unique_rectangle`]. The `UniqueRectangle`
    /// strategy assumes the puzzle has exactly one solution and will eliminate
    /// candidates from valid alternative branches, producing an incorrect
    /// count. The strategy is disabled by default.
    pub fn count_solutions_with_hint<S: AsRef<GameState>>(
        &self,
        state: S,
        limit: usize,
        hint: &GameState,
    ) -> usize {
        self.count_solutions_impl(state.as_ref(), limit, Some(hint))
    }

    /// Returns `true` if `state` has exactly one solution.
    pub fn is_unique<S: AsRef<GameState>>(&self, state: S) -> bool {
        self.count_solutions(state, 2) == 1
    }

    /// Like [`is_unique`] but uses solution-guided branching for faster uniqueness
    /// checking. `hint` must be a fully solved grid consistent with `state`.
    ///
    /// # Warning
    ///
    /// Same caveat as [`count_solutions`]: do not enable
    /// [`DefaultSolverConfig::unique_rectangle`]. The strategy can falsely
    /// rule out a second solution and make a non-unique puzzle look unique.
    pub fn is_unique_with_hint<S: AsRef<GameState>>(&self, state: S, hint: &GameState) -> bool {
        self.count_solutions_with_hint(state, 2, hint) == 1
    }

    fn count_solutions_impl(
        &self,
        state: &GameState,
        limit: usize,
        hint: Option<&GameState>,
    ) -> usize {
        if limit == 0 {
            return 0;
        }

        let mut count = 0usize;
        let mut stack = StateStack::new_with(state.clone());

        'stack: while let Some(StateStackEntry { state, .. }) = stack.pop() {
            if !state.is_consistent(&self.groups) {
                continue;
            }

            if state.is_solved(&self.groups) {
                count += 1;
                if count >= limit {
                    break;
                }
                continue;
            }

            if self.apply_strategies(&state).is_err() {
                continue 'stack;
            }

            if state.is_solved(&self.groups) {
                count += 1;
                if count >= limit {
                    break;
                }
                continue;
            }

            let fork_index = match self.pick_index_to_fork_from(&state) {
                Some(index) => index,
                None => continue 'stack,
            };

            let fork_cell = state.get_at_index(fork_index);

            // When a hint is provided, prefer its value at the fork index so
            // the first DFS branch follows the known solution.
            let fork_value = hint
                .and_then(|h| h.get_at_index(fork_index).iter_candidates().next())
                .filter(|&v| fork_cell.contains(v))
                .unwrap_or_else(|| fork_cell.iter_candidates().next().unwrap());

            let forked = state.clone();
            forked.place_and_propagate_at_index(fork_index, fork_value, &self.groups);
            state.forget_at_index(fork_index, fork_value);
            stack.push(state);

            if forked.is_consistent(&self.groups) {
                stack.push(forked);
            }
        }

        count
    }

    /// Applies one solving step and reports the strategy responsible for it.
    pub fn solve_step<S: AsRef<GameState>>(
        &self,
        state: S,
    ) -> Result<Option<SolverStep>, InvalidGameState> {
        let state = state.as_ref().clone();
        if state.is_solved(&self.groups) {
            return Ok(None);
        }

        let stats = BoardStatsCache::new(&state);
        let mut last_candidate_step = None;

        'solving: loop {
            for strategy in self.strategies.iter().filter(|&s| s.is_enabled()) {
                let before = state.clone();
                let outcome = strategy.apply(&state, &self.groups, &stats)?;
                if matches!(outcome, StrategyResult::AppliedChange) {
                    stats.invalidate();
                    let (index, value, placed_cells, eliminated_candidates) =
                        describe_state_change(&before, &state);
                    let step = SolverStep {
                        solved: state.is_solved(&self.groups),
                        state: state.clone(),
                        strategy: format!("{strategy:?}"),
                        index,
                        value,
                        placed_cells,
                        eliminated_candidates,
                    };

                    if let (Some(index), Some(value)) = (index, value) {
                        let single_step_state = before.clone();
                        single_step_state.place_and_propagate_at_index(index, value, &self.groups);
                        return Ok(Some(SolverStep {
                            solved: single_step_state.is_solved(&self.groups),
                            state: single_step_state,
                            strategy: step.strategy,
                            index: Some(index),
                            value: Some(value),
                            placed_cells: 1,
                            eliminated_candidates,
                        }));
                    }

                    last_candidate_step = Some(step);
                    continue 'solving;
                }
            }
            break;
        }

        if let Some(step) = last_candidate_step {
            return Ok(Some(step));
        }

        if !state.is_consistent(&self.groups) {
            return Err(InvalidGameState {});
        }

        let Some(index) = self.pick_index_to_fork_from(&state) else {
            return Ok(None);
        };
        let Some(value) = state.get_at_index(index).iter_candidates().next() else {
            return Ok(None);
        };
        state.place_and_propagate_at_index(index, value, &self.groups);
        Ok(Some(SolverStep {
            solved: state.is_solved(&self.groups),
            state,
            strategy: "Guess".to_string(),
            index: Some(index),
            value: Some(value),
            placed_cells: 1,
            eliminated_candidates: 0,
        }))
    }

    /// Applies different strategies for solving the board without branching.
    fn apply_strategies(&self, state: &GameState) -> Result<(), InvalidGameState> {
        // Lazy cache for board-wide stats. The first heavy strategy that
        // needs them triggers the build; cheap strategies (NakedSingles,
        // HiddenSingles, the Hidden/Naked subset families) never touch it,
        // so simple puzzles pay no overhead at all. We invalidate the cache
        // whenever a strategy mutates state.
        let stats = BoardStatsCache::new(state);

        'solving: loop {
            'next_strategy: for strategy in self.strategies.iter().filter(|&s| s.is_enabled()) {
                match strategy.apply(state, &self.groups, &stats) {
                    Err(e) => return Err(e),
                    Ok(outcome) => {
                        #[cfg(debug_assertions)]
                        {
                            if !state.is_consistent(&self.groups) {
                                debug!(
                                    "{strategy:?} resulted in inconsistent state",
                                    strategy = strategy
                                );
                                return Err(InvalidGameState {});
                            }
                        }

                        // Some strategies (NakedSingles) keep walking the
                        // pipeline even after mutating state. Drop any cached
                        // stats so the next consumer rebuilds them.
                        if strategy.always_continue() {
                            if matches!(outcome, StrategyResult::AppliedChange) {
                                stats.invalidate();
                            }
                            continue 'next_strategy;
                        }

                        // Assuming that strategies are ordered by complexity,
                        // restarting with the easiest one should result in
                        // fastest gains. Because of that, when changes were applied
                        // we start over until all strategies report no change.
                        match outcome {
                            StrategyResult::AppliedChange => {
                                stats.invalidate();
                                continue 'solving;
                            }
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
            Err(InvalidGameState {})
        }
    }

    fn pick_index_to_fork_from(&self, state: &GameState) -> Option<Index> {
        Index::range()
            .filter_map(|index| {
                let len = state.get_at_index(index).len();
                if len > 1 {
                    Some((index, len))
                } else {
                    None
                }
            })
            .min_by_key(|&(_, len)| len)
            .map(|(index, _)| index)
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

fn describe_state_change(
    before: &GameState,
    after: &GameState,
) -> (Option<Index>, Option<Value>, usize, usize) {
    let mut first_placed = None;
    let mut placed_cells = 0usize;
    let mut eliminated_candidates = 0usize;

    for index in Index::range() {
        let before_cell = before.get_at_index(index);
        let after_cell = after.get_at_index(index);
        if before_cell == after_cell {
            continue;
        }

        if !before_cell.is_solved() && after_cell.is_solved() {
            placed_cells += 1;
            if first_placed.is_none() {
                first_placed = after_cell
                    .iter_candidates()
                    .next()
                    .map(|value| (index, value));
            }
        }

        for value in before_cell.iter_candidates() {
            if !after_cell.contains(value) {
                eliminated_candidates += 1;
            }
        }
    }

    match first_placed {
        Some((index, value)) => (
            Some(index),
            Some(value),
            placed_cells,
            eliminated_candidates,
        ),
        None => (None, None, placed_cells, eliminated_candidates),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solve_step_returns_single_visible_placement() {
        let game = crate::example_games::sudoku::example_sudoku();
        let solver = DefaultSolver::new(&game);
        let step = solver
            .solve_step(&game.initial_state)
            .expect("step should be valid")
            .expect("step should be available");

        let added_values = Index::range()
            .filter(|&index| {
                !game.initial_state.get_at_index(index).is_solved()
                    && step.state.get_at_index(index).is_solved()
            })
            .count();
        assert_eq!(added_values, 1);
        assert_eq!(step.placed_cells, 1);
    }

    fn no_logic_config() -> DefaultSolverConfig {
        DefaultSolverConfig {
            hidden_singles: false,
            naked_twins: false,
            hidden_twins: false,
            naked_triples: false,
            hidden_triples: false,
            naked_quads: false,
            hidden_quads: false,
            h_pattern: false,
            skyscraper: false,
            xwings: false,
            xy_wing: false,
            w_wing: false,
            unique_rectangle: false,
        }
    }

    #[test]
    fn solve_step_reports_candidate_only_change() {
        let groups = CellGroups::default().with_default_rows_and_columns();
        let state = GameState::new();
        state.set_at_index(Index::new(0), Value::ONE);
        let solver = DefaultSolver::new_with(&groups, &no_logic_config());

        let step = solver
            .solve_step(&state)
            .expect("candidate-only step should be valid")
            .expect("candidate-only step should be reported");

        assert_eq!(step.strategy, "Naked singles");
        assert_eq!(step.index, None);
        assert_eq!(step.value, None);
        assert_eq!(step.placed_cells, 0);
        assert!(step.eliminated_candidates > 0);
    }

    #[test]
    fn solve_step_guesses_when_no_strategy_can_advance() {
        let groups = CellGroups::default().with_default_rows_and_columns();
        let state = GameState::new();
        let solver = DefaultSolver::new_with(&groups, &no_logic_config());

        let step = solver
            .solve_step(&state)
            .expect("guess step should be valid")
            .expect("guess step should be available");

        assert_eq!(step.strategy, "Guess");
        assert_eq!(step.index, Some(Index::new(0)));
        assert!(step.value.is_some());
        assert_eq!(step.placed_cells, 1);
    }

    #[test]
    fn solve_step_rejects_impossible_state() {
        let groups = CellGroups::default().with_default_rows_and_columns();
        let state = GameState::new();
        for value in Value::range() {
            state.forget_at_index(Index::new(0), value);
        }
        let solver = DefaultSolver::new_with(&groups, &no_logic_config());

        assert!(solver.solve_step(&state).is_err());
    }

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
    fn solving_sudoku_with_naked_xwings() {
        let game = crate::example_games::sudoku_xwings::example_sudoku();
        let solver = DefaultSolver::new(&game);
        let result = solver.solve(&game);
        assert!(result.is_ok());

        let solution = result.unwrap();
        assert!(solution.is_consistent(&game.groups));
        assert!(solution.is_solved(&game.groups));
    }

    #[test]
    fn solving_sudoku_with_xy_wing() {
        let game = crate::example_games::sudoku_xy_wing::example_sudoku();
        let solver = DefaultSolver::new(&game);
        let result = solver.solve(&game.initial_state);
        assert!(result.is_ok());

        let solution = result.unwrap();
        assert!(solution.is_consistent(&game.groups));
        assert!(solution.is_solved(&game.groups));
    }

    #[test]
    fn solving_sudoku_with_w_wing() {
        let game = crate::example_games::sudoku_w_wing::example_sudoku();
        let solver = DefaultSolver::new(&game);
        let result = solver.solve(&game);
        assert!(result.is_ok());

        let solution = result.unwrap();
        assert!(solution.is_consistent(&game.groups));
        assert!(solution.is_solved(&game.groups));
    }

    #[test]
    fn solving_sudoku_with_skyscraper() {
        let game = crate::example_games::sudoku_skyscraper::example_sudoku();
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

    #[test]
    fn solving_hardest() {
        let game = crate::example_games::sudoku2::example_sudoku_hardest();
        let solver = DefaultSolver::new(&game);
        let result = solver.solve(&game);
        assert!(result.is_ok());

        let solution = result.unwrap();
        assert!(solution.is_consistent(&game.groups));
        assert!(solution.is_solved(&game.groups));
    }

    #[test]
    fn count_solutions_returns_one_for_unique_puzzle() {
        let game = crate::example_games::sudoku::example_sudoku();
        let solver = DefaultSolver::new(&game);
        assert_eq!(solver.count_solutions(&game.initial_state, 2), 1);
    }

    #[test]
    fn count_solutions_returns_zero_for_no_solution() {
        let game = crate::example_games::sudoku::example_sudoku();
        let solver = DefaultSolver::new(&game);

        // Two cells in the same row both solved to ONE → immediately inconsistent.
        #[rustfmt::skip]
        let state = GameState::new_from([
            1u8, 1, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,

              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,

              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
        ]);
        assert_eq!(solver.count_solutions(&state, 2), 0);
    }

    #[test]
    fn count_solutions_returns_at_least_two_for_near_empty_board() {
        let game = crate::example_games::sudoku::example_sudoku();
        let solver = DefaultSolver::new(&game);

        // Only one given value → vast number of solutions.
        #[rustfmt::skip]
        let state = GameState::new_from([
            1u8, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,

              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,

              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
        ]);
        assert_eq!(solver.count_solutions(&state, 2), 2);
    }

    #[test]
    fn count_solutions_stops_at_limit() {
        let game = crate::example_games::sudoku::example_sudoku();
        let solver = DefaultSolver::new(&game);

        #[rustfmt::skip]
        let state = GameState::new_from([
            1u8, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,

              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,

              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
        ]);
        // limit=0 always returns 0 without touching the stack
        assert_eq!(solver.count_solutions(&state, 0), 0);
        // limit=1 stops after the first solution
        assert_eq!(solver.count_solutions(&state, 1), 1);
        // limit=3 stops after 3 solutions
        assert_eq!(solver.count_solutions(&state, 3), 3);
    }

    #[test]
    fn is_unique_returns_true_for_unique_puzzle() {
        let game = crate::example_games::sudoku::example_sudoku();
        let solver = DefaultSolver::new(&game);
        assert!(solver.is_unique(&game.initial_state));
    }

    #[test]
    fn is_unique_returns_false_for_near_empty_board() {
        let game = crate::example_games::sudoku::example_sudoku();
        let solver = DefaultSolver::new(&game);

        #[rustfmt::skip]
        let state = GameState::new_from([
            1u8, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,

              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,

              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
        ]);
        assert!(!solver.is_unique(&state));
    }

    #[test]
    fn count_solutions_hypersudoku_is_unique() {
        let game = crate::example_games::hypersudoku::example_hypersudoku();
        let solver = DefaultSolver::new(&game);
        assert_eq!(solver.count_solutions(&game.initial_state, 2), 1);
    }

    #[test]
    fn count_solutions_nonomino_is_unique() {
        let game = crate::example_games::nonomino::example_nonomino();
        let solver = DefaultSolver::new(&game);
        assert_eq!(solver.count_solutions(&game.initial_state, 2), 1);
    }

    #[test]
    fn count_solutions_with_hint_matches_count_solutions_for_unique_puzzle() {
        let game = crate::example_games::sudoku::example_sudoku();
        let solver = DefaultSolver::new(&game);
        let solution = solver.solve(&game.initial_state).expect("solvable");
        assert_eq!(
            solver.count_solutions_with_hint(&game.initial_state, 2, &solution),
            solver.count_solutions(&game.initial_state, 2)
        );
        assert_eq!(
            solver.count_solutions_with_hint(&game.initial_state, 2, &solution),
            1
        );
    }

    #[test]
    fn count_solutions_with_hint_matches_count_solutions_for_near_empty_board() {
        let game = crate::example_games::sudoku::example_sudoku();
        let solver = DefaultSolver::new(&game);

        #[rustfmt::skip]
        let state = GameState::new_from([
            1u8, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,

              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,

              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
              0, 0, 0,  0, 0, 0,  0, 0, 0,
        ]);
        let some_solution = solver.solve(&state).expect("near-empty is solvable");
        // With limit=2 both paths must observe at least 2 solutions.
        assert_eq!(
            solver.count_solutions_with_hint(&state, 2, &some_solution),
            solver.count_solutions(&state, 2)
        );
        assert_eq!(
            solver.count_solutions_with_hint(&state, 2, &some_solution),
            2
        );
    }

    #[test]
    fn is_unique_with_hint_falls_back_when_hint_inconsistent_with_state() {
        // Use one puzzle's clues but another puzzle's solution as the hint.
        // The hint values for clue cells will disagree, so the hint's value
        // for a forked cell will not appear in the candidate set; the impl
        // must fall back to the first candidate and still report uniqueness.
        let game = crate::example_games::sudoku::example_sudoku();
        let solver = DefaultSolver::new(&game);
        let real_solution = solver.solve(&game.initial_state).expect("solvable");

        // Construct a "fake" hint by rotating digits in the real solution. The
        // result is no longer the puzzle's solution, but the hint API must
        // still return the correct unique count for the original puzzle.
        let mut values = [0u8; 81];
        for (i, idx) in crate::index::Index::range().enumerate() {
            let cell = real_solution.get_at_index(idx);
            let v: u8 = cell.iter_candidates().next().unwrap().into();
            values[i] = (v % 9) + 1;
        }
        let fake_hint = GameState::new_from(values);
        assert!(solver.is_unique_with_hint(&game.initial_state, &fake_hint));
        assert_eq!(
            solver.count_solutions_with_hint(&game.initial_state, 2, &fake_hint),
            1
        );
    }
}
