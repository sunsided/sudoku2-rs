use std::io::Write;
use std::num::NonZeroU8;
use std::time::Instant;
use sudoku2::prelude::*;
use sudoku2::visualization::{ascii::print_game_state, PrintAscii};

fn main() {
    // Enable logging with RUST_LOG=debug
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .init();

    let game = sudoku2::example_games::sudoku::example_sudoku();

    println!("Cell groups:");
    game.print_cell_groups();

    println!("Initial game state:");
    game.print_game_state();

    assert!(game.initial_state.is_consistent(&game.groups));

    let mut solver = DefaultSolver::new(game.groups);
    solver.set_print_fn(|state| print_game_state(state));

    let now = Instant::now();
    let solved = solver.solve(game.initial_state).unwrap();

    println!(
        "Search terminated after {} s.",
        now.elapsed().subsec_micros() as f64 * 1e-6
    );

    println!("Best solution:");
    print_game_state(&solved);
}
