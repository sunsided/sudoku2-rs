use crate::cell_group::{CellGroupType, CellGroups};
use crate::game_state::GameState;
use crate::index::Index;
use crate::value::Value;

/// Errors returned when parsing a puzzle string fails.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ParseError {
    /// The input did not contain exactly 81 puzzle cells.
    #[error("invalid puzzle length: expected 81 cells, found {found}")]
    InvalidLength { found: usize },

    /// A character that is neither a digit (`0`-`9`) nor a dot (`.`) was found
    /// at a position that is not a recognized separator.
    #[error("invalid character '{character}' at byte offset {offset}")]
    InvalidCharacter { offset: usize, character: char },
}

/// Parses and formats standard Sudoku puzzles in common text representations.
///
/// Two formats are supported:
///
/// - **Line format**: an 81-character string where `1`-`9` are clues and `.`
///   or `0` marks an empty cell.
/// - **Grid format**: the same encoding spread over multiple lines, with
///   optional separator characters (`|`, `-`, `+`) and whitespace between cells.
///
/// Both formats represent standard 9×9 Sudoku only (rows + columns + 3×3 blocks).
/// Variants with non-standard regions (nonomino, hypersudoku) require additional
/// region metadata and are outside the scope of this serializer.
pub struct SudokuSerializer;

impl SudokuSerializer {
    /// Parses an 81-character line into a [`GameState`].
    ///
    /// Digits `1`-`9` are placed as clues; `.` or `0` leaves the cell with all
    /// nine candidates.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::InvalidLength`] if `s` does not contain exactly 81
    /// characters, or [`ParseError::InvalidCharacter`] for any character that is
    /// not `0`-`9` or `.`.
    pub fn parse_line(s: &str) -> Result<GameState, ParseError> {
        let found = s.chars().count();
        if found != 81 {
            return Err(ParseError::InvalidLength { found });
        }
        Self::build_state(s.char_indices(), |c| !matches!(c, '.' | '0'..='9'))
    }

    /// Parses a multi-line grid string into a [`GameState`].
    ///
    /// Unicode whitespace (as determined by [`char::is_whitespace`]) and the
    /// separator characters `|`, `-`, `+` are skipped. All remaining characters
    /// must be `0`-`9` or `.`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::InvalidLength`] if the grid does not contain
    /// exactly 81 cells after stripping separators, or
    /// [`ParseError::InvalidCharacter`] for any unexpected character.
    pub fn parse_grid(s: &str) -> Result<GameState, ParseError> {
        let filtered = s
            .char_indices()
            .filter(|(_, c)| !c.is_whitespace() && !matches!(c, '|' | '-' | '+'));
        Self::build_state(filtered, |c| !matches!(c, '.' | '0'..='9'))
    }

    /// Formats a [`GameState`] as an 81-character line.
    ///
    /// Solved cells (exactly one candidate) are written as their digit; all
    /// other cells are written as `.`.
    pub fn format_line(state: &GameState) -> String {
        let mut s = String::with_capacity(81);
        for index in Index::range() {
            let cell = state.get_at_index(index);
            if cell.is_solved() {
                let v: u8 = cell.iter_candidates().next().unwrap().into();
                s.push((b'0' + v) as char);
            } else {
                s.push('.');
            }
        }
        s
    }

    /// Formats a [`GameState`] as nine lines of nine characters each.
    ///
    /// Each row occupies one line (no separator characters). Each cell follows
    /// the same encoding as [`format_line`]: digit for a solved cell, `.` for
    /// an unsolved one.
    pub fn format_grid(state: &GameState) -> String {
        let line = Self::format_line(state);
        let mut result = String::with_capacity(9 * 10);
        for row in 0..9 {
            result.push_str(&line[row * 9..(row + 1) * 9]);
            result.push('\n');
        }
        result
    }

