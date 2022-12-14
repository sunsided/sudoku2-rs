use crate::index::Index;
use crate::value::{Value, ValueBitSet, ValueBitSetIter};
use crate::Coordinate;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct GameCell {
    /// The possible values for this cell.
    possible_values: ValueBitSet,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct IndexedGameCell {
    /// The index of this cell.
    pub index: Index,
    /// The inner cell.
    inner: GameCell,
}

impl Default for GameCell {
    fn default() -> Self {
        Self {
            possible_values: ValueBitSet::all_values(),
        }
    }
}

impl Deref for IndexedGameCell {
    type Target = GameCell;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for IndexedGameCell {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Into<GameCell> for IndexedGameCell {
    fn into(self) -> GameCell {
        self.inner
    }
}

impl IndexedGameCell {
    /// Constructs a new cell with all possible value candidates.
    pub fn new(index: Index) -> Self {
        Self {
            index,
            inner: GameCell::default(),
        }
    }

    #[inline]
    pub fn as_coordinate(&self) -> Coordinate {
        self.index.into_coordinate()
    }

    #[inline]
    pub fn to_coordinate(self) -> Coordinate {
        self.index.into_coordinate()
    }
}

impl GameCell {
    #[inline]
    pub const fn from_value(value: Value) -> Self {
        Self {
            possible_values: ValueBitSet::empty().with_value(value),
        }
    }

    #[inline]
    pub const fn from_values(values: ValueBitSet) -> Self {
        Self {
            possible_values: values,
        }
    }

    /// Determines whether this cell contains a specific value (possibly as a candidate).
    #[inline]
    pub const fn contains(&self, value: Value) -> bool {
        self.possible_values.contains(value)
    }

    /// Determines whether this cell contains a specific value (possibly as a candidate).
    #[inline]
    pub const fn is_exactly(&self, value: Value) -> bool {
        self.possible_values.is_exactly(value)
    }

    /// Determines whether this cell contains at least one of the specified values.
    #[inline]
    pub const fn contains_some(&self, values: ValueBitSet) -> bool {
        self.possible_values.contains_some(values)
    }

    /// Determines whether this cell contains at least one of the specified values.
    #[inline]
    pub const fn contains_all(&self, values: ValueBitSet) -> bool {
        self.possible_values.contains_all(values)
    }

    /// Replaces this cell's value(s) with the specified one.
    #[inline]
    pub fn set_to(&mut self, value: Value) -> &mut Self {
        self.possible_values.set_to(value);
        self
    }

    /// Removes a candidate from this cell.
    #[inline]
    pub fn remove(&mut self, value: Value) -> &mut Self {
        self.possible_values.remove(value);
        self
    }

    /// Removes a candidate from this cell.
    #[inline]
    pub fn without_value(mut self, value: Value) -> Self {
        self.possible_values.remove(value);
        self
    }

    /// Removes a candidate from this cell.
    #[inline]
    pub fn without_values(mut self, values: ValueBitSet) -> Self {
        self.possible_values.remove_many(values);
        self
    }

    /// Converts this [`GameCell`] into an [`IndexedGameCell`].
    #[inline]
    pub fn into_indexed(self, index: Index) -> IndexedGameCell {
        IndexedGameCell { index, inner: self }
    }

    /// Determines whether this cell is solved.
    #[inline]
    pub const fn is_solved(&self) -> bool {
        self.len() == 1
    }

    /// Determines whether this cell is impossible to solve.
    #[inline]
    pub const fn is_impossible(&self) -> bool {
        self.empty()
    }

    /// Gets the number of possible values for this cell.
    ///
    /// ## Return values
    /// If the result is `0`, this cell is impossible to solve.
    /// If the result is `1`, this cell is solved.
    #[inline]
    pub const fn len(&self) -> usize {
        self.possible_values.len()
    }

    /// Determines whether this cell has any value candidates.
    #[inline]
    pub const fn empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets the possible values as a bitset.
    #[inline]
    pub fn as_bitset(&self) -> &ValueBitSet {
        &self.possible_values
    }

    /// Gets the possible values as a bitset.
    #[inline]
    pub fn to_bitset(&self) -> ValueBitSet {
        self.possible_values
    }

    /// Iterates all possible values for this cell.
    #[inline]
    pub fn iter_candidates(&self) -> ValueBitSetIter {
        self.possible_values.iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::game_cell::{GameCell, IndexedGameCell};
    use crate::index::Index;
    use crate::value::Value;

    #[test]
    pub fn indexed_cell_acts_like_cell() {
        let ic = IndexedGameCell::new(Index::default());

        assert_eq!(*ic.index, 0u8);
        assert!(!ic.is_solved());
        assert_eq!(ic.len(), 9);

        let values: Vec<_> = ic.iter_candidates().collect();
        assert!(values.contains(&Value::ONE));
        assert!(values.contains(&Value::TWO));
        assert!(values.contains(&Value::THREE));
        assert!(values.contains(&Value::FOUR));
        assert!(values.contains(&Value::FIVE));
        assert!(values.contains(&Value::SIX));
        assert!(values.contains(&Value::SEVEN));
        assert!(values.contains(&Value::EIGHT));
        assert!(values.contains(&Value::NINE));
    }

    #[test]
    pub fn into_indexed() {
        let c = GameCell::default();
        assert_eq!(c.len(), 9);

        let values: Vec<_> = c.iter_candidates().collect();
        assert!(values.contains(&Value::ONE));
        assert!(values.contains(&Value::TWO));
        assert!(values.contains(&Value::THREE));
        assert!(values.contains(&Value::FOUR));
        assert!(values.contains(&Value::FIVE));
        assert!(values.contains(&Value::SIX));
        assert!(values.contains(&Value::SEVEN));
        assert!(values.contains(&Value::EIGHT));
        assert!(values.contains(&Value::NINE));

        let ic = c.into_indexed(Index::default());
        assert_eq!(*ic.index, 0u8);
        assert!(!ic.is_solved());
    }
}
