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

/// Visual style of a single border segment.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Stroke {
    None,
    Solid,
    Dashed,
}

impl Stroke {
    fn present(self) -> bool {
        !matches!(self, Stroke::None)
    }
}

/// Builds an 81-entry array assigning each cell to a group of the given type.
/// Returns None if no such groups exist or if coverage is incomplete.
fn cell_group_map(groups: &CellGroups, group_type: CellGroupType) -> Option<[u8; 81]> {
    let mut map = [u8::MAX; 81];
    let mut found = false;
    for (i, group) in groups
        .iter()
        .filter(|g| g.group_type == group_type)
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

/// Like [`cell_group_map`] but allows partial coverage. Cells not in any matching
/// group are `None`. Returns `None` if no groups of the given type exist at all.
fn partial_cell_group_map(
    groups: &CellGroups,
    group_type: CellGroupType,
) -> Option<[Option<u8>; 81]> {
    let mut map: [Option<u8>; 81] = [None; 81];
    let mut found = false;
    for (i, group) in groups
        .iter()
        .filter(|g| g.group_type == group_type)
        .enumerate()
    {
        found = true;
        for index in group.iter_indexes() {
            map[*index as usize] = Some(i as u8);
        }
    }
    found.then_some(map)
}

/// Nonomino-specific: full Custom group coverage required.
fn custom_region_map(groups: &CellGroups) -> Option<[u8; 81]> {
    cell_group_map(groups, CellGroupType::Custom)
}

/// Returns the stroke style of the border segment between cells (ax,ay) and (bx,by).
///
/// Solid borders come from `primary` differences or grid boundaries. Dashed borders
/// come from `secondary` differences when there is no primary border. Out-of-bounds
/// coords on both sides return [`Stroke::None`]; mixed in/out returns [`Stroke::Solid`].
fn segment_stroke(
    primary: &Option<[u8; 81]>,
    secondary: &Option<[Option<u8>; 81]>,
    ax: i32,
    ay: i32,
    bx: i32,
    by: i32,
) -> Stroke {
    let in_bounds = |x: i32, y: i32| (0..9).contains(&x) && (0..9).contains(&y);
    let a_valid = in_bounds(ax, ay);
    let b_valid = in_bounds(bx, by);
    if !a_valid && !b_valid {
        return Stroke::None;
    }
    if a_valid != b_valid {
        return Stroke::Solid; // grid boundary
    }
    let ai = (ay * 9 + ax) as usize;
    let bi = (by * 9 + bx) as usize;
    let primary_diff = match primary {
        None => true,
        Some(m) => m[ai] != m[bi],
    };
    if primary_diff {
        return Stroke::Solid;
    }
    if let Some(s) = secondary {
        if s[ai] != s[bi] {
            return Stroke::Dashed;
        }
    }
    Stroke::None
}

/// Returns the box-drawing character for a junction at (jx, jy).
fn junction_char_for(
    primary: &Option<[u8; 81]>,
    secondary: &Option<[Option<u8>; 81]>,
    jx: i32,
    jy: i32,
) -> char {
    let n_s = segment_stroke(primary, secondary, jx - 1, jy - 1, jx, jy - 1);
    let s_s = segment_stroke(primary, secondary, jx - 1, jy, jx, jy);
    let w_s = segment_stroke(primary, secondary, jx - 1, jy - 1, jx - 1, jy);
    let e_s = segment_stroke(primary, secondary, jx, jy - 1, jx, jy);
    let n = n_s.present();
    let s = s_s.present();
    let w = w_s.present();
    let e = e_s.present();
    let any_solid = matches!(n_s, Stroke::Solid)
        || matches!(s_s, Stroke::Solid)
        || matches!(w_s, Stroke::Solid)
        || matches!(e_s, Stroke::Solid);
    match (n, s, w, e) {
        (true, true, true, true) => '┼',
        (true, true, true, false) => '┤',
        (true, true, false, true) => '├',
        // Pure-dashed vertical pass-through uses dashed glyph; mixed/solid uses '│'.
        (true, true, false, false) if !any_solid => '┆',
        (true, true, false, false) => '│',
        (true, false, true, true) => '┴',
        (true, false, true, false) => '┘',
        (true, false, false, true) => '└',
        (true, false, false, false) => '╵',
        (false, true, true, true) => '┬',
        (false, true, true, false) => '┐',
        (false, true, false, true) => '┌',
        (false, true, false, false) => '╷',
        // Pure-dashed horizontal pass-through uses dashed glyph; mixed/solid uses '─'.
        (false, false, true, true) if !any_solid => '┄',
        (false, false, true, true) => '─',
        (false, false, true, false) => '╴',
        (false, false, false, true) => '╶',
        (false, false, false, false) => ' ',
    }
}

fn east_segment_str(stroke: Stroke) -> &'static str {
    match stroke {
        Stroke::Solid => "───",
        Stroke::Dashed => "┄┄┄",
        Stroke::None => "   ",
    }
}

fn vertical_segment_char(stroke: Stroke) -> char {
    match stroke {
        Stroke::Solid => '│',
        Stroke::Dashed => '┆',
        Stroke::None => ' ',
    }
}

