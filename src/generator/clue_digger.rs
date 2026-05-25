use crate::cell_group::CellGroups;
use crate::default_solver::{DefaultSolver, DefaultSolverConfig};
use crate::difficulty_estimator::{estimate_difficulty, Difficulty};
use crate::value::Value;
use crate::GameState;
use rand::seq::SliceRandom;
use rand::Rng;

/// Strategy for choosing which cells to remove during clue digging.
pub enum RemovalStrategy {
    /// Remove cells in a uniformly random order.
    Random,
    /// Remove cells in symmetric pairs (180-degree rotational symmetry).
    ///
    /// For each pair `(i, 80-i)`, both cells are removed together or neither is.
    /// The center cell (index 40) is treated as its own pair.
    /// Resulting puzzles have the aesthetically standard clue symmetry used
    /// by most published Sudoku.
    Symmetric,
}

/// When to stop removing clues.
pub enum StoppingCondition {
    /// Try every cell; produce a minimal puzzle (no further removals possible).
    Minimal,
    /// Stop once the clue count drops to at or below the given target.
    ClueCount(usize),
}

/// Removes clues from a complete solution grid while preserving a unique solution.
///
/// Wraps a [`DefaultSolver`] to check uniqueness after each attempted removal.
/// Cells that would break uniqueness are kept as fixed clues.
///
/// When a `target_difficulty` is set via [`ClueDigger::with_target_difficulty`],
/// removals that push the puzzle above that difficulty are also reverted. This
/// prevents over-digging and reliably produces puzzles at the target level.
pub struct ClueDigger {
    solver: DefaultSolver,
    groups: CellGroups,
    target_difficulty: Option<Difficulty>,
}

/// Progress emitted while removing clues from a completed grid.
pub struct ClueDiggingProgress {
    pub processed_steps: usize,
    pub total_steps: usize,
    pub remaining_clues: usize,
}

impl ClueDigger {
    pub fn new(groups: &CellGroups) -> Self {
        // Advanced strategies (HiddenSingles, etc.) assume the state is
        // consistent and can panic via debug_assert when applied during
        // backtracking over states with multiple solutions. NakedSingles only
        // removes candidates and is always safe.
        let config = DefaultSolverConfig {
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
        };
        Self {
            solver: DefaultSolver::new_with(groups, &config),
            groups: groups.clone(),
            target_difficulty: None,
        }
    }

    /// Stop removing clues when further removal would push difficulty above `target`.
    /// The resulting puzzle will have difficulty at most `target`.
    ///
    /// `Difficulty::Extreme` is treated as "no cap" because no estimated value
    /// can exceed it, so the check is skipped entirely to avoid an unnecessary
    /// `estimate_difficulty` call after every successful removal.
    pub fn with_target_difficulty(mut self, target: Difficulty) -> Self {
        self.target_difficulty = if target == Difficulty::Extreme {
            None
        } else {
            Some(target)
        };
        self
    }

    /// Digs clues from `solution`, returning a puzzle [`GameState`] (partial grid).
    ///
    /// The returned state contains only the surviving clue cells; all other cells
    /// are open (full candidate set). The solution must be a fully solved grid.
    pub fn dig<R: Rng>(
        &self,
        solution: &GameState,
        strategy: RemovalStrategy,
        stop: StoppingCondition,
        rng: &mut R,
    ) -> GameState {
        self.dig_with_callback(solution, strategy, stop, rng, |_| {})
    }

    pub fn dig_with_callback<R, F>(
        &self,
        solution: &GameState,
        strategy: RemovalStrategy,
        stop: StoppingCondition,
        rng: &mut R,
        mut on_progress: F,
    ) -> GameState
    where
        R: Rng,
        F: FnMut(ClueDiggingProgress),
    {
        match self.try_dig_with_callback(solution, strategy, stop, rng, |progress| {
            on_progress(progress);
            Ok::<(), std::convert::Infallible>(())
        }) {
            Ok(state) => state,
            Err(err) => match err {},
        }
    }

    pub fn try_dig_with_callback<R, F, E>(
        &self,
        solution: &GameState,
        strategy: RemovalStrategy,
        stop: StoppingCondition,
        rng: &mut R,
        mut on_progress: F,
    ) -> Result<GameState, E>
    where
        R: Rng,
        F: FnMut(ClueDiggingProgress) -> Result<(), E>,
    {
        debug_assert!(
            solution.iter_indexed().all(|c| c.is_solved()),
            "solution must be a fully solved grid"
        );
        let mut clues: [Option<Value>; 81] = [None; 81];
        for cell in solution.iter_indexed() {
            if cell.is_solved() {
                clues[*cell.index as usize] = cell.iter_candidates().next();
            }
        }

        match strategy {
            RemovalStrategy::Random => {
                self.dig_random(&mut clues, &stop, rng, solution, &mut on_progress)?;
            }
            RemovalStrategy::Symmetric => {
                self.dig_symmetric(&mut clues, &stop, rng, solution, &mut on_progress)?;
            }
        }

        Ok(Self::state_from_clues(&clues))
    }

