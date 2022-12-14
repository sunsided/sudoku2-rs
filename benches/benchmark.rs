use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sudoku2::{example_games, DefaultSolver, DefaultSolverConfig};

fn criterion_benchmark(c: &mut Criterion) {
    {
        let sudoku_game = example_games::sudoku::example_sudoku();
        let sudoku_solver = DefaultSolver::new(&sudoku_game.groups);
        c.bench_function("sudoku", |b| {
            b.iter(|| sudoku_solver.solve(black_box(&sudoku_game.initial_state)))
        });
    }

    {
        let sudoku_game = example_games::sudoku_xwings::example_sudoku();
        let sudoku_solver = DefaultSolver::new(&sudoku_game.groups);
        c.bench_function("sudoku-xwings", |b| {
            b.iter(|| sudoku_solver.solve(black_box(&sudoku_game.initial_state)))
        });
    }

    {
        let sudoku_game = example_games::sudoku2::example_sudoku_hardest();
        let sudoku_solver = DefaultSolver::new_with(
            &sudoku_game.groups,
            DefaultSolverConfig {
                hidden_twins: false,
                ..Default::default()
            },
        );
        c.bench_function("sudoku-hardest without Hidden Twins", |b| {
            b.iter(|| sudoku_solver.solve(black_box(&sudoku_game.initial_state)))
        });

        let sudoku_solver = DefaultSolver::new_with(
            &sudoku_game.groups,
            DefaultSolverConfig {
                hidden_twins: true,
                ..Default::default()
            },
        );
        c.bench_function("sudoku-hardest with Hidden Twins", |b| {
            b.iter(|| sudoku_solver.solve(black_box(&sudoku_game.initial_state)))
        });
    }

    {
        let nonomino_game = example_games::nonomino::example_nonomino();
        let nonomino_solver = DefaultSolver::new(&nonomino_game.groups);
        c.bench_function("nonomino", |b| {
            b.iter(|| nonomino_solver.solve(black_box(&nonomino_game.initial_state)))
        });
    }

    {
        let hypersudoku_game = example_games::hypersudoku::example_hypersudoku();
        let hypersudoku_solver = DefaultSolver::new(&hypersudoku_game.groups);
        c.bench_function("hypersudoku", |b| {
            b.iter(|| hypersudoku_solver.solve(black_box(&hypersudoku_game.initial_state)))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
