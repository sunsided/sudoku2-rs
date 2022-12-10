use std::num::NonZeroU8;
use sudoku2::prelude::*;

fn main() {
    let state = GameState::new();

    state.set_at_xy(0, 0, GameCell::from_value(Value::ONE));

    print_game_state(&state);
}

fn print_game_state(state: &GameState) {
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
                    let value = Value::new(NonZeroU8::try_from(value).unwrap());
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
}
