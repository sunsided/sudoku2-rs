use clap::{Arg, ArgGroup, Command};
use std::time::Instant;
use sudoku2::visualization::{ascii::print_game_state, PrintAscii};
use sudoku2::*;

fn main() {
    // Enable logging with RUST_LOG=debug
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .init();

    let matches = build_command().get_matches();
    let game = if matches.get_flag("normal-ht") {
        example_games::sudoku2::example_sudoku()
    } else if matches.get_flag("normal-xwing") {
        example_games::sudoku_xwings::example_sudoku()
    } else if matches.get_flag("normal-hardest") {
        example_games::sudoku2::example_sudoku_hardest()
    } else if matches.get_flag("nonomino") {
        example_games::nonomino::example_nonomino()
    } else if matches.get_flag("hypersudoku") {
        example_games::hypersudoku::example_hypersudoku()
    } else {
        debug_assert!(matches.get_flag("normal"));
        example_games::sudoku::example_sudoku()
    };

    println!("Cell groups:");
    game.print_cell_groups();

    println!("Initial game state:");
    game.print_game_state();

    assert!(game.initial_state.is_consistent(&game.groups));

    let mut solver = DefaultSolver::new(&game.groups);
    solver.set_print_fn(|state| print_game_state(state));

    let now = Instant::now();
    let result = solver.solve(&game.initial_state);
    let duration = now.elapsed();

    match result {
        Ok(solution) => {
            println!("Found solution:");
            print_game_state(&solution);

            if let Some(expected_solution) = game.expected_solution {
                if expected_solution.eq(&solution) {
                    println!("(Solution matches expectation.)");
                } else {
                    println!("(Solution differs from expectation.)");
                }
            }
        }
        Err(Unsolvable(state)) => {
            println!("Last available state:");
            print_game_state(&state);
            eprintln!("Failed to find a solution.");
        }
    }

    println!(
        "Search terminated after {} s.",
        duration.subsec_micros() as f64 * 1e-6
    );
}

pub fn build_command() -> Command {
    let command = Command::new("Sudoku Solver Example")
        .version("0.1.0")
        .author("Markus Mayer")
        .arg(
            Arg::new("normal")
                .long("sudoku")
                .help("Solve a regular Sudoku")
                .action(clap::ArgAction::SetTrue)
                .help_heading("Game type")
                .group("type"),
        )
        .arg(
            Arg::new("normal-ht")
                .long("sudoku-ht")
                .help("Solve a regular Sudoku with known Hidden Twins")
                .action(clap::ArgAction::SetTrue)
                .help_heading("Game type")
                .group("type"),
        )
        .arg(
            Arg::new("normal-xwing")
                .long("sudoku-xwing")
                .help("Solve a regular Sudoku with known X-Wings")
                .action(clap::ArgAction::SetTrue)
                .help_heading("Game type")
                .group("type"),
        )
        .arg(
            Arg::new("normal-hardest")
                .long("sudoku-hardest")
                .help("Solve a Sudoku of \"hardest\" difficulty")
                .action(clap::ArgAction::SetTrue)
                .help_heading("Game type")
                .group("type"),
        )
        .arg(
            Arg::new("nonomino")
                .long("nonomino")
                .help("Solve a Nonomino-type game")
                .action(clap::ArgAction::SetTrue)
                .help_heading("Game type")
                .group("type"),
        )
        .arg(
            Arg::new("hypersudoku")
                .long("hyper")
                .help("Solve a Hypersoduko-type game")
                .action(clap::ArgAction::SetTrue)
                .help_heading("Game type")
                .group("type"),
        )
        .group(ArgGroup::new("type"));
    command
}
