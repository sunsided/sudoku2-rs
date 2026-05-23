use clap::{value_parser, Arg, Command};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::time::Instant;
use sudoku2::serialization::SudokuSerializer;
use sudoku2::visualization::ascii::{print_nonomino_regions, print_solution_with_regions};
use sudoku2::*;

fn main() {
    let matches = build_command().get_matches();

    let variant = match matches.get_one::<String>("variant").map(String::as_str) {
        Some("hypersudoku") | Some("hyper") => Variant::Hypersudoku,
        Some("nonomino") => Variant::Nonomino,
        _ => Variant::Standard,
    };

    let target = match matches.get_one::<String>("difficulty").map(String::as_str) {
        Some("easy") => Difficulty::Easy,
        Some("hard") => Difficulty::Hard,
        Some("expert") => Difficulty::Expert,
        Some("extreme") => Difficulty::Extreme,
        _ => Difficulty::Medium,
    };

    let symmetry = match matches.get_one::<String>("symmetry").map(String::as_str) {
        Some("rotational") | Some("rot") => Symmetry::Rotational180,
        _ => Symmetry::None,
    };

    let max_attempts = *matches.get_one::<usize>("attempts").unwrap_or(&200);
    let format_arg = matches.get_one::<String>("format").map(String::as_str);
    let output_arg = matches.get_one::<String>("output").map(String::as_str);

    let seed = matches
        .get_one::<u64>("seed")
        .copied()
        .unwrap_or_else(|| rand::rng().random());
    println!("Seed: {seed}");
    let mut rng = StdRng::seed_from_u64(seed);

    println!("Variant:     {variant:?}");
    println!("Difficulty:  {target:?}");
    println!("Symmetry:    {symmetry:?}");
    println!("Max attempts: {max_attempts}");

    let config = PuzzleGeneratorConfig {
        variant,
        target_difficulty: target,
        symmetry,
        max_attempts,
    };

    let now = Instant::now();
    let result = PuzzleGenerator::new(config).generate(&mut rng);
    let elapsed = now.elapsed();

    match result {
        Ok(puzzle) => {
            println!(
                "\nGenerated {:?} puzzle in {:.3}s:",
                puzzle.difficulty,
                elapsed.as_secs_f64()
            );
            println!("\nPuzzle:");
            print_solution_with_regions(&puzzle.state, &puzzle.groups);
            println!("\nSolution:");
            print_solution_with_regions(&puzzle.solution, &puzzle.groups);
            if let Some(region_line) = SudokuSerializer::format_region_line(&puzzle.groups) {
                println!("\nRegion layout (line): {region_line}");
                println!("\nRegion layout:");
                print_nonomino_regions(&puzzle.groups);
            }
            export_puzzle(&puzzle, format_arg, output_arg);
        }
        Err(GenerationError::MaxAttemptsExceeded {
            attempts,
            closest: Some(puzzle),
        }) => {
            eprintln!(
                "Warning: target {:?} not reached in {attempts} attempts; showing closest ({:?})",
                target, puzzle.difficulty
            );
            println!(
                "\nPuzzle ({:?}) generated in {:.3}s:",
                puzzle.difficulty,
                elapsed.as_secs_f64()
            );
            println!("\nPuzzle:");
            print_solution_with_regions(&puzzle.state, &puzzle.groups);
            println!("\nSolution:");
            print_solution_with_regions(&puzzle.solution, &puzzle.groups);
            if let Some(region_line) = SudokuSerializer::format_region_line(&puzzle.groups) {
                println!("\nRegion layout (line): {region_line}");
                println!("\nRegion layout:");
                print_nonomino_regions(&puzzle.groups);
            }
            export_puzzle(&puzzle, format_arg, output_arg);
        }
        Err(GenerationError::MaxAttemptsExceeded {
            attempts,
            closest: None,
        }) => {
            eprintln!("Failed: no valid puzzle produced in {attempts} attempts.");
            std::process::exit(1);
        }
    }
}

/// Prints the serialized puzzle to stdout and/or writes it to a file.
///
/// `format_arg` controls stdout output: `"line"` prints an 81-character string,
/// `"grid"` prints nine lines of nine characters. When `None`, no serialized
/// output is printed.
///
/// `output_arg` is an optional file path. When provided, the puzzle is written
/// in line format to that file.
fn export_puzzle(puzzle: &Puzzle, format_arg: Option<&str>, output_arg: Option<&str>) {
    let line = SudokuSerializer::format_line(&puzzle.state);
    let region_line = SudokuSerializer::format_region_line(&puzzle.groups);

    match format_arg {
        Some("line") => {
            println!("\nPuzzle (line): {line}");
            if let Some(ref r) = region_line {
                println!("Region (line): {r}");
            }
        }
        Some("grid") => {
            println!("\nPuzzle (grid):");
            print!("{}", SudokuSerializer::format_grid(&puzzle.state));
        }
        _ => {}
    }

    if let Some(path) = output_arg {
        let content = if let Some(ref r) = region_line {
            format!("{line}\n{r}\n")
        } else {
            format!("{line}\n")
        };
        std::fs::write(path, &content).unwrap_or_else(|e| {
            eprintln!("Error writing to '{path}': {e}");
            std::process::exit(1);
        });
        println!("Puzzle written to '{path}'.");
    }
}

fn build_command() -> Command {
    Command::new("Sudoku Generator")
        .version("0.1.0")
        .author("Markus Mayer")
        .about("Generate Sudoku puzzles at a target difficulty")
        .arg(
            Arg::new("variant")
                .long("variant")
                .short('v')
                .help("Puzzle variant: standard, hypersudoku (hyper), nonomino")
                .default_value("standard"),
        )
        .arg(
            Arg::new("difficulty")
                .long("difficulty")
                .short('d')
                .help("Target difficulty: easy, medium, hard, expert, extreme")
                .default_value("medium"),
        )
        .arg(
            Arg::new("symmetry")
                .long("symmetry")
                .short('s')
                .help("Clue symmetry: none, rotational (rot)")
                .default_value("none"),
        )
        .arg(
            Arg::new("seed")
                .long("seed")
                .help("RNG seed for reproducible output")
                .value_parser(value_parser!(u64)),
        )
        .arg(
            Arg::new("attempts")
                .long("attempts")
                .short('a')
                .help("Maximum generation attempts")
                .value_parser(value_parser!(usize))
                .default_value("200"),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .short('f')
                .help("Print puzzle in serialized form: line (81 chars) or grid (9 lines)")
                .value_name("line|grid"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .help("Write puzzle to a file in line format (81 characters)")
                .value_name("FILE"),
        )
}
