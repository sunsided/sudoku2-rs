use crate::cell_group::CellGroups;
use crate::game_cell::GameCell;
use crate::index::Index;
use crate::prelude::{CellGroup, Coordinate};
use crate::value::Value;
use std::cell::Cell;
use std::mem::MaybeUninit;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GameState {
    cells: [Cell<GameCell>; 81],
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
    pub fn place_at_index(&self, index: Index, value: Value, groups: &CellGroups) -> &Self {
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
        let groups = groups
            .get_at_index(index)
            .expect("group does not exist at index");

        // Propagate changes through peers.
        for group in groups.into_iter() {
            for peer_index in group.iter_indexes() {
                if index == peer_index {
                    continue;
                }

                let peer = self.cell_at_index(peer_index);
                let mut cell = peer.get();
                cell.remove(value);
                peer.set(cell);
            }
        }

        self
    }

    /// Places a value at the specified cell, propagating the changes through the board.
    #[inline]
    pub fn place_at_coord(&self, coord: Coordinate, value: Value, groups: &CellGroups) -> &Self {
        self.place_at_index(coord.into(), value, groups);
        self
    }

    /// Places a value at the specified cell, propagating the changes through the board.
    #[inline]
    pub fn place_at_xy(&self, x: u8, y: u8, value: Value, groups: &CellGroups) -> &Self {
        self.place_at_coord(Coordinate::new(x, y), value, groups);
        self
    }

    /// This method simply sets the value of a cell at the specified coordinates.
    /// It does not propagate the changes through the board.
    #[inline]
    fn set_at_index(&self, index: Index, cell: GameCell) -> &Self {
        self.cell_at_index(index).set(cell);
        self
    }

    /// This method simply sets the value of a cell at the specified coordinates.
    /// It does not propagate the changes through the board.
    #[inline]
    fn set_at_coord(&self, coord: Coordinate, cell: GameCell) -> &Self {
        self.cell_at_coord(coord).set(cell);
        self
    }

    /// This method simply sets the value of a cell at the specified coordinates.
    /// It does not propagate the changes through the board.
    #[inline]
    fn set_at_xy(&self, x: u8, y: u8, cell: GameCell) -> &Self {
        self.set_at_coord(Coordinate::new(x, y), cell);
        self
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
                .get_at_index(index)
                .expect("no groups found for specified index");
            for group in groups.into_iter() {
                for peer_index in group.iter_indexes().filter(|x| *x > index) {
                    let peer_cell = self.get_at_index(peer_index);
                    let peer_value = peer_cell.iter_candidates().next().unwrap();
                    if peer_value == value {
                        return false;
                    }
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

        todo!()
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