    fn dig_random<R: Rng, F, E>(
        &self,
        clues: &mut [Option<Value>; 81],
        stop: &StoppingCondition,
        rng: &mut R,
        solution: &GameState,
        on_progress: &mut F,
    ) -> Result<(), E>
    where
        F: FnMut(ClueDiggingProgress) -> Result<(), E>,
    {
        let mut order: Vec<usize> = (0..81).collect();
        order.shuffle(rng);

        let total_steps = order.len();
        for (processed, idx) in order.into_iter().enumerate() {
            if clues[idx].is_some() && !self.should_stop(clues, stop) {
                let saved = clues[idx].take();
                let state = Self::state_from_clues(clues);
                if !self.solver.is_unique_with_hint(&state, solution) {
                    clues[idx] = saved;
                } else if let Some(target) = self.target_difficulty {
                    if estimate_difficulty(&state, &self.groups) > target {
                        clues[idx] = saved;
                    }
                }
            }
            on_progress(ClueDiggingProgress {
                processed_steps: processed + 1,
                total_steps,
                remaining_clues: clue_count(clues),
            })?;
            if self.should_stop(clues, stop) {
                break;
            }
        }
        Ok(())
    }

    fn dig_symmetric<R: Rng, F, E>(
        &self,
        clues: &mut [Option<Value>; 81],
        stop: &StoppingCondition,
        rng: &mut R,
        solution: &GameState,
        on_progress: &mut F,
    ) -> Result<(), E>
    where
        F: FnMut(ClueDiggingProgress) -> Result<(), E>,
    {
        // Build pairs (i, 80-i). Index 40 maps to itself (center cell).
        let mut pairs: Vec<(usize, usize)> = (0..40).map(|i| (i, 80 - i)).collect();
        pairs.push((40, 40));
        pairs.shuffle(rng);

        let total_steps = pairs.len();
        for (processed, (a, b)) in pairs.into_iter().enumerate() {
            if (clues[a].is_some() || clues[b].is_some()) && !self.should_stop(clues, stop) {
                let saved_a = clues[a].take();
                let saved_b = if a != b { clues[b].take() } else { None };

                let state = Self::state_from_clues(clues);
                if !self.solver.is_unique_with_hint(&state, solution) {
                    clues[a] = saved_a;
                    if a != b {
                        clues[b] = saved_b;
                    }
                } else if let Some(target) = self.target_difficulty {
                    if estimate_difficulty(&state, &self.groups) > target {
                        clues[a] = saved_a;
                        if a != b {
                            clues[b] = saved_b;
                        }
                    }
                }
            }
            on_progress(ClueDiggingProgress {
                processed_steps: processed + 1,
                total_steps,
                remaining_clues: clue_count(clues),
            })?;
            if self.should_stop(clues, stop) {
                break;
            }
        }
        Ok(())
    }

    fn should_stop(&self, clues: &[Option<Value>; 81], stop: &StoppingCondition) -> bool {
        match stop {
            StoppingCondition::Minimal => false,
            StoppingCondition::ClueCount(n) => clue_count(clues) <= *n,
        }
    }

    fn state_from_clues(clues: &[Option<Value>; 81]) -> GameState {
        let mut values = [0u8; 81];
        for (i, v) in clues.iter().enumerate() {
            if let Some(val) = v {
                values[i] = u8::from(*val);
            }
        }
        GameState::new_from(values)
    }
}