    /// Formats the Custom group layout as an 81-character string where each cell
    /// is encoded as `A`–`I` for its Custom group index (0→A, 1→B, …, 8→I).
    ///
    /// Returns `None` if `groups` contains no Custom groups or if Custom groups
    /// do not cover all 81 cells (e.g. for standard or hypersudoku puzzles).
    /// For nonomino puzzles, returns `Some` with exactly 81 characters.
    pub fn format_region_line(groups: &CellGroups) -> Option<String> {
        let mut map = [u8::MAX; 81];
        let mut found = false;
        for (i, group) in groups
            .iter()
            .filter(|g| g.group_type == CellGroupType::Custom)
            .enumerate()
        {
            found = true;
            let letter = b'A' + i as u8;
            for index in group.iter_indexes() {
                map[*index as usize] = letter;
            }
        }
        if !found || map.contains(&u8::MAX) {
            return None;
        }
        Some(String::from_utf8(map.to_vec()).expect("A-I are valid UTF-8"))
    }

    fn build_state(
        iter: impl Iterator<Item = (usize, char)>,
        is_invalid: impl Fn(char) -> bool,
    ) -> Result<GameState, ParseError> {
        let state = GameState::new();
        let mut count = 0usize;

        for (offset, ch) in iter {
            // Check count first so that an over-long input produces InvalidLength
            // rather than InvalidCharacter when the surplus character happens to
            // be invalid.
            if count >= 81 {
                count += 1;
                continue;
            }
            if is_invalid(ch) {
                return Err(ParseError::InvalidCharacter {
                    offset,
                    character: ch,
                });
            }
            if let Some(digit) = ch.to_digit(10).filter(|&d| d > 0) {
                let index = Index::new(count as u8);
                let value = Value::try_from(digit as u8).unwrap();
                state.set_at_index(index, value);
            }
            count += 1;
        }

        if count != 81 {
            return Err(ParseError::InvalidLength { found: count });
        }

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Wikipedia Sudoku puzzle in canonical dot form (empty cells → '.').
    const WIKI_LINE: &str =
        ".28..7....16.83.7.....2.85113729.......73........463.729..7.......86.14....3..7..";
    // Same puzzle with '0' instead of '.' — accepted as input, normalized on output.
    const WIKI_LINE_ZEROS: &str =
        "028007000016083070000020851137290000000730000000046307290070000000860140000300700";

    #[test]
    fn parse_line_round_trips() {
        let state = SudokuSerializer::parse_line(WIKI_LINE).unwrap();
        let formatted = SudokuSerializer::format_line(&state);
        assert_eq!(formatted, WIKI_LINE);
    }

    #[test]
    fn parse_grid_round_trips() {
        let grid = SudokuSerializer::format_grid(&SudokuSerializer::parse_line(WIKI_LINE).unwrap());
        let state2 = SudokuSerializer::parse_grid(&grid).unwrap();
        assert_eq!(SudokuSerializer::format_line(&state2), WIKI_LINE);
    }

    #[test]
    fn parse_grid_with_separators() {
        let grid_with_seps = "\
.28|..7|...\n\
.16|.83|.7.\n\
...|.2.|851\n\
---+---+---\n\
137|29.|...\n\
...|73.|...\n\
...|.46|3.7\n\
---+---+---\n\
29.|.7.|...\n\
...|86.|14.\n\
...|3..|7..\n";
        let state = SudokuSerializer::parse_grid(grid_with_seps).unwrap();
        assert_eq!(SudokuSerializer::format_line(&state), WIKI_LINE);
    }

    #[test]
    fn parse_line_accepts_zeros_as_empty() {
        let state = SudokuSerializer::parse_line(WIKI_LINE_ZEROS).unwrap();
        assert_eq!(SudokuSerializer::format_line(&state), WIKI_LINE);
    }

    #[test]
    fn parse_line_error_on_wrong_length() {
        assert_eq!(
            SudokuSerializer::parse_line("123").unwrap_err(),
            ParseError::InvalidLength { found: 3 }
        );
        let too_long: String = "1".repeat(82);
        assert_eq!(
            SudokuSerializer::parse_line(&too_long).unwrap_err(),
            ParseError::InvalidLength { found: 82 }
        );
    }

    #[test]
    fn parse_line_error_on_invalid_char() {
        let bad: String = format!("{}X{}", "0".repeat(40), "0".repeat(40));
        assert!(matches!(
            SudokuSerializer::parse_line(&bad).unwrap_err(),
            ParseError::InvalidCharacter { character: 'X', .. }
        ));
    }

    #[test]
    fn parse_grid_error_on_invalid_char() {
        let bad = "028007000\n016083070\n000020851\n137290000\n000730000\n000046307\n290070000\n000860140\n0003007?0";
        assert!(matches!(
            SudokuSerializer::parse_grid(bad).unwrap_err(),
            ParseError::InvalidCharacter { character: '?', .. }
        ));
    }

    #[test]
    fn parse_grid_error_on_wrong_length() {
        let short = "530070000\n600195000\n098000060\n800060003\n400803001\n700020006\n060000280\n000419005\n";
        assert_eq!(
            SudokuSerializer::parse_grid(short).unwrap_err(),
            ParseError::InvalidLength { found: 72 }
        );
        let long = "530070000\n600195000\n098000060\n800060003\n400803001\n700020006\n060000280\n000419005\n000080079\n123456789\n";
        assert_eq!(
            SudokuSerializer::parse_grid(long).unwrap_err(),
            ParseError::InvalidLength { found: 90 }
        );
    }

    #[test]
    fn format_grid_has_nine_lines() {
        let state = SudokuSerializer::parse_line(WIKI_LINE).unwrap();
        let grid = SudokuSerializer::format_grid(&state);
        let lines: Vec<&str> = grid.lines().collect();
        assert_eq!(lines.len(), 9);
        for line in &lines {
            assert_eq!(line.len(), 9);
        }
    }

    #[test]
    fn empty_board_formats_as_dots() {
        let state = GameState::new();
        let line = SudokuSerializer::format_line(&state);
        assert_eq!(line, ".".repeat(81));
    }

    #[test]
    fn format_region_line_standard_returns_none() {
        let groups = crate::CellGroups::default()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns();
        assert!(SudokuSerializer::format_region_line(&groups).is_none());
    }

    #[test]
    fn format_region_line_nonomino_returns_81_char_a_to_i_string() {
        use crate::generator::NonominoRegionGenerator;
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(1);
        let groups = loop {
            if let Some(g) = NonominoRegionGenerator::default().generate(&mut rng) {
                break g;
            }
        };
        let line = SudokuSerializer::format_region_line(&groups)
            .expect("nonomino must have complete custom coverage");
        assert_eq!(line.len(), 81);
        assert!(line.chars().all(|c| ('A'..='I').contains(&c)));
        // Each letter appears exactly 9 times (9 regions × 9 cells)
        for ch in b'A'..=b'I' {
            let count = line.chars().filter(|&c| c == ch as char).count();
            assert_eq!(
                count, 9,
                "region {} has {count} cells, expected 9",
                ch as char
            );
        }
    }

    #[test]
    fn format_region_line_hypersudoku_returns_none() {
        let groups = crate::CellGroups::default()
            .with_hypersudoku_windows()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns();
        assert!(SudokuSerializer::format_region_line(&groups).is_none());
    }

    #[test]
    fn fully_solved_board_formats_without_dots() {
        use crate::example_games::sudoku::example_sudoku;
        let game = example_sudoku();
        let solver = crate::DefaultSolver::new(&game.groups);
        let state = solver.solve(&game.initial_state).unwrap();
        let line = SudokuSerializer::format_line(&state);
        assert!(!line.contains('.'));
        assert_eq!(line.len(), 81);
    }
}
