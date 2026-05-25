use crate::cell_group::{CellGroups, WithGroupFromIterator};
use crate::default_solver::Unsolvable;
use crate::difficulty_estimator::Difficulty;
use crate::generator::{
    GenerationError, GenerationProgress, ProgressCallbackError, Puzzle, PuzzleGenerator,
    PuzzleGeneratorConfig, Symmetry, Variant,
};
use crate::{DefaultSolver, SudokuSerializer};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum WasmVariant {
    #[default]
    Standard,
    #[serde(alias = "hyper")]
    Hypersudoku,
    Nonomino,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum WasmDifficulty {
    Easy,
    #[default]
    Medium,
    Hard,
    Expert,
    Extreme,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum WasmSymmetry {
    #[default]
    None,
    #[serde(alias = "rot")]
    Rotational,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum PuzzleFormat {
    Line,
    Grid,
    #[default]
    Auto,
}

#[derive(Debug, Deserialize)]
struct SolveRequest {
    puzzle: String,
    #[serde(default)]
    variant: WasmVariant,
    #[serde(default)]
    format: PuzzleFormat,
    region_line: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SolveResponse {
    solved: bool,
    state_line: String,
    state_grid: String,
    region_line: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SolveStepResponse {
    changed: bool,
    solved: bool,
    state_line: String,
    state_grid: String,
    region_line: Option<String>,
    strategy: Option<String>,
    cell: Option<WasmCell>,
    value: Option<u8>,
    placed_cells: usize,
    eliminated_candidates: usize,
    explanation: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WasmCell {
    index: u8,
    x: u8,
    y: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenerationProgressResponse {
    event: String,
    attempt: usize,
    max_attempts: usize,
    puzzle_line: Option<String>,
    puzzle_grid: Option<String>,
    solution_line: Option<String>,
    solution_grid: Option<String>,
    region_line: Option<String>,
    difficulty: Option<String>,
    target_met: Option<bool>,
    processed_steps: Option<usize>,
    total_steps: Option<usize>,
    remaining_clues: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct GenerateRequest {
    #[serde(default)]
    variant: WasmVariant,
    #[serde(default)]
    target_difficulty: WasmDifficulty,
    #[serde(default)]
    symmetry: WasmSymmetry,
    #[serde(default = "default_max_attempts")]
    max_attempts: usize,
    seed: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenerateResponse {
    puzzle_line: String,
    puzzle_grid: String,
    solution_line: String,
    solution_grid: String,
    region_line: Option<String>,
    difficulty: String,
    target_met: bool,
    warning: Option<String>,
}

const fn default_max_attempts() -> usize {
    200
}

#[wasm_bindgen]
pub fn solve_puzzle(input: JsValue) -> Result<JsValue, JsValue> {
    let req: SolveRequest = serde_wasm_bindgen::from_value(input)
        .map_err(|e| JsValue::from_str(&format!("invalid solve request: {e}")))?;
    let response = solve_puzzle_impl(req).map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsValue::from_str(&format!("failed to serialize solve response: {e}")))
}

#[wasm_bindgen]
pub fn generate_puzzle(input: JsValue) -> Result<JsValue, JsValue> {
    let req: GenerateRequest = serde_wasm_bindgen::from_value(input)
        .map_err(|e| JsValue::from_str(&format!("invalid generation request: {e}")))?;
    let response = generate_puzzle_impl(req).map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsValue::from_str(&format!("failed to serialize generation response: {e}")))
}

#[wasm_bindgen]
pub fn generate_puzzle_with_callback(
    input: JsValue,
    callback: js_sys::Function,
) -> Result<JsValue, JsValue> {
    let req: GenerateRequest = serde_wasm_bindgen::from_value(input)
        .map_err(|e| JsValue::from_str(&format!("invalid generation request: {e}")))?;
    let response = generate_puzzle_with_callback_impl(req, |progress| {
        let value = serde_wasm_bindgen::to_value(&progress)
            .map_err(|e| format!("failed to serialize progress: {e}"))?;
        callback
            .call1(&JsValue::NULL, &value)
            .map(|_| ())
            .map_err(|e| {
                e.as_string()
                    .unwrap_or_else(|| "progress callback failed".to_string())
            })
    })
    .map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsValue::from_str(&format!("failed to serialize generation response: {e}")))
}

#[wasm_bindgen]
pub fn solve_step(input: JsValue) -> Result<JsValue, JsValue> {
    let req: SolveRequest = serde_wasm_bindgen::from_value(input)
        .map_err(|e| JsValue::from_str(&format!("invalid solve request: {e}")))?;
    let response = solve_step_impl(req).map_err(|e| JsValue::from_str(&e))?;
    serde_wasm_bindgen::to_value(&response)
        .map_err(|e| JsValue::from_str(&format!("failed to serialize solve-step response: {e}")))
}

fn solve_puzzle_impl(req: SolveRequest) -> Result<SolveResponse, String> {
    let groups = build_groups(req.variant, req.region_line.as_deref())?;
    let state = parse_state(&req.puzzle, req.format)?;
    let solver = DefaultSolver::new(&groups);

    match solver.solve(&state) {
        Ok(solution) => Ok(SolveResponse {
            solved: true,
            state_line: SudokuSerializer::format_line(&solution),
            state_grid: SudokuSerializer::format_grid(&solution),
            region_line: SudokuSerializer::format_region_line(&groups),
            error: None,
        }),
        Err(Unsolvable(last_state)) => Ok(SolveResponse {
            solved: false,
            state_line: SudokuSerializer::format_line(&last_state),
            state_grid: SudokuSerializer::format_grid(&last_state),
            region_line: SudokuSerializer::format_region_line(&groups),
            error: Some("The puzzle is unsolvable".to_string()),
        }),
    }
}

fn solve_step_impl(req: SolveRequest) -> Result<SolveStepResponse, String> {
    let groups = build_groups(req.variant, req.region_line.as_deref())?;
    let state = parse_state(&req.puzzle, req.format)?;
    let solver = DefaultSolver::new(&groups);

    match solver.solve_step(&state) {
        Ok(Some(step)) => {
            let (cell, value) = match (step.index, step.value) {
                (Some(index), Some(value)) => {
                    let coord = index.into_coordinate();
                    (
                        Some(WasmCell {
                            index: (*index),
                            x: coord.x,
                            y: coord.y,
                        }),
                        Some(value.into()),
                    )
                }
                _ => (None, None),
            };
            let explanation = match (&cell, value) {
                (Some(cell), Some(value)) => Some(format!(
                    "cell {}/{} is {} due to {}",
                    cell.x + 1,
                    cell.y + 1,
                    value,
                    step.strategy
                )),
                _ => Some(format!(
                    "{} changed {} candidate(s)",
                    step.strategy, step.eliminated_candidates
                )),
            };
            let visible_state = match (step.index, step.value) {
                (Some(index), Some(value)) => {
                    let visible = state.clone();
                    visible.set_at_index(index, value);
                    visible
                }
                _ => state.clone(),
            };
            Ok(SolveStepResponse {
                changed: true,
                solved: visible_state.is_solved(&groups),
                state_line: SudokuSerializer::format_line(&visible_state),
                state_grid: SudokuSerializer::format_grid(&visible_state),
                region_line: SudokuSerializer::format_region_line(&groups),
                strategy: Some(step.strategy),
                cell,
                value,
                placed_cells: step.placed_cells,
                eliminated_candidates: step.eliminated_candidates,
                explanation,
                error: None,
            })
        }
        Ok(None) => Ok(SolveStepResponse {
            changed: false,
            solved: state.is_solved(&groups),
            state_line: SudokuSerializer::format_line(&state),
            state_grid: SudokuSerializer::format_grid(&state),
            region_line: SudokuSerializer::format_region_line(&groups),
            strategy: None,
            cell: None,
            value: None,
            placed_cells: 0,
            eliminated_candidates: 0,
            explanation: Some("No logical step available".to_string()),
            error: None,
        }),
        Err(_) => Ok(SolveStepResponse {
            changed: false,
            solved: false,
            state_line: SudokuSerializer::format_line(&state),
            state_grid: SudokuSerializer::format_grid(&state),
            region_line: SudokuSerializer::format_region_line(&groups),
            strategy: None,
            cell: None,
            value: None,
            placed_cells: 0,
            eliminated_candidates: 0,
            explanation: None,
            error: Some("The puzzle has an invalid game state".to_string()),
        }),
    }
}

fn generate_puzzle_impl(req: GenerateRequest) -> Result<GenerateResponse, String> {
    generate_puzzle_with_callback_impl(req, |_| Ok(()))
}

fn generate_puzzle_with_callback_impl<F>(
    req: GenerateRequest,
    mut on_progress: F,
) -> Result<GenerateResponse, String>
where
    F: FnMut(GenerationProgressResponse) -> Result<(), String>,
{
    let seed = req.seed.unwrap_or_else(default_seed);
    let mut rng = StdRng::seed_from_u64(seed);
    let target_difficulty = map_difficulty(req.target_difficulty);

    let config = PuzzleGeneratorConfig {
        variant: map_variant(req.variant),
        target_difficulty,
        symmetry: map_symmetry(req.symmetry),
        max_attempts: req.max_attempts,
    };

    let mut progress_error = None;
    match PuzzleGenerator::new(config).generate_with_callback(
        &mut rng,
        |progress| match on_progress(map_generation_progress(progress)) {
            Ok(()) => Ok(()),
            Err(err) => {
                progress_error = Some(err);
                Err(ProgressCallbackError)
            }
        },
    ) {
        Ok(puzzle) => Ok(generate_response_from_puzzle(puzzle, true, None)),
        Err(GenerationError::MaxAttemptsExceeded {
            attempts,
            closest: Some(puzzle),
        }) => {
            let target_met = puzzle.difficulty == target_difficulty;
            Ok(generate_response_from_puzzle(
                puzzle,
                target_met,
                Some(format!(
                    "target difficulty {} not reached in {attempts} attempts",
                    difficulty_name(target_difficulty)
                )),
            ))
        }
        Err(GenerationError::MaxAttemptsExceeded {
            attempts,
            closest: None,
        }) => Err(format!(
            "generation failed after {attempts} attempts without producing a puzzle"
        )),
        Err(GenerationError::ProgressCallbackFailed) => {
            Err(progress_error.unwrap_or_else(|| "progress callback failed".to_string()))
        }
    }
}

fn generate_response_from_puzzle(
    puzzle: Puzzle,
    target_met: bool,
    warning: Option<String>,
) -> GenerateResponse {
    GenerateResponse {
        puzzle_line: SudokuSerializer::format_line(&puzzle.state),
        puzzle_grid: SudokuSerializer::format_grid(&puzzle.state),
        solution_line: SudokuSerializer::format_line(&puzzle.solution),
        solution_grid: SudokuSerializer::format_grid(&puzzle.solution),
        region_line: SudokuSerializer::format_region_line(&puzzle.groups),
        difficulty: difficulty_name(puzzle.difficulty).to_string(),
        target_met,
        warning,
    }
}

fn map_generation_progress(progress: GenerationProgress<'_>) -> GenerationProgressResponse {
    match progress {
        GenerationProgress::AttemptStarted {
            attempt,
            max_attempts,
        } => progress_response(
            "attempt_started",
            attempt,
            max_attempts,
            None,
            None,
            None,
            None,
        ),
        GenerationProgress::RegionGenerationFailed {
            attempt,
            max_attempts,
        } => progress_response(
            "region_generation_failed",
            attempt,
            max_attempts,
            None,
            None,
            None,
            None,
        ),
        GenerationProgress::GroupsReady {
            attempt,
            max_attempts,
            groups,
        } => progress_response(
            "groups_ready",
            attempt,
            max_attempts,
            Some(groups),
            None,
            None,
            None,
        ),
        GenerationProgress::SolutionGenerated {
            attempt,
            max_attempts,
            groups,
            solution,
        } => progress_response(
            "solution_generated",
            attempt,
            max_attempts,
            Some(groups),
            Some(solution),
            None,
            None,
        ),
        GenerationProgress::ClueDiggingProgress {
            attempt,
            max_attempts,
            processed_steps,
            total_steps,
            remaining_clues,
        } => {
            let mut response = progress_response(
                "clue_digging",
                attempt,
                max_attempts,
                None,
                None,
                None,
                None,
            );
            response.processed_steps = Some(processed_steps);
            response.total_steps = Some(total_steps);
            response.remaining_clues = Some(remaining_clues);
            response
        }
        GenerationProgress::PuzzleDug {
            attempt,
            max_attempts,
            groups,
            solution,
            state,
        } => progress_response(
            "puzzle_dug",
            attempt,
            max_attempts,
            Some(groups),
            Some(solution),
            Some(state),
            None,
        ),
        GenerationProgress::AttemptFinished {
            attempt,
            max_attempts,
            puzzle,
            target_met,
        } => progress_response(
            "attempt_finished",
            attempt,
            max_attempts,
            Some(&puzzle.groups),
            Some(&puzzle.solution),
            Some(&puzzle.state),
            Some((puzzle.difficulty, target_met)),
        ),
        GenerationProgress::ClosestUpdated {
            attempt,
            max_attempts,
            puzzle,
        } => progress_response(
            "closest_updated",
            attempt,
            max_attempts,
            Some(&puzzle.groups),
            Some(&puzzle.solution),
            Some(&puzzle.state),
            Some((puzzle.difficulty, false)),
        ),
    }
}

fn progress_response(
    event: &str,
    attempt: usize,
    max_attempts: usize,
    groups: Option<&crate::cell_group::CellGroups>,
    solution: Option<&crate::GameState>,
    state: Option<&crate::GameState>,
    difficulty: Option<(Difficulty, bool)>,
) -> GenerationProgressResponse {
    GenerationProgressResponse {
        event: event.to_string(),
        attempt,
        max_attempts,
        puzzle_line: state.map(SudokuSerializer::format_line),
        puzzle_grid: state.map(SudokuSerializer::format_grid),
        solution_line: solution.map(SudokuSerializer::format_line),
        solution_grid: solution.map(SudokuSerializer::format_grid),
        region_line: groups.and_then(SudokuSerializer::format_region_line),
        difficulty: difficulty.map(|(difficulty, _)| difficulty_name(difficulty).to_string()),
        target_met: difficulty.map(|(_, target_met)| target_met),
        processed_steps: None,
        total_steps: None,
        remaining_clues: None,
    }
}

fn default_seed() -> u64 {
    let date_bits = js_sys::Date::now().to_bits();
    let random_bits = js_sys::Math::random().to_bits();
    date_bits ^ random_bits.rotate_left(17)
}

fn parse_state(puzzle: &str, format: PuzzleFormat) -> Result<crate::GameState, String> {
    let raw = puzzle.trim();
    let parsed = match format {
        PuzzleFormat::Line => SudokuSerializer::parse_line(raw),
        PuzzleFormat::Grid => SudokuSerializer::parse_grid(raw),
        PuzzleFormat::Auto => {
            if raw.contains('\n') || raw.contains('|') || raw.contains('-') || raw.contains('+') {
                SudokuSerializer::parse_grid(raw)
            } else {
                SudokuSerializer::parse_line(raw)
            }
        }
    };
    parsed.map_err(|e| format!("failed to parse puzzle: {e}"))
}

fn build_groups(variant: WasmVariant, region_line: Option<&str>) -> Result<CellGroups, String> {
    match variant {
        WasmVariant::Standard => Ok(CellGroups::default()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns()),
        WasmVariant::Hypersudoku => Ok(CellGroups::default()
            .with_hypersudoku_windows()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns()),
        WasmVariant::Nonomino => {
            let region_line = region_line
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "nonomino solving requires `region_line`".to_string())?;
            build_nonomino_groups(region_line)
        }
    }
}

fn build_nonomino_groups(region_line: &str) -> Result<CellGroups, String> {
    let chars: Vec<char> = region_line.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.len() != 81 {
        return Err(format!(
            "invalid `region_line` length: expected 81 cells, found {}",
            chars.len()
        ));
    }

    let mut groups = std::array::from_fn::<_, 9, _>(|_| Vec::<u8>::with_capacity(9));
    for (idx, symbol) in chars.into_iter().enumerate() {
        let upper = symbol.to_ascii_uppercase();
        if !('A'..='I').contains(&upper) {
            return Err(format!(
                "invalid region symbol '{symbol}' at position {idx}; expected A-I"
            ));
        }
        let group_index = (upper as u8 - b'A') as usize;
        groups[group_index].push(idx as u8);
    }

    for (i, group) in groups.iter().enumerate() {
        if group.len() != 9 {
            return Err(format!(
                "invalid region layout: group {} has {} cells, expected 9",
                (b'A' + i as u8) as char,
                group.len()
            ));
        }
    }

    let mut cell_groups = CellGroups::default().with_default_rows_and_columns();
    for group in groups {
        cell_groups = cell_groups.with_group_from_iter(group);
    }
    Ok(cell_groups)
}

const fn map_variant(value: WasmVariant) -> Variant {
    match value {
        WasmVariant::Standard => Variant::Standard,
        WasmVariant::Hypersudoku => Variant::Hypersudoku,
        WasmVariant::Nonomino => Variant::Nonomino,
    }
}

const fn map_difficulty(value: WasmDifficulty) -> Difficulty {
    match value {
        WasmDifficulty::Easy => Difficulty::Easy,
        WasmDifficulty::Medium => Difficulty::Medium,
        WasmDifficulty::Hard => Difficulty::Hard,
        WasmDifficulty::Expert => Difficulty::Expert,
        WasmDifficulty::Extreme => Difficulty::Extreme,
    }
}

const fn map_symmetry(value: WasmSymmetry) -> Symmetry {
    match value {
        WasmSymmetry::None => Symmetry::None,
        WasmSymmetry::Rotational => Symmetry::Rotational180,
    }
}

const fn difficulty_name(value: Difficulty) -> &'static str {
    match value {
        Difficulty::Easy => "easy",
        Difficulty::Medium => "medium",
        Difficulty::Hard => "hard",
        Difficulty::Expert => "expert",
        Difficulty::Extreme => "extreme",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::example_games;

    #[test]
    fn solve_standard_returns_solution() {
        let game = example_games::sudoku::example_sudoku();
        let req = SolveRequest {
            puzzle: SudokuSerializer::format_line(&game.initial_state),
            variant: WasmVariant::Standard,
            format: PuzzleFormat::Line,
            region_line: None,
        };

        let response = solve_puzzle_impl(req).expect("solve request should succeed");
        assert!(response.solved);
        assert_eq!(
            response.state_line,
            SudokuSerializer::format_line(&game.expected_solution.unwrap())
        );
    }

    #[test]
    fn solve_nonomino_requires_regions() {
        let game = example_games::nonomino::example_nonomino();
        let req = SolveRequest {
            puzzle: SudokuSerializer::format_line(&game.initial_state),
            variant: WasmVariant::Nonomino,
            format: PuzzleFormat::Line,
            region_line: None,
        };

        let err = solve_puzzle_impl(req).expect_err("missing region_line should fail");
        assert!(err.contains("region_line"));
    }

    #[test]
    fn solve_nonomino_with_regions_returns_solution() {
        let game = example_games::nonomino::example_nonomino();
        let req = SolveRequest {
            puzzle: SudokuSerializer::format_line(&game.initial_state),
            variant: WasmVariant::Nonomino,
            format: PuzzleFormat::Line,
            region_line: SudokuSerializer::format_region_line(&game.groups),
        };

        let response = solve_puzzle_impl(req).expect("solve request should succeed");
        assert!(response.solved);
        assert_eq!(
            response.state_line,
            SudokuSerializer::format_line(&game.expected_solution.unwrap())
        );
    }

    #[test]
    fn solve_step_response_adds_one_visible_value() {
        let game = example_games::sudoku::example_sudoku();
        let initial_line = SudokuSerializer::format_line(&game.initial_state);
        let req = SolveRequest {
            puzzle: initial_line.clone(),
            variant: WasmVariant::Standard,
            format: PuzzleFormat::Line,
            region_line: None,
        };

        let response = solve_step_impl(req).expect("solve step should succeed");
        let added = initial_line
            .chars()
            .zip(response.state_line.chars())
            .filter(|(before, after)| *before == '.' && *after != '.')
            .count();
        assert_eq!(added, 1);
    }

    #[test]
    fn map_helpers_cover_all_variants() {
        assert!(matches!(
            map_variant(WasmVariant::Standard),
            Variant::Standard
        ));
        assert!(matches!(
            map_variant(WasmVariant::Hypersudoku),
            Variant::Hypersudoku
        ));
        assert!(matches!(
            map_variant(WasmVariant::Nonomino),
            Variant::Nonomino
        ));
        assert_eq!(map_difficulty(WasmDifficulty::Easy), Difficulty::Easy);
        assert_eq!(map_difficulty(WasmDifficulty::Medium), Difficulty::Medium);
        assert_eq!(map_difficulty(WasmDifficulty::Hard), Difficulty::Hard);
        assert_eq!(map_difficulty(WasmDifficulty::Expert), Difficulty::Expert);
        assert_eq!(map_difficulty(WasmDifficulty::Extreme), Difficulty::Extreme);
        assert!(matches!(map_symmetry(WasmSymmetry::None), Symmetry::None));
        assert!(matches!(
            map_symmetry(WasmSymmetry::Rotational),
            Symmetry::Rotational180
        ));
        assert_eq!(difficulty_name(Difficulty::Easy), "easy");
        assert_eq!(difficulty_name(Difficulty::Medium), "medium");
        assert_eq!(difficulty_name(Difficulty::Hard), "hard");
        assert_eq!(difficulty_name(Difficulty::Expert), "expert");
        assert_eq!(difficulty_name(Difficulty::Extreme), "extreme");
        assert_eq!(default_max_attempts(), 200);
    }

    #[test]
    fn parse_state_auto_handles_grid_and_reports_errors() {
        let game = example_games::sudoku::example_sudoku();
        let grid = SudokuSerializer::format_grid(&game.initial_state);
        let parsed = parse_state(&grid, PuzzleFormat::Auto).expect("grid should parse");
        assert_eq!(
            SudokuSerializer::format_line(&parsed),
            SudokuSerializer::format_line(&game.initial_state)
        );

        let err = parse_state("123", PuzzleFormat::Line).expect_err("short line should fail");
        assert!(err.contains("failed to parse puzzle"));
    }

    #[test]
    fn build_groups_covers_hyper_and_region_errors() {
        let hyper =
            build_groups(WasmVariant::Hypersudoku, None).expect("hyper groups should build");
        assert!(hyper.iter().count() > 27);

        let short = build_nonomino_groups("ABC").expect_err("short layout should fail");
        assert!(short.contains("expected 81"));

        let bad_symbol = build_nonomino_groups(&format!("{}Z", "A".repeat(80)))
            .expect_err("bad symbol should fail");
        assert!(bad_symbol.contains("invalid region symbol"));
    }

    #[test]
    fn solve_step_response_handles_solved_state() {
        let game = example_games::sudoku::example_sudoku();
        let solved = game.expected_solution.unwrap();
        let solved_req = SolveRequest {
            puzzle: SudokuSerializer::format_line(&solved),
            variant: WasmVariant::Standard,
            format: PuzzleFormat::Line,
            region_line: None,
        };
        let solved_response = solve_step_impl(solved_req).expect("solved step should succeed");
        assert!(!solved_response.changed);
        assert!(solved_response.solved);
    }

    #[test]
    fn solve_step_response_reports_invalid_state() {
        let mut puzzle = [0u8; 81];
        puzzle[0] = 1;
        puzzle[1] = 1;
        let req = SolveRequest {
            puzzle: SudokuSerializer::format_line(&crate::GameState::new_from(puzzle)),
            variant: WasmVariant::Standard,
            format: PuzzleFormat::Line,
            region_line: None,
        };

        let response = solve_step_impl(req).expect("invalid state should serialize response");
        assert!(!response.changed);
        assert_eq!(
            response.error.as_deref(),
            Some("The puzzle has an invalid game state")
        );
    }

    #[test]
    fn generation_progress_serializes_clue_digging_details() {
        let response = map_generation_progress(GenerationProgress::ClueDiggingProgress {
            attempt: 2,
            max_attempts: 5,
            processed_steps: 7,
            total_steps: 81,
            remaining_clues: 42,
        });

        assert_eq!(response.event, "clue_digging");
        assert_eq!(response.attempt, 2);
        assert_eq!(response.max_attempts, 5);
        assert_eq!(response.processed_steps, Some(7));
        assert_eq!(response.total_steps, Some(81));
        assert_eq!(response.remaining_clues, Some(42));
        assert!(response.puzzle_line.is_none());
    }

    #[test]
    fn generate_with_callback_reports_progress_to_wasm_caller() {
        let req = GenerateRequest {
            variant: WasmVariant::Standard,
            target_difficulty: WasmDifficulty::Easy,
            symmetry: WasmSymmetry::None,
            max_attempts: 1,
            seed: Some(42),
        };
        let mut events = Vec::new();
        let response = generate_puzzle_with_callback_impl(req, |progress| {
            events.push(progress.event);
            Ok(())
        })
        .expect("generation should return a puzzle or closest puzzle");

        assert_eq!(response.puzzle_line.len(), 81);
        assert!(events.contains(&"attempt_started".to_string()));
        assert!(events.contains(&"groups_ready".to_string()));
        assert!(events.contains(&"solution_generated".to_string()));
        assert!(events.contains(&"clue_digging".to_string()));
        assert!(events.contains(&"puzzle_dug".to_string()));
        assert!(events.contains(&"attempt_finished".to_string()));
    }

    #[test]
    fn generate_with_callback_propagates_progress_errors() {
        let req = GenerateRequest {
            variant: WasmVariant::Standard,
            target_difficulty: WasmDifficulty::Easy,
            symmetry: WasmSymmetry::None,
            max_attempts: 1,
            seed: Some(42),
        };
        let mut events = 0usize;
        let err = generate_puzzle_with_callback_impl(req, |_| {
            events += 1;
            Err("callback failed".to_string())
        })
        .expect_err("callback errors should abort generation");

        assert_eq!(err, "callback failed");
        assert_eq!(events, 1);
    }

    #[test]
    fn invalid_nonomino_region_layout_reports_group_size() {
        let err = build_nonomino_groups(&"A".repeat(81)).expect_err("layout should fail");
        assert!(err.contains("group A has 81 cells"));
    }

    #[test]
    fn generation_progress_serializes_edge_variants() {
        let game = example_games::sudoku::example_sudoku();
        let puzzle = Puzzle {
            state: game.initial_state.clone(),
            solution: game.expected_solution.unwrap(),
            groups: game.groups.clone(),
            difficulty: Difficulty::Hard,
        };

        let attempt = map_generation_progress(GenerationProgress::AttemptStarted {
            attempt: 1,
            max_attempts: 3,
        });
        assert_eq!(attempt.event, "attempt_started");
        assert_eq!(attempt.attempt, 1);

        let failed = map_generation_progress(GenerationProgress::RegionGenerationFailed {
            attempt: 2,
            max_attempts: 3,
        });
        assert_eq!(failed.event, "region_generation_failed");

        let closest = map_generation_progress(GenerationProgress::ClosestUpdated {
            attempt: 3,
            max_attempts: 3,
            puzzle: &puzzle,
        });
        assert_eq!(closest.event, "closest_updated");
        assert_eq!(closest.difficulty.as_deref(), Some("hard"));
        assert_eq!(closest.target_met, Some(false));
    }

    #[test]
    fn generate_impl_reports_closest_and_empty_failures() {
        let closest_req = GenerateRequest {
            variant: WasmVariant::Standard,
            target_difficulty: WasmDifficulty::Medium,
            symmetry: WasmSymmetry::None,
            max_attempts: 1,
            seed: Some(42),
        };
        let closest = generate_puzzle_impl(closest_req).expect("closest puzzle should be returned");
        assert_eq!(closest.puzzle_line.len(), 81);
        assert!(closest
            .warning
            .as_deref()
            .unwrap_or_default()
            .contains("target difficulty"));

        let empty_req = GenerateRequest {
            variant: WasmVariant::Standard,
            target_difficulty: WasmDifficulty::Easy,
            symmetry: WasmSymmetry::None,
            max_attempts: 0,
            seed: Some(42),
        };
        let err = generate_puzzle_impl(empty_req).expect_err("zero attempts should fail");
        assert!(err.contains("without producing a puzzle"));
    }

    #[test]
    fn generate_standard_returns_puzzle() {
        let req = GenerateRequest {
            variant: WasmVariant::Standard,
            target_difficulty: WasmDifficulty::Easy,
            symmetry: WasmSymmetry::None,
            max_attempts: 30,
            seed: Some(42),
        };

        let response = generate_puzzle_impl(req).expect("generation should produce a puzzle");
        assert_eq!(response.puzzle_line.len(), 81);
        assert_eq!(response.solution_line.len(), 81);
    }
}
