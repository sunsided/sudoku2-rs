use crate::cell_group::CellGroups;
use crate::game_cell::GameCell;
use crate::index::{Index, IndexBitSet};
use crate::prelude::Coordinate;
use crate::value::{IntoValueOptions, Value, ValueBitSet};
use std::cell::Cell;
use std::mem::MaybeUninit;

#[derive(Debug, thiserror::Error)]
#[error("An invalid game state was reached")]
pub struct InvalidGameState {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GameState {
    cells: [Cell<GameCell>; 81],
}

impl AsRef<GameState> for &GameState {
    fn as_ref(&self) -> &GameState {
        self
    }
}

impl GameState {
    pub fn new() -> Self {
        let mut cells: [MaybeUninit<Cell<GameCell>>; 81] =
            unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..81 {
            cells[i].write(Cell::new(GameCell::default()));
        }
        Self {
            cells: unsafe { std::mem::transmute(cells) },
        }
    }

    pub fn new_from<S: IntoValueOptions>(values: S) -> Self {
        let mut cells: [MaybeUninit<Cell<GameCell>>; 81] =
            unsafe { MaybeUninit::uninit().assume_init() };

        let values = values.into();
        for i in 0..81 {
            match values[i] {
                Some(value) => cells[i].write(Cell::new(GameCell::from_value(value))),
                None => cells[i].write(Cell::new(GameCell::default())),
            };
        }
        Self {
            cells: unsafe { std::mem::transmute(cells) },
        }
    }

    #[inline]
    pub fn get_at_index(&self, index: Index) -> GameCell {
        self.cell_at_index(index).get()
    }

    #[inline]
    pub fn get_at_coord(&self, coord: Coordinate) -> GameCell {
        self.cell_at_coord(coord).get()
    }

    #[inline]
    pub fn get_at_xy(&self, x: u8, y: u8) -> GameCell {
        self.get_at_coord(Coordinate::new(x, y))
    }

    /// Places a value at the specified cell, propagating the changes through the board.
    #[inline]
    pub fn place_and_propagate_at_index(
        &self,
        index: Index,
        value: Value,
        groups: &CellGroups,
    ) -> &Self {
        self.set_at_index(index, value);

        let groups = groups
            .get_at_index(index, false)
            .expect("group does not exist at index");

        // Propagate changes through peers.
        for peer_index in groups.iter() {
            debug_assert_ne!(peer_index, index);

            let peer = self.cell_at_index(peer_index);
            peer.set(peer.get().without_value(value));
        }

        self
    }

    /// Places a value at the specified cell, propagating the changes through the board.
    #[inline]
    pub fn place_and_propagate_at_coord(
        &self,
        coord: Coordinate,
        value: Value,
        groups: &CellGroups,
    ) -> &Self {
        self.place_and_propagate_at_index(coord.into(), value, groups);
        self
    }

    /// Places a value at the specified cell, propagating the changes through the board.
    #[inline]
    pub fn place_and_propagate_at_xy(
        &self,
        x: u8,
        y: u8,
        value: Value,
        groups: &CellGroups,
    ) -> &Self {
        self.place_and_propagate_at_coord(Coordinate::new(x, y), value, groups);
        self
    }

    /// Places a value at the specified cell, but does not propagate the changes through the board.
    /// For making a proper move, use [`place_and_propagate_at_index`] instead.
    #[inline]
    pub fn set_at_index(&self, index: Index, value: Value) -> &Self {
        let cell = self.cell_at_index(index);

        #[cfg(debug_assertions)]
        {
            let test = cell.get();
            debug_assert!(
                !test.is_solved() || test.contains(value),
                "Attempted to overwrite solved cell at {index:?} with differing value: had {old:?}, instructed to write {new:?}",
                index = index,
                old = test.iter_candidates().next().unwrap(),
                new = value
            );
        }

        cell.set(GameCell::from_value(value));
        self
    }

    /// Places a value at the specified cell, but does not propagate the changes through the board.
    /// For making a proper move, use [`place_and_propagate_at_coord`] instead.
    #[inline]
    pub fn set_at_coord(&self, coord: Coordinate, value: Value) -> &Self {
        self.set_at_index(coord.into(), value);
        self
    }

    /// Places a value at the specified cell, but does not propagate the changes through the board.
    /// For making a proper move, use [`place_and_propagate_at_xy`] instead.
    #[inline]
    pub fn set_at_xy(&self, x: u8, y: u8, value: Value) -> &Self {
        self.set_at_coord(Coordinate::new(x, y), value);
        self
    }

    /// Forgets a value at the specified cell. No changes will be propagated,
    /// but the cell will be treated as if the value was never an option.
    #[inline]
    pub fn forget_at_index(&self, index: Index, value: Value) -> bool {
        let cell = self.cell_at_index(index);
        let gc = cell.get();
        if gc.contains(value) {
            cell.set(gc.without_value(value));
            true
        } else {
            false
        }
    }

    /// Forgets a value at the specified cell. No changes will be propagated,
    /// but the cell will be treated as if the value was never an option.
    #[inline]
    pub fn forget_many_at_index(&self, index: Index, values: &ValueBitSet) -> bool {
        let cell = self.cell_at_index(index);
        let gc = cell.get();
        if gc.contains_some(values) {
            cell.set(gc.without_values(values));
            true
        } else {
            false
        }
    }

    #[inline]
    fn cell_at_index(&self, index: Index) -> &Cell<GameCell> {
        debug_assert!((*index as usize) < self.cells.len());
        &self.cells[*index as usize]
    }

    #[inline]
    fn cell_at_coord(&self, coord: Coordinate) -> &Cell<GameCell> {
        self.cell_at_index(coord.into())
    }

