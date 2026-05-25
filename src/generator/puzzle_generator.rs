use crate::cell_group::CellGroups;
use crate::difficulty_estimator::{estimate_difficulty, Difficulty};
use crate::game_state::GameState;
use crate::generator::{
    ClueDigger, ClueDiggingProgress, GridGenerator, NonominoRegionGenerator, RemovalStrategy,
    StoppingCondition,
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

/// Progress event emitted while generating a puzzle.
pub enum GenerationProgress<'a> {
    /// A full generation attempt is starting.
    AttemptStarted { attempt: usize, max_attempts: usize },
    /// Nonomino region generation failed for this attempt.
    RegionGenerationFailed { attempt: usize, max_attempts: usize },
    /// Groups are available for the current attempt.
    GroupsReady {
        attempt: usize,
        max_attempts: usize,
        groups: &'a CellGroups,
    },
    /// A solved grid has been generated for this attempt.
    SolutionGenerated {
        attempt: usize,
        max_attempts: usize,
        groups: &'a CellGroups,
        solution: &'a GameState,
    },
    /// Clue digging is removing givens from the solved grid.
    ClueDiggingProgress {
        attempt: usize,
        max_attempts: usize,
        processed_steps: usize,
        total_steps: usize,
        remaining_clues: usize,
    },
    /// Clue digging has produced a candidate puzzle.
    PuzzleDug {
        attempt: usize,
        max_attempts: usize,
        groups: &'a CellGroups,
        solution: &'a GameState,
        state: &'a GameState,
    },
    /// Difficulty estimation completed for this attempt.
    AttemptFinished {
        attempt: usize,
        max_attempts: usize,
        puzzle: &'a Puzzle,
        target_met: bool,
    },
    /// The closest puzzle seen so far has been updated.
    ClosestUpdated {
        attempt: usize,
        max_attempts: usize,
        puzzle: &'a Puzzle,
    },
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
        self.generate_with_callback(rng, |_| {})
    }

    /// Generates a puzzle and reports progress through `on_progress`.
    #[allow(clippy::result_large_err)]
    pub fn generate_with_callback<R, F>(
        &self,
        rng: &mut R,
        mut on_progress: F,
    ) -> Result<Puzzle, GenerationError>
    where
        R: Rng,
        F: FnMut(GenerationProgress<'_>),
    {
        let mut closest: Option<Puzzle> = None;

        for attempt_index in 0..self.config.max_attempts {
            let attempt = attempt_index + 1;
            on_progress(GenerationProgress::AttemptStarted {
                attempt,
                max_attempts: self.config.max_attempts,
            });

            let (groups, solution) = match self.build_groups_and_solution(rng) {
                Some(base) => base,
                None => {
                    on_progress(GenerationProgress::RegionGenerationFailed {
                        attempt,
                        max_attempts: self.config.max_attempts,
                    });
                    continue;
                }
            };
            on_progress(GenerationProgress::GroupsReady {
                attempt,
                max_attempts: self.config.max_attempts,
                groups: &groups,
            });
            on_progress(GenerationProgress::SolutionGenerated {
                attempt,
                max_attempts: self.config.max_attempts,
                groups: &groups,
                solution: &solution,
            });

            let digger =
                ClueDigger::new(&groups).with_target_difficulty(self.config.target_difficulty);
            let removal = match self.config.symmetry {
                Symmetry::None => RemovalStrategy::Random,
                Symmetry::Rotational180 => RemovalStrategy::Symmetric,
            };
            let state = digger.dig_with_callback(
                &solution,
                removal,
                StoppingCondition::Minimal,
                rng,
                |progress| {
                    on_progress(map_clue_digging_progress(
                        attempt,
                        self.config.max_attempts,
                        progress,
                    ));
                },
            );
            on_progress(GenerationProgress::PuzzleDug {
                attempt,
                max_attempts: self.config.max_attempts,
                groups: &groups,
                solution: &solution,
                state: &state,
            });

            let difficulty = estimate_difficulty(&state, &groups);

            let puzzle = Puzzle {
                state,
                solution,
                groups,
                difficulty,
            };

            let target_met = difficulty == self.config.target_difficulty;
            on_progress(GenerationProgress::AttemptFinished {
                attempt,
                max_attempts: self.config.max_attempts,
                puzzle: &puzzle,
                target_met,
            });

            if target_met {
                return Ok(puzzle);
            }

            if is_closer(difficulty, self.config.target_difficulty, &closest) {
                on_progress(GenerationProgress::ClosestUpdated {
                    attempt,
                    max_attempts: self.config.max_attempts,
                    puzzle: &puzzle,
                });
                closest = Some(puzzle);
            }
        }

        Err(GenerationError::MaxAttemptsExceeded {
            attempts: self.config.max_attempts,
            closest,
        })
    }

    #[cfg(test)]
    pub(crate) fn build_groups<R: Rng>(&self, rng: &mut R) -> Option<CellGroups> {
        self.build_groups_and_solution(rng)
            .map(|(groups, _)| groups)
    }

    fn build_groups_and_solution<R: Rng>(&self, rng: &mut R) -> Option<(CellGroups, GameState)> {
        match self.config.variant {
            Variant::Standard => {
                let groups = CellGroups::default()
                    .with_default_sudoku_blocks()
                    .with_default_rows_and_columns();
                let solution = GridGenerator::new(groups.clone()).generate(rng);
                Some((groups, solution))
            }
            Variant::Hypersudoku => {
                let groups = CellGroups::default()
                    .with_hypersudoku_windows()
                    .with_default_sudoku_blocks()
                    .with_default_rows_and_columns();
                let solution = GridGenerator::new(groups.clone()).generate(rng);
                Some((groups, solution))
            }
            Variant::Nonomino => NonominoRegionGenerator::default()
                .generate_with_solution(rng)
                .map(|generated| (generated.groups, generated.solution)),
        }
    }
}

