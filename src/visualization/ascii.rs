use crate::prelude::*;
use std::io::Write;
use std::num::NonZeroU8;

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

pub fn print_cell_groups(groups: &CellGroups) {
    let mut group_names = Vec::default();
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

            let group = groups.get_groups_at_xy(x, y).expect("invalid groups");
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
