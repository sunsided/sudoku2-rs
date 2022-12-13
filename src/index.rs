use crate::coordinate::Coordinate;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;

/// A classical Sudoku index, ranging 0..=80 for 81 fields.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Index(u8);

impl Index {
    #[inline]
    pub const fn new(index: u8) -> Self {
        debug_assert!(index < 81);
        Self(index)
    }

    #[inline]
    pub fn range() -> impl Iterator<Item = Index> {
        (0..81).map(Index::new)
    }

    #[inline]
    pub const fn into_coordinate(self) -> Coordinate {
        let x = self.0 % Coordinate::GAME_WIDTH;
        let y = self.0 / Coordinate::GAME_WIDTH;
        Coordinate { x, y }
    }
}

impl Deref for Index {
    type Target = u8;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Into<u8> for Index {
    #[inline]
    fn into(self) -> u8 {
        self.0
    }
}

impl Into<u8> for &Index {
    #[inline]
    fn into(self) -> u8 {
        self.0
    }
}

impl Debug for Index {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let coordinate = self.into_coordinate();
        coordinate.fmt(f)
    }
}

/// A simple bitset for storing regular Sudoku-sized (i.e., up to 81) index values.
///
/// ## Technical Notes
/// Practically this implementation allows for storing up to 127 different indexes.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct IndexBitSet {
    /// We anticipate at most 81 fields on a standard Sudoku game.
    /// We use a 128-bit type here to directly encode the field values,
    /// even though this wastes 47 bits.
    state: u128,
}

impl IndexBitSet {
    /// The mask for storing the actual values.
    const MASK: u128 = 0b111111111_111111111_111111111_111111111_111111111_111111111_111111111_111111111_111111111u128;

    /// The set that contains all indexes.
    pub const ALL: IndexBitSet = IndexBitSet { state: Self::MASK };

    /// The set that contains no indexes.
    pub const NONE: IndexBitSet = IndexBitSet { state: 0 };

    #[inline]
    pub const fn empty() -> Self {
        Self { state: 0 }
    }

    #[inline]
    pub const fn with_index(mut self, index: Index) -> Self {
        debug_assert!(index.0 < 81);
        let value = index.0 as u128;
        self.state |= (1u128 << value) & Self::MASK;
        self
    }

    #[inline]
    pub fn insert(&mut self, index: Index) -> &mut Self {
        debug_assert!(index.0 < 81);
        let value = index.0 as u128;
        self.state |= (1u128 << value) & Self::MASK;
        self
    }

    #[inline]
    pub fn try_insert(&mut self, index: Index) -> bool {
        debug_assert!(index.0 < 81);
        let value = index.0 as u128;
        let bitmask = (1u128 << value) & Self::MASK;
        let contains = (self.state & bitmask) > 0;
        self.state |= bitmask;
        !contains
    }

    #[inline]
    pub const fn without_index(mut self, index: Index) -> Self {
        debug_assert!(index.0 < 81);
        let value = index.0 as u128;
        self.state &= (!(1u128 << value)) & Self::MASK;
        self
    }

    #[inline]
    pub fn remove(&mut self, index: Index) -> &mut Self {
        debug_assert!(index.0 < 81);
        let value = index.0 as u128;
        self.state &= (!(1u128 << value)) & Self::MASK;
        self
    }

    #[inline]
    pub const fn with_union(mut self, other: &IndexBitSet) -> Self {
        self.state |= other.state & Self::MASK;
        self
    }

    #[inline]
    pub fn union(&mut self, other: &IndexBitSet) -> &mut Self {
        self.state |= other.state & Self::MASK;
        self
    }

    #[inline]
    pub const fn overlaps_with(&self, other: &IndexBitSet) -> bool {
        let state = (self.state & other.state) & Self::MASK;
        state > 0
    }

    #[inline]
    pub const fn contains(&self, index: Index) -> bool {
        debug_assert!(index.0 < 81);
        let value = index.0 as u128;
        let flag = self.state & (1 << value);
        flag != 0
    }

    #[inline]
    pub const fn contains_coord(&self, coord: Coordinate) -> bool {
        self.contains(coord.into_index())
    }

    #[inline]
    pub const fn contains_xy(&self, x: u8, y: u8) -> bool {
        debug_assert!(x < 9 && y < 9);
        self.contains_coord(Coordinate::new(x, y))
    }