fn clue_count(clues: &[Option<Value>; 81]) -> usize {
    clues.iter().filter(|c| c.is_some()).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell_group::CellGroups;
    use crate::generator::GridGenerator;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn standard_groups() -> CellGroups {
        CellGroups::default()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns()
    }

    fn make_solution(rng: &mut impl Rng) -> GameState {
        GridGenerator::new(standard_groups()).generate(rng)
    }

    #[test]
    fn random_produces_unique_puzzle() {
        let mut rng = StdRng::seed_from_u64(42);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(&standard_groups());
        let puzzle = digger.dig(
            &solution,
            RemovalStrategy::Random,
            StoppingCondition::Minimal,
            &mut rng,
        );

        let solver = DefaultSolver::new(&standard_groups());
        assert!(solver.is_unique(&puzzle));
    }

    #[test]
    fn random_dig_reports_progress() {
        let mut rng = StdRng::seed_from_u64(700);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(&standard_groups());
        let mut progress = Vec::new();
        let puzzle = digger.dig_with_callback(
            &solution,
            RemovalStrategy::Random,
            StoppingCondition::ClueCount(80),
            &mut rng,
            |event| {
                progress.push((
                    event.processed_steps,
                    event.total_steps,
                    event.remaining_clues,
                ))
            },
        );

        assert!(!progress.is_empty());
        assert_eq!(progress[0].0, 1);
        assert_eq!(progress[0].1, 81);
        assert!(progress[0].2 <= 80);
        assert!(puzzle.iter_indexed().filter(|c| c.is_solved()).count() <= 80);
    }

    #[test]
    fn symmetric_dig_reports_pair_progress() {
        let mut rng = StdRng::seed_from_u64(701);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(&standard_groups());
        let mut last = None;
        let puzzle = digger.dig_with_callback(
            &solution,
            RemovalStrategy::Symmetric,
            StoppingCondition::ClueCount(80),
            &mut rng,
            |event| {
                last = Some((
                    event.processed_steps,
                    event.total_steps,
                    event.remaining_clues,
                ))
            },
        );

        let (processed, total, remaining) = last.expect("progress should be reported");
        assert_eq!(processed, 1);
        assert_eq!(total, 41);
        assert!(remaining <= 80);
        assert!(puzzle.iter_indexed().filter(|c| c.is_solved()).count() <= 80);
    }

    #[test]
    fn random_clue_count_stop() {
        let mut rng = StdRng::seed_from_u64(7);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(&standard_groups());
        let puzzle = digger.dig(
            &solution,
            RemovalStrategy::Random,
            StoppingCondition::ClueCount(25),
            &mut rng,
        );

        let count = puzzle.iter_indexed().filter(|c| c.is_solved()).count();
        assert!(
            count <= 25,
            "clue count {count} exceeds ClueCount(25) target"
        );
    }

    #[test]
    fn symmetric_produces_unique_puzzle() {
        let mut rng = StdRng::seed_from_u64(123);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(&standard_groups());
        let puzzle = digger.dig(
            &solution,
            RemovalStrategy::Symmetric,
            StoppingCondition::Minimal,
            &mut rng,
        );

        let solver = DefaultSolver::new(&standard_groups());
        assert!(solver.is_unique(&puzzle));
    }

    #[test]
    fn symmetric_produces_180_degree_symmetry() {
        let mut rng = StdRng::seed_from_u64(99);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(&standard_groups());
        let puzzle = digger.dig(
            &solution,
            RemovalStrategy::Symmetric,
            StoppingCondition::Minimal,
            &mut rng,
        );

        let cells: Vec<bool> = puzzle.iter_indexed().map(|c| c.is_solved()).collect();
        for i in 0..81 {
            assert_eq!(
                cells[i],
                cells[80 - i],
                "symmetry broken at index {i}: cell {i} solved={}, cell {} solved={}",
                cells[i],
                80 - i,
                cells[80 - i]
            );
        }
    }

    #[test]
    fn random_with_target_difficulty_caps_estimated_difficulty() {
        let mut rng = StdRng::seed_from_u64(2026);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(&standard_groups()).with_target_difficulty(Difficulty::Easy);
        let puzzle = digger.dig(
            &solution,
            RemovalStrategy::Random,
            StoppingCondition::Minimal,
            &mut rng,
        );
        let difficulty = estimate_difficulty(&puzzle, &standard_groups());
        assert!(
            difficulty <= Difficulty::Easy,
            "random dig produced {difficulty:?}, expected <= Easy"
        );
    }

    #[test]
    fn symmetric_with_target_difficulty_caps_estimated_difficulty() {
        let mut rng = StdRng::seed_from_u64(2027);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(&standard_groups()).with_target_difficulty(Difficulty::Medium);
        let puzzle = digger.dig(
            &solution,
            RemovalStrategy::Symmetric,
            StoppingCondition::Minimal,
            &mut rng,
        );
        let difficulty = estimate_difficulty(&puzzle, &standard_groups());
        assert!(
            difficulty <= Difficulty::Medium,
            "symmetric dig produced {difficulty:?}, expected <= Medium"
        );
    }

    #[test]
    fn extreme_target_disables_difficulty_gate() {
        // Extreme is the maximum tier so the cap can never trigger; the struct
        // should store None to skip the (otherwise unnecessary) estimate call.
        let digger =
            ClueDigger::new(&standard_groups()).with_target_difficulty(Difficulty::Extreme);
        assert!(digger.target_difficulty.is_none());
    }

    #[test]
    fn minimal_puzzle_has_no_removable_clues() {
        let mut rng = StdRng::seed_from_u64(55);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(&standard_groups());
        let puzzle = digger.dig(
            &solution,
            RemovalStrategy::Random,
            StoppingCondition::Minimal,
            &mut rng,
        );

        // Verify no single clue can be removed while keeping uniqueness
        let solver = DefaultSolver::new(&standard_groups());
        let clue_indices: Vec<usize> = puzzle
            .iter_indexed()
            .filter(|c| c.is_solved())
            .map(|c| *c.index as usize)
            .collect();

        for idx in clue_indices {
            let mut values = [0u8; 81];
            for cell in puzzle.iter_indexed() {
                if cell.is_solved() && *cell.index as usize != idx {
                    values[*cell.index as usize] = u8::from(cell.iter_candidates().next().unwrap());
                }
            }
            let reduced = GameState::new_from(values);
            assert!(
                !solver.is_unique(&reduced),
                "removing clue at index {idx} still unique - not minimal"
            );
        }
    }
}
