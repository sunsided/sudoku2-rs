use crate::index::{Index, IndexBitSet, IntoIndexBitSetIter};
use crate::Coordinate;
use std::fmt::{Debug, Formatter};
use std::slice::Iter;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum CellGroupType {
    Custom = 0,
    StandardBlock = 1,
    StandardRow = 2,
    StandardColumn = 3,
}

/// Controls which indexes to collect.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CollectIndexes {
    /// Excludes the specified index during collection.
    ExcludeSelf,
    /// Includes the specified index during collection.
    IncludeSelf,
}

#[cfg(feature = "smallvec")]
type GroupVec = smallvec::SmallVec<[CellGroup; 9]>;

#[cfg(not(feature = "smallvec"))]
type GroupVec = Vec<CellGroup>;

/// The set of all cell groups relevant to a game.
#[derive(Default, Debug, Clone)]
pub struct CellGroups {
    groups: GroupVec,
    groups_tagged: [GroupVec; 4],
}

impl AsRef<CellGroups> for &CellGroups {
    fn as_ref(&self) -> &CellGroups {
        self
    }
}

impl CellGroups {
    pub fn with_group(mut self, group: CellGroup) -> Self {
        self.add_group(group);
        self
    }

    pub fn add_group(&mut self, mut group: CellGroup) -> &mut Self {
        if group.id.is_none() {
            let ids = self.get_highest_id();
            group.id = Some(ids + 1);
        }

        self.groups.push(group.clone());
        self.groups_tagged[group.group_type as usize].push(group);

        self
    }

    pub fn with_default_rows_and_columns(self) -> Self {
        self.with_default_rows().with_default_columns()
    }

    //noinspection DuplicatedCode
    fn with_default_rows(mut self) -> Self {
        let mut check = IndexBitSet::ALL;
        let mut ids = self.get_highest_id();

        for y in 0..9 {
            let mut group = CellGroup::new(ids, CellGroupType::StandardRow);
            ids += 1;
            for x in 0..9 {
                let coord = Coordinate::new(x, y).into_index();
                group.add_index(coord);
                check.remove(coord);
            }

            self.add_group(group);
        }
        self
    }

    fn get_highest_id(&self) -> usize {
        self.groups
            .iter()
            .flat_map(|x| x.id)
            .max()
            .unwrap_or_default()
    }

