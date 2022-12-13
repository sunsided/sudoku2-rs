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

    #[inline]
    pub const fn into_index(self) -> Index {
        Index::new(self.y * Coordinate::GAME_WIDTH + self.x)
    }
}

impl Debug for Coordinate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let index = self.into_index();
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
    #[inline]
    fn into(self) -> Index {
        self.into_index()
    }
}

impl Into<Coordinate> for Index {
    #[inline]
    fn into(self) -> Coordinate {
        self.into_coordinate()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn default_coordinate_to_index() {
        let coordinate = Coordinate::default();
        let index = coordinate.into_index();
        assert_eq!(index, Index::default());
        assert_eq!(format!("{:?}", index), String::from("0 × 0 (index 0)"));
    }

    #[test]
    fn coordinate_to_index() {
        let coordinate = Coordinate::new(8, 8);
        let index = coordinate.into_index();
        assert_eq!(index, Index::new(80));
        assert_eq!(format!("{:?}", index), String::from("8 × 8 (index 80)"));
    }
}