/// Renders a 9×9 grid using `primary` (solid) and optional `secondary` (dashed) borders.
/// Junction rows with no horizontal segments are omitted for compact output.
/// `cell_content(x, y)` must return exactly 3 chars (e.g. `" 5 "` or `" A "`).
fn print_grid_with_strokes<F>(
    primary: &Option<[u8; 81]>,
    secondary: &Option<[Option<u8>; 81]>,
    cell_content: F,
) where
    F: Fn(u8, u8) -> String,
{
    let row_has_horizontal = |jy: i32| -> bool {
        (0i32..9).any(|jx| segment_stroke(primary, secondary, jx, jy - 1, jx, jy).present())
    };

    for jy in 0i32..=9 {
        if row_has_horizontal(jy) {
            for jx in 0i32..=9 {
                print!("{}", junction_char_for(primary, secondary, jx, jy));
                if jx < 9 {
                    let stroke = segment_stroke(primary, secondary, jx, jy - 1, jx, jy);
                    print!("{}", east_segment_str(stroke));
                }
            }
            println!();
        }
        if jy < 9 {
            let y = jy as u8;
            for x in 0u8..9 {
                let stroke = segment_stroke(
                    primary,
                    secondary,
                    x as i32 - 1,
                    y as i32,
                    x as i32,
                    y as i32,
                );
                print!("{}", vertical_segment_char(stroke));
                print!("{}", cell_content(x, y));
            }
            let right_stroke = segment_stroke(primary, secondary, 8, jy, 9, jy);
            println!("{}", vertical_segment_char(right_stroke));
        }
    }
    std::io::stdout().flush().unwrap();
}

/// Prints a 9×9 solution grid with borders drawn at primary-group boundaries.
///
/// Border selection rules:
/// - Nonomino (complete Custom group coverage): borders follow Custom regions.
/// - Standard / Hypersudoku: borders follow the 3×3 StandardBlock groups.
/// - Hypersudoku additionally overlays Custom (window) borders as dashed lines.
pub fn print_solution_with_regions(state: &GameState, groups: &CellGroups) {
    let custom_full = cell_group_map(groups, CellGroupType::Custom);
    let block_full = cell_group_map(groups, CellGroupType::StandardBlock);
    let primary = custom_full.or(block_full);
    // Secondary overlay: Custom groups (e.g. hypersudoku windows) only when the
    // primary outline already came from blocks, so we don't double-render nonomino.
    let secondary = if cell_group_map(groups, CellGroupType::Custom).is_some() {
        None
    } else {
        partial_cell_group_map(groups, CellGroupType::Custom)
    };
    print_grid_with_strokes(&primary, &secondary, |x, y| {
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
    print_grid_with_strokes(&map, &None, |x, y| {
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
    fn print_solution_with_regions_runs_for_standard_groups() {
        let game = example_sudoku();
        // Standard groups: rendering uses 3×3 block outlines from StandardBlock groups.
        print_solution_with_regions(&game.initial_state, &game.groups);
        let solver = DefaultSolver::new(&game.groups);
        let solved = solver
            .solve(&game.initial_state)
            .expect("example sudoku should be solvable");
        print_solution_with_regions(&solved, &game.groups);
    }

    #[test]
    fn print_solution_with_regions_runs_for_hypersudoku_groups() {
        // Hypersudoku: block outlines as primary, window outlines as dashed overlay.
        let groups = CellGroups::default()
            .with_hypersudoku_windows()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns();
        let state = GameState::new();
        print_solution_with_regions(&state, &groups);
    }

    #[test]
    fn cell_group_map_standard_blocks_returns_some() {
        let groups = CellGroups::default()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns();
        assert!(cell_group_map(&groups, CellGroupType::StandardBlock).is_some());
        assert!(cell_group_map(&groups, CellGroupType::Custom).is_none());
    }

    #[test]
    fn partial_cell_group_map_hypersudoku_returns_partial_coverage() {
        let groups = CellGroups::default()
            .with_hypersudoku_windows()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns();
        let partial = partial_cell_group_map(&groups, CellGroupType::Custom)
            .expect("hypersudoku has Custom groups");
        // 4 windows × 9 cells = 36 cells covered.
        let covered = partial.iter().filter(|c| c.is_some()).count();
        assert_eq!(covered, 36);
        // The remaining 45 cells are not in any window.
        let uncovered = partial.iter().filter(|c| c.is_none()).count();
        assert_eq!(uncovered, 45);
    }

    #[test]
    fn segment_stroke_grid_boundary_is_solid() {
        let stroke = segment_stroke(&None, &None, -1, 0, 0, 0);
        assert_eq!(stroke, Stroke::Solid);
    }

    #[test]
    fn segment_stroke_same_primary_no_secondary_is_none() {
        let map: [u8; 81] = [0; 81];
        let stroke = segment_stroke(&Some(map), &None, 0, 0, 1, 0);
        assert_eq!(stroke, Stroke::None);
    }

    #[test]
    fn segment_stroke_same_primary_diff_secondary_is_dashed() {
        let primary: [u8; 81] = [0; 81];
        let mut secondary: [Option<u8>; 81] = [None; 81];
        secondary[0] = Some(0);
        secondary[1] = None;
        let stroke = segment_stroke(&Some(primary), &Some(secondary), 0, 0, 1, 0);
        assert_eq!(stroke, Stroke::Dashed);
    }

    #[test]
    fn segment_stroke_diff_primary_overrides_secondary() {
        let mut primary: [u8; 81] = [0; 81];
        primary[1] = 1;
        let secondary: [Option<u8>; 81] = [None; 81];
        let stroke = segment_stroke(&Some(primary), &Some(secondary), 0, 0, 1, 0);
        assert_eq!(stroke, Stroke::Solid);
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