    //noinspection DuplicatedCode
    fn with_default_columns(mut self) -> Self {
        let mut check = IndexBitSet::ALL;
        let mut ids = self.get_highest_id();

        for x in 0..9 {
            ids += 1;
            let mut group = CellGroup::new(ids, CellGroupType::StandardColumn);
            for y in 0..9 {
                let coord = Coordinate::new(x, y).into_index();
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
                ids += 1;
                let mut group = CellGroup::new(ids, CellGroupType::StandardBlock);

                for row in 0..3 {
                    for col in 0..3 {
                        let coord = Coordinate::new(x + col, y + row).into_index();
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

    pub fn with_hypersudoku_windows(self) -> Self {
        self.with_group_from_iter([10, 11, 12, 19, 20, 21, 28, 29, 30])
            .with_group_from_iter([14, 15, 16, 23, 24, 25, 32, 33, 34])
            .with_group_from_iter([46, 47, 48, 55, 56, 57, 64, 65, 66])
            .with_group_from_iter([50, 51, 52, 59, 60, 61, 68, 69, 70])
    }

    #[inline]
    pub fn get_at_xy(
        &self,
        x: u8,
        y: u8,
        mode: CollectIndexes,
    ) -> Result<IndexBitSet, NoMatchingGroup> {
        debug_assert!(x <= 9 && y <= 9);
        self.get_at_coord(Coordinate::new(x, y), mode)
    }

    #[inline]
    pub fn get_at_coord(
        &self,
        coord: Coordinate,
        mode: CollectIndexes,
    ) -> Result<IndexBitSet, NoMatchingGroup> {
        self.get_peers_at_index(coord.into_index(), mode)
    }

    pub fn get_peers_at_index(
        &self,
        index: Index,
        mode: CollectIndexes,
    ) -> Result<IndexBitSet, NoMatchingGroup> {
        let mut set = IndexBitSet::empty();
        for group in self.groups.iter().filter(|&g| g.contains(index)) {
            set.union(&group.indexes);
        }

        match mode {
            CollectIndexes::IncludeSelf => { /* intentionally left empty */ }
            CollectIndexes::ExcludeSelf => {
                set.remove(index);
            }
        };

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
        self.get_groups_at_index(coord.into_index())
    }

    pub fn get_groups_at_index(&self, index: Index) -> Result<Vec<CellGroup>, NoMatchingGroup> {
        let set: Vec<_> = self
            .groups
            .iter()
            .filter(|&g| g.contains(index))
            .cloned()
            .collect();

        if set.is_empty() {
            Err(NoMatchingGroup {})
        } else {
            Ok(set)
        }
    }

    pub fn get_peer_indexes(
        &self,
        index: Index,
        group_type: CellGroupType,
    ) -> impl Iterator<Item = Index> + '_ {
        self.groups_tagged[group_type as usize]
            .iter()
            .filter(move |&g| g.group_type == group_type && g.contains(index))
            .flat_map(CellGroup::iter_indexes)
    }

    pub fn iter(&self) -> Iter<'_, CellGroup> {
        self.groups.iter()
    }
}

/// A convenience trait for registering a [`CellGroup`] constructed from an iterator.
pub trait WithGroupFromIterator<A>: Sized {
    fn with_group_from_iter<T: IntoIterator<Item = A>>(self, iter: T) -> Self;
}

impl WithGroupFromIterator<Index> for CellGroups {
    fn with_group_from_iter<T: IntoIterator<Item = Index>>(self, iter: T) -> Self {
        self.with_group(CellGroup::from_iter(iter))
    }
}

impl WithGroupFromIterator<u8> for CellGroups {
    fn with_group_from_iter<T: IntoIterator<Item = u8>>(self, iter: T) -> Self {
        self.with_group(CellGroup::from_iter(iter))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("No matching group was found")]
pub struct NoMatchingGroup {}

impl Debug for CellGroupType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom => write!(f, "custom group"),
            Self::StandardBlock => write!(f, "standard block"),
            Self::StandardColumn => write!(f, "standard column"),
            Self::StandardRow => write!(f, "standard row"),
        }
    }
}

impl Default for CellGroupType {
    fn default() -> Self {
        CellGroupType::Custom
    }
}

/// A group of related indexes, e.g. a row, a column, ...
#[derive(Default, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct CellGroup {
    /// The internal ID of this group.
    pub id: Option<usize>,
    /// The type of the group
    pub group_type: CellGroupType,
    /// The indexes for this group.
    indexes: IndexBitSet,
}

impl CellGroup {
    #[inline]
    pub const fn new(id: usize, group_type: CellGroupType) -> Self {
        Self {
            id: Some(id),
            group_type,
            indexes: IndexBitSet::empty(),
        }
    }

    #[inline]
    pub const fn with_index(mut self, index: Index) -> Self {
        self.indexes = self.indexes.with_index(index);
        self
    }

    pub fn with_indexes<T: IntoIterator<Item = Index>>(mut self, indexes: T) -> Self {
        for index in indexes.into_iter() {
            self.indexes = self.indexes.with_index(index);
        }
        self
    }

    #[inline]
    pub fn with_type(mut self, cell_type: CellGroupType) -> Self {
        self.group_type = cell_type;
        self
    }

    #[inline]
    pub fn add_index(&mut self, index: Index) -> &mut Self {
        self.indexes.insert(index);
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
    pub fn iter_indexes(&self) -> IntoIndexBitSetIter {
        self.indexes.iter()
    }

    /// Iterates all indexes for this cell group.
    #[inline]
    pub fn into_iter_indexes(&self) -> IntoIndexBitSetIter {
        self.indexes.iter()
    }
}

impl FromIterator<Index> for CellGroup {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Index>>(iter: T) -> Self {
        Self::default().with_indexes(iter)
    }
}

impl FromIterator<u8> for CellGroup {
    #[inline]
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        Self::from_iter(IndexBitSet::from_iter(iter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Index;

    #[test]
    fn from_index_iter() {
        let cg = CellGroup::from_iter([Index::new(0), Index::new(2), Index::new(3)]);
        assert!(cg.contains(Index::new(0)));
        assert!(cg.contains(Index::new(2)));
        assert!(cg.contains(Index::new(3)));
        assert!(!cg.contains(Index::new(1)));
    }

    #[test]
    fn from_u8_iter() {
        let cg = CellGroup::from_iter([0, 2, 3]);
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
