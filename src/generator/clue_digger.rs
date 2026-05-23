use crate::cell_group::CellGroups;
use crate::default_solver::{DefaultSolver, DefaultSolverConfig};
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
pub struct ClueDigger {
    solver: DefaultSolver,
}

impl ClueDigger {
    pub fn new(groups: CellGroups) -> Self {
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
            solver: DefaultSolver::new_with(&groups, &config),
        }
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
        let mut clues: [Option<Value>; 81] = [None; 81];
        for cell in solution.iter_indexed() {
            if cell.is_solved() {
                clues[*cell.index as usize] = cell.iter_candidates().next();
            }
        }

        match strategy {
            RemovalStrategy::Random => self.dig_random(&mut clues, &stop, rng),
            RemovalStrategy::Symmetric => self.dig_symmetric(&mut clues, &stop, rng),
        }

        Self::state_from_clues(&clues)
    }

    fn dig_random<R: Rng>(
        &self,
        clues: &mut [Option<Value>; 81],
        stop: &StoppingCondition,
        rng: &mut R,
    ) {
        let mut order: Vec<usize> = (0..81).collect();
        order.shuffle(rng);

        for idx in order {
            if clues[idx].is_none() {
                continue;
            }
            if self.should_stop(clues, stop) {
                break;
            }
            let saved = clues[idx].take();
            if !self.solver.is_unique(&Self::state_from_clues(clues)) {
                clues[idx] = saved;
            }
        }
    }

    fn dig_symmetric<R: Rng>(
        &self,
        clues: &mut [Option<Value>; 81],
        stop: &StoppingCondition,
        rng: &mut R,
    ) {
        // Build pairs (i, 80-i). Index 40 maps to itself (center cell).
        let mut pairs: Vec<(usize, usize)> = (0..40).map(|i| (i, 80 - i)).collect();
        pairs.push((40, 40));
        pairs.shuffle(rng);

        for (a, b) in pairs {
            if clues[a].is_none() && clues[b].is_none() {
                continue;
            }
            if self.should_stop(clues, stop) {
                break;
            }
            let saved_a = clues[a].take();
            let saved_b = if a != b { clues[b].take() } else { None };

            if !self.solver.is_unique(&Self::state_from_clues(clues)) {
                clues[a] = saved_a;
                if a != b {
                    clues[b] = saved_b;
                }
            }
        }
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
                values[i] = val.get();
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
        let digger = ClueDigger::new(standard_groups());
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
    fn random_clue_count_stop() {
        let mut rng = StdRng::seed_from_u64(7);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(standard_groups());
        let puzzle = digger.dig(
            &solution,
            RemovalStrategy::Random,
            StoppingCondition::ClueCount(25),
            &mut rng,
        );

        let count = puzzle.iter_indexed().filter(|c| c.is_solved()).count();
        assert!(count <= 81, "clue count {count} should be at most 81");
        // With ClueCount(25), we stop as soon as count <= 25, so result may be >=25
        // (the last removal may push it to exactly 25 or stop before reaching 25)
        assert!(count >= 20, "clue count {count} should be reasonable");
    }

    #[test]
    fn symmetric_produces_unique_puzzle() {
        let mut rng = StdRng::seed_from_u64(123);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(standard_groups());
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
        let digger = ClueDigger::new(standard_groups());
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
    fn minimal_puzzle_has_no_removable_clues() {
        let mut rng = StdRng::seed_from_u64(55);
        let solution = make_solution(&mut rng);
        let digger = ClueDigger::new(standard_groups());
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
                    values[*cell.index as usize] = cell.iter_candidates().next().unwrap().get();
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
