use clap::{value_parser, Arg, Command};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::time::Instant;
use sudoku2::visualization::ascii::print_solution;
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
            print_solution(&puzzle.state);
            println!("\nSolution:");
            print_solution(&puzzle.solution);
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
            print_solution(&puzzle.state);
            println!("\nSolution:");
            print_solution(&puzzle.solution);
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
}