    #[inline]
    pub const fn len(&self) -> usize {
        (self.state & Self::MASK).count_ones() as _
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.state & Self::MASK == 0
    }

    #[inline]
    pub const fn iter(&self) -> IndexBitSetIter {
        IndexBitSetIter {
            value: *self,
            index: 0,
        }
    }
}

impl FromIterator<u8> for IndexBitSet {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        Self {
            state: iter.into_iter().fold(0u128, |state, value| {
                debug_assert!(value < 81, "Index is out of range");
                state | 1u128 << value
            }),
        }
    }
}

impl FromIterator<Index> for IndexBitSet {
    fn from_iter<T: IntoIterator<Item = Index>>(iter: T) -> Self {
        Self {
            state: iter
                .into_iter()
                .fold(0u128, |state, value| state | 1u128 << value.0),
        }
    }
}

impl IntoIterator for IndexBitSet {
    type Item = Index;
    type IntoIter = IndexBitSetIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct IndexBitSetIter {
    value: IndexBitSet,
    index: u8,
}

impl Iterator for IndexBitSetIter {
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        let state = self.value.state;
        let mut index = self.index;
        while index < 81 {
            let test = 1u128 << index;
            if state & test != 0 {
                self.index = index + 1;
                return Some(Index::new(index));
            }
            index += 1;
        }

        self.index = 81;
        None
    }
}

pub trait CollectIndexBitSet {
    fn collect_bitset(self) -> IndexBitSet;
}

impl<T> CollectIndexBitSet for T
where
    T: IntoIterator<Item = Index>,
{
    fn collect_bitset(self) -> IndexBitSet {
        IndexBitSet::from_iter(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::index::{Index, IndexBitSet};

    #[test]
    fn with_value() {
        let a = Index::new(80);
        let b = Index::new(17);
        let c = Index::new(2);

        let bitset = IndexBitSet::default().with_index(a).with_index(b);

        assert!(bitset.contains(a));
        assert!(bitset.contains(b));
        assert!(!bitset.contains(c));

        assert_eq!(bitset.len(), 2);
        assert!(!bitset.is_empty());
    }

    #[test]
    fn insert() {
        let a = Index::new(80);
        let b = Index::new(17);
        let c = Index::new(2);

        let mut bitset = IndexBitSet::empty();
        bitset.insert(a);
        bitset.insert(b);

        assert!(bitset.contains(a));
        assert!(bitset.contains(b));
        assert!(!bitset.contains(c));

        assert_eq!(bitset.len(), 2);
        assert!(!bitset.is_empty());
    }

    #[test]
    fn union() {
        let a = Index::new(80);
        let b = Index::new(17);
        let c = Index::new(2);

        let bitset_a = IndexBitSet::default().with_index(a);
        let bitset_b = IndexBitSet::default().with_index(b);
        let bitset = bitset_a.with_union(&bitset_b);

        assert!(bitset.contains(a));
        assert!(bitset.contains(b));
        assert!(!bitset.contains(c));
    }

    #[test]
    fn without_value() {
        let a = Index::new(80);
        let b = Index::new(17);
        let c = Index::new(2);

        let bitset = IndexBitSet::default()
            .with_index(a)
            .with_index(b)
            .with_index(c);
        let bitset = bitset.without_index(a).without_index(b);

        assert!(!bitset.contains(a));
        assert!(!bitset.contains(b));
        assert!(bitset.contains(c));
    }

    #[test]
    fn from_u8_slice() {
        let a = Index::new(80);
        let b = Index::new(17);
        let c = Index::new(2);

        let bitset = IndexBitSet::from_iter([a, b]);

        assert!(bitset.contains(a));
        assert!(bitset.contains(b));
        assert!(!bitset.contains(c));
    }

    #[test]
    fn iter() {
        let a = Index::new(80);
        let b = Index::new(17);

        let bitset = IndexBitSet::default().with_index(a).with_index(b);
        let mut iter = bitset.iter();

        assert_eq!(iter.next(), Some(b));
        assert_eq!(iter.next(), Some(a));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_empty() {
        let bitset = IndexBitSet::empty();
        let mut iter = bitset.iter();

        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }
}
