use crate::cell_group::CellGroups;
use crate::difficulty_estimator::{estimate_difficulty, Difficulty};
use crate::game_state::GameState;
use crate::generator::{
    ClueDigger, GridGenerator, NonominoRegionGenerator, RemovalStrategy, StoppingCondition,
};
use rand::Rng;

/// Sudoku variant determining which cell groups are active.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Variant {
    /// Standard 9×9 Sudoku with rows, columns, and 3×3 blocks.
    Standard,
    /// Hypersudoku: standard groups plus four overlapping 3×3 windows.
    Hypersudoku,
    /// Nonomino (jigsaw Sudoku): rows, columns, and 9 irregular connected regions.
    Nonomino,
}

/// Clue symmetry pattern applied during clue digging.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Symmetry {
    /// Clues placed in no particular symmetric pattern.
    None,
    /// Clues symmetric under 180-degree rotation: for each clue at index `i`,
    /// there is also a clue at index `80 - i`.
    Rotational180,
}

/// Configuration for [`PuzzleGenerator`].
pub struct PuzzleGeneratorConfig {
    pub variant: Variant,
    pub target_difficulty: Difficulty,
    pub symmetry: Symmetry,
    /// Maximum number of full generation attempts before giving up.
    pub max_attempts: usize,
}

impl Default for PuzzleGeneratorConfig {
    fn default() -> Self {
        Self {
            variant: Variant::Standard,
            target_difficulty: Difficulty::Medium,
            symmetry: Symmetry::None,
            max_attempts: 100,
        }
    }
}

/// A generated Sudoku puzzle with its solution and metadata.
#[derive(Debug, Clone)]
pub struct Puzzle {
    /// The puzzle to solve (partial grid with clues only).
    pub state: GameState,
    /// The unique complete solution.
    pub solution: GameState,
    /// Cell groups for this puzzle's variant.
    pub groups: CellGroups,
    /// Difficulty tier estimated for this puzzle.
    pub difficulty: Difficulty,
}

/// Error returned when puzzle generation fails.
#[derive(Debug)]
pub enum GenerationError {
    /// All attempts exhausted without hitting the target difficulty.
    /// `closest` holds the best puzzle found (if any valid puzzle was produced).
    MaxAttemptsExceeded {
        attempts: usize,
        closest: Option<Puzzle>,
    },
}

