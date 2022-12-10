use crate::index::{Index, IndexBitSet, IndexBitSetIter};
use crate::prelude::Coordinate;
use std::slice::Iter;

/// The set of all cell groups relevant to a game.
#[derive(Default, Debug, Clone)]
pub struct CellGroups {
    groups: Vec<CellGroup>,
}

impl CellGroups {
    pub fn with_group(mut self, group: CellGroup) -> Self {
        self.add_group(group);
        self
    }

    pub fn add_group(&mut self, group: CellGroup) -> &mut Self {
        self.groups.push(group);
        self
    }

    pub fn with_default_rows_and_columns(self) -> Self {
        self.with_default_rows().with_default_columns()
    }

    //noinspection DuplicatedCode
    fn with_default_rows(mut self) -> Self {
        let mut check = IndexBitSet::ALL;
        let mut ids = self
            .groups
            .iter()
            .flat_map(|x| x.id)
            .max()
            .unwrap_or_default();

        for y in 0..9 {
            let mut group = CellGroup::new(ids, CellGroupType::StandardRow);
            ids += 1;
            for x in 0..9 {
                let coord = Coordinate::new(x, y).into();
                group.add_index(coord);
                check.remove(coord);
            }

            self.add_group(group);
        }
        self
    }

    //noinspection DuplicatedCode
    fn with_default_columns(mut self) -> Self {
        let mut check = IndexBitSet::ALL;
        let mut ids = self
            .groups
            .iter()
            .flat_map(|x| x.id)
            .max()
            .unwrap_or_default();

        for x in 0..9 {
            let mut group = CellGroup::new(ids, CellGroupType::StandardColumn);
            ids += 1;
            for y in 0..9 {
                let coord = Coordinate::new(x, y).into();
                group.add_index(coord);
                check.remove(coord);
            }

            self.add_group(group);
        }
        self
    }

    pub fn with_default_sudoku_blocks(mut self) -> Self {
        let mut check = IndexBitSet::ALL;
        let mut ids = self
            .groups
            .iter()
            .flat_map(|x| x.id)
            .max()
            .unwrap_or_default();

        for y in (0..9).step_by(3) {
            for x in (0..9).step_by(3) {
                let mut group = CellGroup::new(ids, CellGroupType::StandardBlock);
                ids += 1;

                for row in 0..3 {
                    for col in 0..3 {
                        let coord = Coordinate::new(x + col, y + row).into();
                        group.add_index(coord);
                        check.remove(coord);
                    }
                }

                self.add_group(group);
            }
        }

        debug_assert!(check.is_empty());
        self
    }

    #[inline]
    pub fn get_at_xy(&self, x: u8, y: u8) -> Result<IndexBitSet, NoMatchingGroup> {
        debug_assert!(x <= 9 && y <= 9);
        self.get_at_coord(Coordinate::new(x, y))
    }

    #[inline]
    pub fn get_at_coord(&self, coord: Coordinate) -> Result<IndexBitSet, NoMatchingGroup> {
        self.get_at_index(coord.into())
    }

    pub fn get_at_index(&self, index: Index) -> Result<IndexBitSet, NoMatchingGroup> {
        let mut set = IndexBitSet::empty();

        for group in self.groups.iter() {
            if group.contains(index) {
                set.union(&group.indexes);
            }
        }

        if set.is_empty() {
            Err(NoMatchingGroup {})
        } else {
            Ok(set)
        }
    }

    #[inline]
    pub fn get_groups_at_xy(&self, x: u8, y: u8) -> Result<Vec<CellGroup>, NoMatchingGroup> {
        debug_assert!(x <= 9 && y <= 9);
        self.get_groups_at_coord(Coordinate::new(x, y))
    }

    #[inline]
    pub fn get_groups_at_coord(
        &self,
        coord: Coordinate,
    ) -> Result<Vec<CellGroup>, NoMatchingGroup> {
        self.get_groups_at_index(coord.into())
    }

    pub fn get_groups_at_index(&self, index: Index) -> Result<Vec<CellGroup>, NoMatchingGroup> {
        let mut set = Vec::default();

        for group in self.groups.iter() {
            if group.contains(index) {
                set.push(group.clone());
            }
        }

        if set.is_empty() {
            Err(NoMatchingGroup {})
        } else {
            Ok(set)
        }
    }

    pub fn iter(&self) -> Iter<'_, CellGroup> {
        self.groups.iter()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("The specified group overlaps with an already existing group")]
pub struct OverlappingGroups {}

#[derive(Debug, thiserror::Error)]
#[error("No matching group was found")]
pub struct NoMatchingGroup {}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CellGroupType {
    Custom,
    StandardBlock,
    StandardRow,
    StandardColumn,
}

impl Default for CellGroupType {
    fn default() -> Self {
        CellGroupType::Custom
    }
}

/// A group of related indexes, e.g. a row, a column, ...
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct CellGroup {
    /// The internal ID of this group.
    pub id: Option<usize>,
    /// The type of the group
    pub group_type: CellGroupType,
    /// The indexes for this group.
    indexes: IndexBitSet,
}

impl CellGroup {
    pub fn new(id: usize, group_type: CellGroupType) -> Self {
        Self {
            id: Some(id),
            group_type,
            indexes: IndexBitSet::default(),
        }
    }

    pub const fn with_index(mut self, index: Index) -> Self {
        self.indexes = self.indexes.with_index(index);
        self
    }

    pub fn from_indexes<T: IntoIterator<Item = Index>>(indexes: T) -> Self {
        Self::default().with_indexes(indexes)
    }

    pub fn with_indexes<T: IntoIterator<Item = Index>>(mut self, indexes: T) -> Self {
        for index in indexes.into_iter() {
            self.indexes = self.indexes.with_index(index);
        }
        self
    }

    pub fn add_index(&mut self, index: Index) -> &mut Self {
        self.indexes = self.indexes.with_index(index);
        self
    }

    /// Determines whether this cell contains a specific value (possibly as a candidate).
    #[inline]
    pub const fn contains(&self, index: Index) -> bool {
        self.indexes.contains(index)
    }

    /// Gets the number of indexes for this group.
    #[inline]
    pub const fn len(&self) -> usize {
        self.indexes.len()
    }

    /// Determines whether this group has any indexes.
    #[inline]
    pub const fn empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterates all indexes for this cell group.
    #[inline]
    pub fn iter_indexes(&self) -> IndexBitSetIter {
        self.indexes.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Index;

    #[test]
    fn from_iter() {
        let cg = CellGroup::from_indexes([Index::new(0), Index::new(2), Index::new(3)]);
        assert!(cg.contains(Index::new(0)));
        assert!(cg.contains(Index::new(2)));
        assert!(cg.contains(Index::new(3)));
        assert!(!cg.contains(Index::new(1)));
    }

    //noinspection DuplicatedCode
    #[test]
    fn add_groups() {
        #[rustfmt::skip]
        let group_a = CellGroup::default().with_indexes([
            Index::new(0), Index::new(1), Index::new(2),
            Index::new(9), Index::new(10), Index::new(11),
            Index::new(18), Index::new(19), Index::new(20)]);

        #[rustfmt::skip]
        let group_b = CellGroup::default().with_indexes([
            Index::new(3), Index::new(4), Index::new(5),
            Index::new(12), Index::new(13), Index::new(14),
            Index::new(21), Index::new(22), Index::new(23)]);

        CellGroups::default().add_group(group_a).add_group(group_b);
    }
}
