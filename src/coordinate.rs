use crate::index::Index;
use std::fmt::{Debug, Formatter};

/// A classical Sudoku coordinate, ranging 0..=8 on each axis for 9x9 cells.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Coordinate {
    pub x: u8,
    pub y: u8,
}

impl Coordinate {
    pub const GAME_WIDTH: u8 = 9;
    pub const GAME_HEIGHT: u8 = 9;

    pub const fn new(x: u8, y: u8) -> Self {
        debug_assert!(x < Self::GAME_WIDTH && y < Self::GAME_HEIGHT);
        Self { x, y }
    }
}

impl Debug for Coordinate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let index: Index = self.clone().into();
        write!(
            f,
            "{x} × {y} (index {index})",
            x = self.x,
            y = self.y,
            index = *index
        )
    }
}

impl Into<Index> for Coordinate {
    fn into(self) -> Index {
        Index::new(self.y * Coordinate::GAME_WIDTH + self.x)
    }
}

impl Into<Coordinate> for Index {
    fn into(self) -> Coordinate {
        let x = (*self) % Coordinate::GAME_WIDTH;
        let y = (*self) / Coordinate::GAME_WIDTH;
        Coordinate { x, y }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn default_coordinate_to_index() {
        let coordinate = Coordinate::default();
        let index: Index = coordinate.into();
        assert_eq!(index, Index::default());
        assert_eq!(format!("{:?}", index), String::from("0 × 0 (index 0)"));
    }

    #[test]
    fn coordinate_to_index() {
        let coordinate = Coordinate::new(8, 8);
        let index: Index = coordinate.into();
        assert_eq!(index, Index::new(80));
        assert_eq!(format!("{:?}", index), String::from("8 × 8 (index 80)"));
    }
}
