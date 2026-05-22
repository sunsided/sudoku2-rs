use crate::*;
use std::io::Write;

pub trait PrintAscii {
    fn print_game_state(&self);
    fn print_cell_groups(&self);
}

impl PrintAscii for Game {
    fn print_game_state(&self) {
        print_game_state(&self.initial_state);
    }

    fn print_cell_groups(&self) {
        print_cell_groups(&self.groups);
    }
}

pub fn print_game_state(state: &GameState) {
    // Horizontal bar.
    for x in 0..(6 * 9 + 19) {
        if x == 0 {
            print!("┌");
        } else if x == 6 * 9 + 18 {
            print!("┐");
        } else if x % 8 == 0 {
            print!("┬");
        } else {
            print!("─");
        }
    }

    println!();

    for y in 0..9 {
        // We'll require three rows to print all possible values.
        for row in 0..3 {
            for x in 0..9 {
                if x == 0 {
                    print!("│ ");
                }

                let cell = state.get_at_xy(x, y);
                for value in (1 + row * 3)..=(3 + row * 3) {
                    let value = Value::try_from(value).unwrap();
                    if cell.contains(value) {
                        print!("{} ", *value);
                    } else {
                        print!("· ")
                    }
                }

                if x < 8 {
                    print!("│ ");
                } else {
                    print!("│");
                }
            }
            println!();
        }

        if y < 8 {
            // Horizontal bar.
            for x in 0..(6 * 9 + 19) {
                if x == 0 {
                    print!("├");
                } else if x == 6 * 9 + 18 {
                    print!("┤");
                } else if x % 8 == 0 {
                    print!("┼");
                } else {
                    print!("─");
                }
            }

            println!();
        }
    }

    // Horizontal bar.
    for x in 0..(6 * 9 + 19) {
        if x == 0 {
            print!("└");
        } else if x == 6 * 9 + 18 {
            print!("┘");
        } else if x % 8 == 0 {
            print!("┴");
        } else {
            print!("─");
        }
    }

    println!();
    std::io::stdout().flush().unwrap();
}

pub fn print_solution(state: &GameState) {
    let bar = |left: &str, mid: &str, right: &str| {
        print!("{}", left);
        for x in 0..9 {
            print!("───");
            if x < 8 {
                print!("{}", mid);
            }
        }
        println!("{}", right);
    };

    bar("┌", "┬", "┐");
    for y in 0..9 {
        print!("│");
        for x in 0..9 {
            let cell = state.get_at_xy(x, y);
            if cell.is_solved() {
                let value = cell.iter_candidates().next().unwrap();
                print!(" {} │", value.get());
            } else {
                print!(" ? │");
            }
        }
        println!();
        if y < 8 {
            bar("├", "┼", "┤");
        }
    }
    bar("└", "┴", "┘");
    std::io::stdout().flush().unwrap();
}

pub fn print_cell_groups(groups: &CellGroups) {
    let mut group_names = vec!["@".into()];
    for i in 0..27 {
        group_names.push(format!("{}", (b'A' + i) as char));
    }
    for i in 0..10 {
        group_names.push(format!("{}", i));
    }

    // Horizontal bar.
    for x in 0..(2 * 9 + 19) {
        if x == 0 {
            print!("┌");
        } else if x == 2 * 9 + 18 {
            print!("┐");
        } else if x % 4 == 0 {
            print!("┬");
        } else {
            print!("─");
        }
    }

    println!();

    for y in 0..9 {
        for x in 0..9 {
            if x == 0 {
                print!("│ ");
            }

            let mut group = groups.get_groups_at_xy(x, y).expect("invalid groups");
            group.sort_unstable_by_key(|g| g.group_type); // ensure custom groups first
            let group = group.first().unwrap();
            print!(
                "{} ",
                group.id.map_or("-".into(), |x| group_names[x].clone())
            );

            if (x + 1) % 3 == 0 {
                print!("│ ");
            } else if x < 8 {
                print!("· ");
            } else {
                print!("│");
            }
        }
        println!();

        if y < 8 {
            // Horizontal bar.
            for x in 0..(2 * 9 + 19) {
                if x == 0 {
                    print!("├");
                } else if x == 2 * 9 + 18 {
                    print!("┤");
                } else if x % 4 == 0 {
                    print!("┼");
                } else if (y + 1) % 3 == 0 {
                    print!("─");
                } else {
                    print!("·");
                }
            }

            println!();
        }
    }

    // Horizontal bar.
    for x in 0..(2 * 9 + 19) {
        if x == 0 {
            print!("└");
        } else if x == 2 * 9 + 18 {
            print!("┘");
        } else if x % 4 == 0 {
            print!("┴");
        } else {
            print!("─");
        }
    }

    println!();
    std::io::stdout().flush().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::example_games::sudoku::example_sudoku;
    use crate::DefaultSolver;

    #[test]
    fn print_game_state_runs_for_initial_board() {
        let game = example_sudoku();
        // Exercises the trait impl and the free function path.
        game.print_game_state();
        print_game_state(&game.initial_state);
    }

    #[test]
    fn print_cell_groups_runs_for_default_groups() {
        let game = example_sudoku();
        game.print_cell_groups();
        print_cell_groups(&game.groups);
    }

    #[test]
    fn print_solution_runs_for_unsolved_and_solved_states() {
        let game = example_sudoku();
        // Unsolved state hits the `?` branch for every cell.
        print_solution(&game.initial_state);

        let solver = DefaultSolver::new(&game.groups);
        let solved = solver
            .solve(&game.initial_state)
            .expect("example sudoku should be solvable");
        // Solved state hits the `cell.is_solved()` branch.
        print_solution(&solved);
    }
}
