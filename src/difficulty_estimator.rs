use crate::board_stats::BoardStatsCache;
use crate::cell_group::CellGroups;
use crate::game_state::GameState;
use crate::strategies::{
    HPattern, HiddenQuads, HiddenSingles, HiddenTriples, HiddenTwins, NakedQuads, NakedSingles,
    NakedTriples, NakedTwins, Skyscraper, Strategy, StrategyResult, UniqueRectangle, WWing, XWing,
    XYWing,
};

/// Difficulty of a Sudoku puzzle, determined by the hardest technique required
/// to solve it without backtracking.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Difficulty {
    /// Solvable with naked/hidden singles only.
    Easy,
    /// Requires naked/hidden twins or box-line reductions.
    Medium,
    /// Requires naked/hidden triples, quads, X-Wing, or Skyscraper.
    Hard,
    /// Requires XY-Wing, W-Wing, or Unique Rectangle.
    Expert,
    /// No pure-logic path found; backtracking required.
    Extreme,
}

/// Estimates the difficulty of `state` by running the strategy hierarchy and
/// recording the hardest technique that fires. Returns [`Difficulty::Extreme`]
/// when the puzzle cannot be solved by any registered strategy without
/// backtracking.
///
/// # Precondition
///
/// `state` must be a valid, uniquely-solvable puzzle. If a strategy reports an
/// inconsistency (`InvalidGameState`) the loop terminates early, and the result
/// is [`Difficulty::Extreme`] — the same value returned for puzzles that
/// genuinely require backtracking. Contradictory inputs are not distinguished
/// from puzzles that exhaust the strategy set; callers that need to tell them
/// apart should validate the puzzle first (e.g. via `DefaultSolver::is_unique`).
///
/// `state` is not modified.
pub fn estimate_difficulty(state: &GameState, groups: &CellGroups) -> Difficulty {
    let state = state.clone();
    let stats = BoardStatsCache::new(&state);

    // Strategies ordered Easy → Expert, mirroring DefaultSolver's pipeline.
    // UniqueRectangle is enabled because we are estimating a puzzle that is
    // expected to have a unique solution.
    let strategies: Vec<(Box<dyn Strategy>, Difficulty)> = vec![
        (NakedSingles::new_box(), Difficulty::Easy),
        (HiddenSingles::new_box(true), Difficulty::Easy),
        (NakedTwins::new_box(true), Difficulty::Medium),
        (HiddenTwins::new_box(true), Difficulty::Medium),
        (HPattern::new_box(true), Difficulty::Medium),
        (NakedTriples::new_box(true), Difficulty::Hard),
        (HiddenTriples::new_box(true), Difficulty::Hard),
        (NakedQuads::new_box(true), Difficulty::Hard),
        (HiddenQuads::new_box(true), Difficulty::Hard),
        (Skyscraper::new_box(true), Difficulty::Hard),
        (XWing::new_box(true), Difficulty::Hard),
        (XYWing::new_box(true), Difficulty::Expert),
        (WWing::new_box(true), Difficulty::Expert),
        (UniqueRectangle::new_box(true), Difficulty::Expert),
    ];

    let mut max_tier = Difficulty::Easy;

    'solving: loop {
        for (strategy, tier) in &strategies {
            if !strategy.is_enabled() {
                continue;
            }
            match strategy.apply(&state, groups, &stats) {
                Err(_) => break 'solving,
                Ok(StrategyResult::NoChange) => continue,
                Ok(StrategyResult::AppliedChange) => {
                    stats.invalidate();
                    if *tier > max_tier {
                        max_tier = *tier;
                    }
                    if strategy.always_continue() {
                        // Strategy applies all its moves in one pass; continue
                        // through the pipeline rather than restarting.
                        continue;
                    }
                    if state.is_solved(groups) {
                        return max_tier;
                    }
                    continue 'solving;
                }
            }
        }
        // No strategy made progress.
        break;
    }

    if state.is_solved(groups) {
        max_tier
    } else {
        Difficulty::Extreme
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell_group::CellGroups;

    fn standard_groups() -> CellGroups {
        CellGroups::default()
            .with_default_sudoku_blocks()
            .with_default_rows_and_columns()
    }

    #[test]
    fn simple_puzzle_classified_as_at_most_medium() {
        let game = crate::example_games::sudoku::example_sudoku();
        let difficulty = estimate_difficulty(&game.initial_state, &game.groups);
        assert!(
            difficulty <= Difficulty::Medium,
            "Expected at most Medium, got {difficulty:?}"
        );
    }

    #[test]
    fn skyscraper_puzzle_classified_as_expert() {
        // Skyscraper (Hard) fires and then UniqueRectangle (Expert) fires too,
        // so the overall classification lands at Expert.
        let game = crate::example_games::sudoku_skyscraper::example_sudoku();
        let difficulty = estimate_difficulty(&game.initial_state, &game.groups);
        assert_eq!(difficulty, Difficulty::Expert);
    }

    #[test]
    fn xwing_puzzle_classified_as_expert() {
        // X-Wing (Hard) fires and then UniqueRectangle (Expert) fires too.
        let game = crate::example_games::sudoku_xwings::example_sudoku();
        let difficulty = estimate_difficulty(&game.initial_state, &game.groups);
        assert_eq!(difficulty, Difficulty::Expert);
    }

    #[test]
    fn xy_wing_puzzle_solvable_without_backtracking() {
        // Norvig hard1: the doc notes XY-Wing fires before branching, but with
        // HPattern (box-line reduction) enabled the puzzle resolves at Medium
        // without ever reaching XY-Wing or needing backtracking.
        let game = crate::example_games::sudoku_xy_wing::example_sudoku();
        let difficulty = estimate_difficulty(&game.initial_state, &game.groups);
        assert!(
            difficulty >= Difficulty::Medium && difficulty < Difficulty::Extreme,
            "Expected Medium..Expert, got {difficulty:?}"
        );
    }

    #[test]
    fn w_wing_puzzle_classified_as_extreme() {
        // No registered strategy can solve this without backtracking.
        let game = crate::example_games::sudoku_w_wing::example_sudoku();
        let difficulty = estimate_difficulty(&game.initial_state, &game.groups);
        assert_eq!(difficulty, Difficulty::Extreme);
    }

    #[test]
    fn hardest_puzzle_classified_as_expert() {
        // Requires Expert techniques (UniqueRectangle fires to finish the solve).
        let game = crate::example_games::sudoku2::example_sudoku_hardest();
        let difficulty = estimate_difficulty(&game.initial_state, &game.groups);
        assert_eq!(difficulty, Difficulty::Expert);
    }

    #[test]
    fn difficulty_ord_ordering_is_correct() {
        assert!(Difficulty::Easy < Difficulty::Medium);
        assert!(Difficulty::Medium < Difficulty::Hard);
        assert!(Difficulty::Hard < Difficulty::Expert);
        assert!(Difficulty::Expert < Difficulty::Extreme);
    }

    #[test]
    fn fully_solved_state_returns_easy() {
        let groups = standard_groups();
        #[rustfmt::skip]
        let state = crate::GameState::new_from([
            5u8, 3, 4,  6, 7, 8,  9, 1, 2,
              6, 7, 2,  1, 9, 5,  3, 4, 8,
              1, 9, 8,  3, 4, 2,  5, 6, 7,

              8, 5, 9,  7, 6, 1,  4, 2, 3,
              4, 2, 6,  8, 5, 3,  7, 9, 1,
              7, 1, 3,  9, 2, 4,  8, 5, 6,

              9, 6, 1,  5, 3, 7,  2, 8, 4,
              2, 8, 7,  4, 1, 9,  6, 3, 5,
              3, 4, 5,  2, 8, 6,  1, 7, 9,
        ]);
        let difficulty = estimate_difficulty(&state, &groups);
        assert_eq!(difficulty, Difficulty::Easy);
    }
}