    /// Determines if this board state is a valid solution.
    pub fn is_solved(&self, groups: &CellGroups) -> bool {
        // Naive check: Any cell with not exactly one remaining value
        // implies the board is either unsolved or invalid.
        for index in 0..81 {
            let index = Index::new(index);
            let cell = self.get_at_index(index);
            if !cell.is_solved() {
                return false;
            }
        }

        // Since we now know that all cells have exactly one value,
        // we can sanity check them.
        for index in 0..81 {
            let index = Index::new(index);
            let cell = self.get_at_index(index);
            let value = cell.iter_candidates().next().unwrap();

            let groups = groups
                .get_at_index(index, false)
                .expect("no groups found for specified index");
            for peer_index in groups.iter().filter(|x| *x > index) {
                let peer_cell = self.get_at_index(peer_index);
                let peer_value = peer_cell.iter_candidates().next().unwrap();
                if peer_value == value {
                    return false;
                }
            }
        }

        return true;
    }

    /// Determines if this board state is consistent (i.e. doesn't
    /// violate the game rules) but does not check for a proper solution.
    pub fn is_consistent(&self, groups: &CellGroups) -> bool {
        for index in 0..81 {
            let index = Index::new(index);
            let cell = self.get_at_index(index);
            if cell.is_impossible() {
                return false;
            }
        }

        // Ensure values appear only once.
        for index_under_test in 0..81 {
            let index_under_test = Index::new(index_under_test);
            let cell_under_test = self.get_at_index(index_under_test);

            // Consider only cells with exactly one value.
            // Zero-candidate cells are already ruled out.
            if !cell_under_test.is_solved() {
                continue;
            }

            let cell_under_test = cell_under_test.as_bitset();
            let mut seen_indexes = IndexBitSet::empty().with_index(index_under_test);

            let groups = groups.get_at_index(index_under_test, true).unwrap();
            for index in groups.iter() {
                // Only process the indexes once.
                if !seen_indexes.try_insert(index) {
                    continue;
                }

                let cell = self.get_at_index(index);

                // Consider only cells with exactly one value.
                // Zero-candidate cells are already ruled out.
                if !cell.is_solved() {
                    continue;
                }

                let cell_set = cell.as_bitset();
                if cell_under_test.contains_all(cell_set) {
                    return false;
                }
            }
        }

        // It's just a heuristic. :)
        return true;
    }
}

impl core::ops::Index<Index> for GameState {
    type Output = Cell<GameCell>;

    #[inline]
    fn index(&self, index: Index) -> &Self::Output {
        self.cell_at_index(index)
    }
}

impl core::ops::Index<Coordinate> for GameState {
    type Output = Cell<GameCell>;

    #[inline]
    fn index(&self, coord: Coordinate) -> &Self::Output {
        self.cell_at_coord(coord)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::Value;

    //noinspection DuplicatedCode
    #[test]
    #[rustfmt::skip]
    fn from_array() {
        let x = 0u8;
        let test_state = GameState::new_from([
            x, 2, 8,   x, x, 7,   x, x, x,
            x, 1, 6,   x, 8, 3,   x, 7, x,
            x, x, x,   x, 2, x,   8, 5, 1,

            1, 3, 7,   2, 9, x,   x, x, x,
            x, x, x,   7, 3, x,   x, x, x,
            x, x, x,   x, 4, 6,   3, x, 7,

            2, 9, x,   x, 7, x,   x, x, x,
            x, x, x,   8, 6, x,   1, 4, x,
            x, x, x,   3, x, x,   7, x, x,
        ]);

        let expected_state = GameState::new();
        expected_state.set_at_xy(1, 0, Value::TWO);
        expected_state.set_at_xy(2, 0, Value::EIGHT);
        expected_state.set_at_xy(5, 0, Value::SEVEN);

        expected_state.set_at_xy(1, 1, Value::ONE);
        expected_state.set_at_xy(2, 1, Value::SIX);
        expected_state.set_at_xy(4, 1, Value::EIGHT);
        expected_state.set_at_xy(5, 1, Value::THREE);
        expected_state.set_at_xy(7, 1, Value::SEVEN);

        expected_state.set_at_xy(4, 2, Value::TWO);
        expected_state.set_at_xy(6, 2, Value::EIGHT);
        expected_state.set_at_xy(7, 2, Value::FIVE);
        expected_state.set_at_xy(8, 2, Value::ONE);

        expected_state.set_at_xy(0, 3, Value::ONE);
        expected_state.set_at_xy(1, 3, Value::THREE);
        expected_state.set_at_xy(2, 3, Value::SEVEN);
        expected_state.set_at_xy(3, 3, Value::TWO);
        expected_state.set_at_xy(4, 3, Value::NINE);

        expected_state.set_at_xy(3, 4, Value::SEVEN);
        expected_state.set_at_xy(4, 4, Value::THREE);

        expected_state.set_at_xy(4, 5, Value::FOUR);
        expected_state.set_at_xy(5, 5, Value::SIX);
        expected_state.set_at_xy(6, 5, Value::THREE);
        expected_state.set_at_xy(8, 5, Value::SEVEN);

        expected_state.set_at_xy(0, 6, Value::TWO);
        expected_state.set_at_xy(1, 6, Value::NINE);
        expected_state.set_at_xy(4, 6, Value::SEVEN);

        expected_state.set_at_xy(3, 7, Value::EIGHT);
        expected_state.set_at_xy(4, 7, Value::SIX);
        expected_state.set_at_xy(6, 7, Value::ONE);
        expected_state.set_at_xy(7, 7, Value::FOUR);

        expected_state.set_at_xy(3, 8, Value::THREE);
        expected_state.set_at_xy(6, 8, Value::SEVEN);

        assert_eq!(expected_state, test_state);
    }
}
