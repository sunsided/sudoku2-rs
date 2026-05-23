use crate::cell_group::{CellGroups, WithGroupFromIterator};
use crate::default_solver::Unsolvable;
use crate::difficulty_estimator::Difficulty;
use crate::generator::{
    GenerationError, PuzzleGenerator, PuzzleGeneratorConfig, Symmetry, Variant,
};
use crate::{DefaultSolver, SudokuSerializer};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum WasmVariant {
    Standard,
    #[serde(alias = "hyper")]
    Hypersudoku,
    Nonomino,
}

impl Default for WasmVariant {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum WasmDifficulty {
    Easy,
    Medium,
    Hard,
    Expert,
    Extreme,
}

impl Default for WasmDifficulty {
    fn default() -> Self {
        Self::Medium
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum WasmSymmetry {
    None,
    #[serde(alias = "rot")]
    Rotational,
}

impl Default for WasmSymmetry {
    fn default() -> Self {
        Self::None
    }
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

fn generate_puzzle_impl(req: GenerateRequest) -> Result<GenerateResponse, String> {
    let seed = req.seed.unwrap_or_else(|| js_sys::Date::now() as u64);
    let mut rng = StdRng::seed_from_u64(seed);
    let target_difficulty = map_difficulty(req.target_difficulty);

    let config = PuzzleGeneratorConfig {
        variant: map_variant(req.variant),
        target_difficulty,
        symmetry: map_symmetry(req.symmetry),
        max_attempts: req.max_attempts,
    };

    match PuzzleGenerator::new(config).generate(&mut rng) {
        Ok(puzzle) => Ok(GenerateResponse {
            puzzle_line: SudokuSerializer::format_line(&puzzle.state),
            puzzle_grid: SudokuSerializer::format_grid(&puzzle.state),
            solution_line: SudokuSerializer::format_line(&puzzle.solution),
            solution_grid: SudokuSerializer::format_grid(&puzzle.solution),
            region_line: SudokuSerializer::format_region_line(&puzzle.groups),
            difficulty: difficulty_name(puzzle.difficulty).to_string(),
            target_met: true,
            warning: None,
        }),
        Err(GenerationError::MaxAttemptsExceeded {
            attempts,
            closest: Some(puzzle),
        }) => Ok(GenerateResponse {
            puzzle_line: SudokuSerializer::format_line(&puzzle.state),
            puzzle_grid: SudokuSerializer::format_grid(&puzzle.state),
            solution_line: SudokuSerializer::format_line(&puzzle.solution),
            solution_grid: SudokuSerializer::format_grid(&puzzle.solution),
            region_line: SudokuSerializer::format_region_line(&puzzle.groups),
            difficulty: difficulty_name(puzzle.difficulty).to_string(),
            target_met: puzzle.difficulty == target_difficulty,
            warning: Some(format!(
                "target difficulty {:?} not reached in {attempts} attempts",
                target_difficulty
            )),
        }),
        Err(GenerationError::MaxAttemptsExceeded {
            attempts,
            closest: None,
        }) => Err(format!(
            "generation failed after {attempts} attempts without producing a puzzle"
        )),
    }
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
