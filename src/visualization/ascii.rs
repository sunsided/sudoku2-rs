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

/// Builds an 81-entry region index array from Custom groups.
/// Returns None if no Custom groups exist or if coverage is incomplete (not all 81 cells assigned).
fn custom_region_map(groups: &CellGroups) -> Option<[u8; 81]> {
    let mut map = [u8::MAX; 81];
    let mut found = false;
    for (i, group) in groups
        .iter()
        .filter(|g| g.group_type == CellGroupType::Custom)
        .enumerate()
    {
        found = true;
        for index in group.iter_indexes() {
            map[*index as usize] = i as u8;
        }
    }
    if !found || map.contains(&u8::MAX) {
        return None;
    }
    Some(map)
}

/// Returns true if a region border separates cells (ax,ay) and (bx,by).
/// Out-of-bounds coords are treated as outside the grid (boundary = true, both-outside = false).
fn is_edge(map: &Option<[u8; 81]>, ax: i32, ay: i32, bx: i32, by: i32) -> bool {
    let in_bounds = |x: i32, y: i32| (0..9).contains(&x) && (0..9).contains(&y);
    let a_valid = in_bounds(ax, ay);
    let b_valid = in_bounds(bx, by);
    if !a_valid && !b_valid {
        return false;
    }
    if a_valid != b_valid {
        return true; // grid boundary
    }
    match map {
        None => true, // no custom regions: all cell borders shown
        Some(m) => m[(ay * 9 + ax) as usize] != m[(by * 9 + bx) as usize],
    }
}

/// Returns the box-drawing character for a junction at grid position (jx, jy),
/// where jx and jy range over 0..=9 (10×10 junction grid for 9×9 cells).
fn junction_char_at(map: &Option<[u8; 81]>, jx: i32, jy: i32) -> char {
    let n = jy > 0 && is_edge(map, jx - 1, jy - 1, jx, jy - 1);
    let s = jy < 9 && is_edge(map, jx - 1, jy, jx, jy);
    let w = jx > 0 && is_edge(map, jx - 1, jy - 1, jx - 1, jy);
    let e = jx < 9 && is_edge(map, jx, jy - 1, jx, jy);
    match (n, s, w, e) {
        (true, true, true, true) => '┼',
        (true, true, true, false) => '┤',
        (true, true, false, true) => '├',
        (true, true, false, false) => '│',
        (true, false, true, true) => '┴',
        (true, false, true, false) => '┘',
        (true, false, false, true) => '└',
        (true, false, false, false) => '╵',
        (false, true, true, true) => '┬',
        (false, true, true, false) => '┐',
        (false, true, false, true) => '┌',
        (false, true, false, false) => '╷',
        (false, false, true, true) => '─',
        (false, false, true, false) => '╴',
        (false, false, false, true) => '╶',
        (false, false, false, false) => ' ',
    }
}

/// Renders a 9×9 grid using region-aware borders derived from `map`.
/// `cell_content(x, y)` must return exactly 3 chars (e.g. `" 5 "` or `" A "`).
fn print_grid_with_regions<F>(map: &Option<[u8; 81]>, cell_content: F)
where
    F: Fn(u8, u8) -> String,
{
    for jy in 0i32..=9 {
        // Junction row
        for jx in 0i32..=9 {
            print!("{}", junction_char_at(map, jx, jy));
            if jx < 9 {
                if is_edge(map, jx, jy - 1, jx, jy) {
                    print!("───");
                } else {
                    print!("   ");
                }
            }
        }
        println!();
        if jy < 9 {
            let y = jy as u8;
            for x in 0u8..9 {
                let has_left = is_edge(map, x as i32 - 1, y as i32, x as i32, y as i32);
                print!("{}", if has_left { '│' } else { ' ' });
                print!("{}", cell_content(x, y));
            }
            println!("│");
        }
    }
    std::io::stdout().flush().unwrap();
}

/// Prints a 9×9 solution grid with borders drawn at region boundaries.
///
/// For standard and hypersudoku puzzles (no complete Custom group coverage) this
/// renders identically to [`print_solution`]. For nonomino puzzles the borders
/// follow the irregular region outlines.
pub fn print_solution_with_regions(state: &GameState, groups: &CellGroups) {
    let map = custom_region_map(groups);
    print_grid_with_regions(&map, |x, y| {
        let cell = state.get_at_xy(x, y);
        if cell.is_solved() {
            let v = cell.iter_candidates().next().unwrap();
            format!(" {} ", v.get())
        } else {
            " ? ".to_string()
        }
    });
}

/// Prints an 9×9 letter grid showing which region each cell belongs to (A–I).
/// Does nothing if `groups` has no complete Custom group coverage.
pub fn print_nonomino_regions(groups: &CellGroups) {
    let map = custom_region_map(groups);
    if map.is_none() {
        return;
    }
    let names = b"ABCDEFGHI";
    let m = map.as_ref().unwrap();
    print_grid_with_regions(&map, |x, y| {
        let region = m[y as usize * 9 + x as usize];
        let letter = if (region as usize) < names.len() {
            names[region as usize] as char
        } else {
            '?'
        };
        format!(" {} ", letter)
    });
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

    #[test]
    fn print_solution_with_regions_standard_matches_behavior_of_print_solution() {
        let game = example_sudoku();
        // Standard groups have no complete Custom group coverage, so rendering
        // should be equivalent to print_solution (all borders shown).
        print_solution_with_regions(&game.initial_state, &game.groups);
        let solver = DefaultSolver::new(&game.groups);
        let solved = solver
            .solve(&game.initial_state)
            .expect("example sudoku should be solvable");
        print_solution_with_regions(&solved, &game.groups);
    }

    #[test]
    fn print_nonomino_regions_standard_is_noop() {
        let game = example_sudoku();
        // Standard groups have no complete Custom coverage; function returns early.
        print_nonomino_regions(&game.groups);
    }

    #[test]
    fn print_solution_with_regions_nonomino_runs_without_panic() {
        use crate::generator::NonominoRegionGenerator;
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(1);
        let groups = loop {
            if let Some(g) = NonominoRegionGenerator::default().generate(&mut rng) {
                break g;
            }
        };
        // Use an empty state to avoid the slow grid generator in debug mode.
        let state = GameState::new();
        print_solution_with_regions(&state, &groups);
        print_nonomino_regions(&groups);
    }

    #[test]
    fn custom_region_map_complete_coverage_returns_some() {
        use crate::generator::NonominoRegionGenerator;
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(1);
        let groups = loop {
            if let Some(g) = NonominoRegionGenerator::default().generate(&mut rng) {
                break g;
            }
        };
        assert!(custom_region_map(&groups).is_some());
    }

    #[test]
    fn custom_region_map_partial_coverage_returns_none() {
        // Hypersudoku windows cover only 36 of 81 cells.
        let groups = CellGroups::default()
            .with_hypersudoku_windows()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns();
        assert!(custom_region_map(&groups).is_none());
    }
}