fn map_clue_digging_progress(
    attempt: usize,
    max_attempts: usize,
    progress: ClueDiggingProgress,
) -> GenerationProgress<'static> {
    GenerationProgress::ClueDiggingProgress {
        attempt,
        max_attempts,
        processed_steps: progress.processed_steps,
        total_steps: progress.total_steps,
        remaining_clues: progress.remaining_clues,
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
    fn generate_with_callback_reports_generation_phases() {
        let mut rng = StdRng::seed_from_u64(42);
        let config = PuzzleGeneratorConfig {
            variant: Variant::Standard,
            target_difficulty: Difficulty::Easy,
            symmetry: Symmetry::None,
            max_attempts: 1,
        };
        let mut events = Vec::new();
        let _ = PuzzleGenerator::new(config).generate_with_callback(&mut rng, |event| {
            let name = match event {
                GenerationProgress::AttemptStarted { .. } => "attempt_started",
                GenerationProgress::RegionGenerationFailed { .. } => "region_failed",
                GenerationProgress::GroupsReady { groups, .. } => {
                    assert!(groups.iter().count() >= 27);
                    "groups_ready"
                }
                GenerationProgress::SolutionGenerated {
                    solution, groups, ..
                } => {
                    assert!(solution.is_solved(groups));
                    "solution_generated"
                }
                GenerationProgress::ClueDiggingProgress {
                    processed_steps,
                    total_steps,
                    remaining_clues,
                    ..
                } => {
                    assert!(processed_steps <= total_steps);
                    assert!(remaining_clues <= 81);
                    "clue_digging"
                }
                GenerationProgress::PuzzleDug { state, .. } => {
                    assert!(state.iter_indexed().any(|c| !c.is_solved()));
                    "puzzle_dug"
                }
                GenerationProgress::AttemptFinished { puzzle, .. } => {
                    assert_valid_puzzle(puzzle);
                    "attempt_finished"
                }
                GenerationProgress::ClosestUpdated { puzzle, .. } => {
                    assert_valid_puzzle(puzzle);
                    "closest_updated"
                }
            };
            events.push(name);
        });

        assert!(events.contains(&"attempt_started"));
        assert!(events.contains(&"groups_ready"));
        assert!(events.contains(&"solution_generated"));
        assert!(events.contains(&"clue_digging"));
        assert!(events.contains(&"puzzle_dug"));
        assert!(events.contains(&"attempt_finished"));
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

    #[test]
    fn hypersudoku_build_groups_returns_some() {
        let mut rng = StdRng::seed_from_u64(1);
        let config = PuzzleGeneratorConfig {
            variant: Variant::Hypersudoku,
            ..Default::default()
        };
        let groups = PuzzleGenerator::new(config)
            .build_groups(&mut rng)
            .expect("Hypersudoku build_groups must always return Some");
        assert!(
            groups.iter().count() > 27,
            "Hypersudoku must have more than 27 groups"
        );
    }

    #[test]
    fn nonomino_build_groups_calls_region_generator() {
        // Verify build_groups exercises the Nonomino branch without invoking
        // the slow grid generator. We call it until we get a Some result, so
        // both the Some and None code paths in NonominoRegionGenerator::generate
        // are exercised over multiple seeds.
        let config = PuzzleGeneratorConfig {
            variant: Variant::Nonomino,
            ..Default::default()
        };
        let generator = PuzzleGenerator::new(config);
        let mut found_some = false;
        for seed in 0u64..50 {
            let mut rng = StdRng::seed_from_u64(seed);
            if generator.build_groups(&mut rng).is_some() {
                found_some = true;
                break;
            }
        }
        assert!(
            found_some,
            "Nonomino region generator must succeed for at least one seed"
        );
    }

    #[test]
    fn error_display_with_closest() {
        let mut rng = StdRng::seed_from_u64(1);
        let config = PuzzleGeneratorConfig {
            variant: Variant::Standard,
            target_difficulty: Difficulty::Medium,
            symmetry: Symmetry::None,
            max_attempts: 1,
        };
        // Run one attempt; wrap whatever we get into a MaxAttemptsExceeded with
        // a populated closest to exercise the Display branch that shows the
        // closest difficulty.
        let generator = PuzzleGenerator::new(config);
        let groups = generator
            .build_groups(&mut rng)
            .expect("Standard always builds groups");
        let solution = crate::generator::GridGenerator::new(groups.clone()).generate(&mut rng);
        let digger = crate::generator::ClueDigger::new(&groups);
        let state = digger.dig(
            &solution,
            crate::generator::RemovalStrategy::Random,
            crate::generator::StoppingCondition::Minimal,
            &mut rng,
        );
        let difficulty = crate::difficulty_estimator::estimate_difficulty(&state, &groups);
        let puzzle = Puzzle {
            state,
            solution,
            groups,
            difficulty,
        };
        let err = GenerationError::MaxAttemptsExceeded {
            attempts: 7,
            closest: Some(puzzle),
        };
        let s = err.to_string();
        assert!(s.contains("7"));
        assert!(s.contains("closest"));
    }
}
