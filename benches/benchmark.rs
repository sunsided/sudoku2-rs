use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sudoku2::prelude::DefaultSolver;

fn criterion_benchmark(c: &mut Criterion) {
    {
        let sudoku_game = sudoku2::example_games::sudoku::example_sudoku();
        let sudoku_solver = DefaultSolver::new(&sudoku_game.groups);
        c.bench_function("sudoku", |b| {
            b.iter(|| sudoku_solver.solve(black_box(&sudoku_game.initial_state)))
        });
    }

    {
        let nonomino_game = sudoku2::example_games::nonomino::example_nonomino();
        let nonomino_solver = DefaultSolver::new(&nonomino_game.groups);
        c.bench_function("nonomino", |b| {
            b.iter(|| nonomino_solver.solve(black_box(&nonomino_game.initial_state)))
        });
    }

    {
        let hypersudoku_game = sudoku2::example_games::hypersudoku::example_hypersudoku();
        let hypersudoku_solver = DefaultSolver::new(&hypersudoku_game.groups);
        c.bench_function("hypersudoku", |b| {
            b.iter(|| hypersudoku_solver.solve(black_box(&hypersudoku_game.initial_state)))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