impl std::fmt::Display for GenerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenerationError::MaxAttemptsExceeded { attempts, closest } => {
                write!(f, "puzzle generation failed after {attempts} attempts")?;
                if let Some(p) = closest {
                    write!(f, " (closest: {:?})", p.difficulty)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for GenerationError {}

/// Generates Sudoku puzzles by orchestrating grid generation, clue digging,
/// and difficulty estimation.
pub struct PuzzleGenerator {
    config: PuzzleGeneratorConfig,
}

impl PuzzleGenerator {
    pub fn new(config: PuzzleGeneratorConfig) -> Self {
        Self { config }
    }

    /// Generates a puzzle matching the configured target difficulty.
    ///
    /// Retries up to `config.max_attempts` times. On success returns a puzzle
    /// whose estimated difficulty equals the target. On failure returns
    /// [`GenerationError::MaxAttemptsExceeded`] with the closest match found.
    #[allow(clippy::result_large_err)]
    pub fn generate<R: Rng>(&self, rng: &mut R) -> Result<Puzzle, GenerationError> {
        let mut closest: Option<Puzzle> = None;

        for _attempt in 0..self.config.max_attempts {
            let groups = match self.build_groups(rng) {
                Some(g) => g,
                None => continue,
            };

            let solution = GridGenerator::new(groups.clone()).generate(rng);
            let digger = ClueDigger::new(&groups);
            let removal = match self.config.symmetry {
                Symmetry::None => RemovalStrategy::Random,
                Symmetry::Rotational180 => RemovalStrategy::Symmetric,
            };
            let state = digger.dig(&solution, removal, StoppingCondition::Minimal, rng);
            let difficulty = estimate_difficulty(&state, &groups);

            let puzzle = Puzzle {
                state,
                solution,
                groups,
                difficulty,
            };

            if difficulty == self.config.target_difficulty {
                return Ok(puzzle);
            }

            if is_closer(difficulty, self.config.target_difficulty, &closest) {
                closest = Some(puzzle);
            }
        }

        Err(GenerationError::MaxAttemptsExceeded {
            attempts: self.config.max_attempts,
            closest,
        })
    }

    fn build_groups<R: Rng>(&self, rng: &mut R) -> Option<CellGroups> {
        match self.config.variant {
            Variant::Standard => Some(
                CellGroups::default()
                    .with_default_sudoku_blocks()
                    .with_default_rows_and_columns(),
            ),
            Variant::Hypersudoku => Some(
                CellGroups::default()
                    .with_hypersudoku_windows()
                    .with_default_sudoku_blocks()
                    .with_default_rows_and_columns(),
            ),
            Variant::Nonomino => NonominoRegionGenerator::default().generate(rng),
        }
    }
}

fn is_closer(candidate: Difficulty, target: Difficulty, current_best: &Option<Puzzle>) -> bool {
    match current_best {
        None => true,
        Some(best) => {
            difficulty_distance(candidate, target) < difficulty_distance(best.difficulty, target)
        }
    }
}

fn difficulty_distance(a: Difficulty, b: Difficulty) -> usize {
    (a as usize).abs_diff(b as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_solver::DefaultSolver;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn standard_config(target: Difficulty) -> PuzzleGeneratorConfig {
        PuzzleGeneratorConfig {
            variant: Variant::Standard,
            target_difficulty: target,
            symmetry: Symmetry::None,
            max_attempts: 200,
        }
    }

    fn assert_valid_puzzle(puzzle: &Puzzle) {
        let solver = DefaultSolver::new(&puzzle.groups);
        assert!(
            solver.is_unique(&puzzle.state),
            "puzzle must have unique solution"
        );
        assert!(
            puzzle.solution.is_solved(&puzzle.groups),
            "solution must be complete"
        );
        assert!(
            puzzle.solution.is_consistent(&puzzle.groups),
            "solution must be consistent"
        );
    }

    #[test]
    fn generates_valid_easy_puzzle() {
        let mut rng = StdRng::seed_from_u64(1);
        let result = PuzzleGenerator::new(standard_config(Difficulty::Easy)).generate(&mut rng);
        let puzzle = result.expect("should generate easy puzzle");
        assert_eq!(puzzle.difficulty, Difficulty::Easy);
        assert_valid_puzzle(&puzzle);
    }

    #[test]
    fn generates_valid_medium_puzzle() {
        let mut rng = StdRng::seed_from_u64(2);
        let result = PuzzleGenerator::new(standard_config(Difficulty::Medium)).generate(&mut rng);
        let puzzle = result.expect("should generate medium puzzle");
        assert_eq!(puzzle.difficulty, Difficulty::Medium);
        assert_valid_puzzle(&puzzle);
    }

    #[test]
    fn generates_valid_hard_puzzle() {
        let mut rng = StdRng::seed_from_u64(3);
        let result = PuzzleGenerator::new(standard_config(Difficulty::Hard)).generate(&mut rng);
        let puzzle = result.expect("should generate hard puzzle");
        assert_eq!(puzzle.difficulty, Difficulty::Hard);
        assert_valid_puzzle(&puzzle);
    }

    #[test]
    fn generates_valid_expert_puzzle() {
        let mut rng = StdRng::seed_from_u64(4);
        let result = PuzzleGenerator::new(standard_config(Difficulty::Expert)).generate(&mut rng);
        let puzzle = result.expect("should generate expert puzzle");
        assert_eq!(puzzle.difficulty, Difficulty::Expert);
        assert_valid_puzzle(&puzzle);
    }

    #[test]
    fn symmetric_puzzle_has_rotational_symmetry() {
        let mut rng = StdRng::seed_from_u64(10);
        let config = PuzzleGeneratorConfig {
            variant: Variant::Standard,
            target_difficulty: Difficulty::Medium,
            symmetry: Symmetry::Rotational180,
            max_attempts: 200,
        };
        let puzzle = PuzzleGenerator::new(config)
            .generate(&mut rng)
            .expect("should succeed");
        let cells: Vec<bool> = puzzle.state.iter_indexed().map(|c| c.is_solved()).collect();
        for i in 0..81 {
            assert_eq!(cells[i], cells[80 - i], "symmetry broken at index {i}");
        }
    }

    #[test]
    #[ignore = "Hypersudoku grid generation is slow in debug mode"]
    fn hypersudoku_puzzle_is_valid() {
        let mut rng = StdRng::seed_from_u64(20);
        let config = PuzzleGeneratorConfig {
            variant: Variant::Hypersudoku,
            target_difficulty: Difficulty::Easy,
            symmetry: Symmetry::None,
            max_attempts: 10,
        };
        // Hypersudoku may not hit Easy in 10 attempts; closest is also valid.
        let puzzle = match PuzzleGenerator::new(config).generate(&mut rng) {
            Ok(p) => p,
            Err(GenerationError::MaxAttemptsExceeded {
                closest: Some(p), ..
            }) => p,
            Err(GenerationError::MaxAttemptsExceeded { closest: None, .. }) => {
                panic!("no hypersudoku puzzle produced in 10 attempts")
            }
        };
        assert_valid_puzzle(&puzzle);
    }

    #[test]
    fn max_attempts_exceeded_returns_closest() {
        let mut rng = StdRng::seed_from_u64(99);
        let config = PuzzleGeneratorConfig {
            variant: Variant::Standard,
            target_difficulty: Difficulty::Easy,
            symmetry: Symmetry::None,
            max_attempts: 1,
        };
        // With only 1 attempt, it may or may not hit Easy. If it misses, closest must be Some.
        match PuzzleGenerator::new(config).generate(&mut rng) {
            Ok(_) => {} // hit target on first try, fine
            Err(GenerationError::MaxAttemptsExceeded { attempts, closest }) => {
                assert_eq!(attempts, 1);
                assert!(
                    closest.is_some(),
                    "closest should be populated after a valid attempt"
                );
            }
        }
    }

    #[test]
    fn error_display_contains_attempts() {
        let err = GenerationError::MaxAttemptsExceeded {
            attempts: 42,
            closest: None,
        };
        assert!(err.to_string().contains("42"));
    }

    #[test]
    fn generate_10_puzzles_easy() {
        let mut hits = 0usize;
        for seed in 0..10u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let config = PuzzleGeneratorConfig {
                variant: Variant::Standard,
                target_difficulty: Difficulty::Easy,
                symmetry: Symmetry::None,
                max_attempts: 50,
            };
            if let Ok(puzzle) = PuzzleGenerator::new(config).generate(&mut rng) {
                assert_valid_puzzle(&puzzle);
                hits += 1;
            }
        }
        assert!(hits >= 8, "only {hits}/10 easy puzzles generated");
    }

    #[test]
    fn generate_10_puzzles_medium() {
        let mut hits = 0usize;
        for seed in 0..10u64 {
            let mut rng = StdRng::seed_from_u64(seed + 100);
            let config = PuzzleGeneratorConfig {
                variant: Variant::Standard,
                target_difficulty: Difficulty::Medium,
                symmetry: Symmetry::None,
                max_attempts: 50,
            };
            if let Ok(puzzle) = PuzzleGenerator::new(config).generate(&mut rng) {
                assert_valid_puzzle(&puzzle);
                hits += 1;
            }
        }
        assert!(hits >= 5, "only {hits}/10 medium puzzles generated");
    }
}
