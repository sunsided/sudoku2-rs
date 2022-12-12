use std::fmt::{Debug, Formatter};
use std::num::{NonZeroU8, TryFromIntError};
use std::ops::Deref;

/// A classical Sudoku index, ranging 1..=9 for 9 fields.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Value(NonZeroU8);

/// Simplified type name for an optional value.
pub type ValueOption = Option<Value>;

impl Value {
    pub const ONE: Value = unsafe { Value::new_unchecked(1) };
    pub const TWO: Value = unsafe { Value::new_unchecked(2) };
    pub const THREE: Value = unsafe { Value::new_unchecked(3) };
    pub const FOUR: Value = unsafe { Value::new_unchecked(4) };
    pub const FIVE: Value = unsafe { Value::new_unchecked(5) };
    pub const SIX: Value = unsafe { Value::new_unchecked(6) };
    pub const SEVEN: Value = unsafe { Value::new_unchecked(7) };
    pub const EIGHT: Value = unsafe { Value::new_unchecked(8) };
    pub const NINE: Value = unsafe { Value::new_unchecked(9) };

    pub const fn new(value: NonZeroU8) -> Self {
        assert!(value.get() <= 9);
        Self(value)
    }

    pub fn try_from(value: u8) -> Result<Value, TryFromIntError> {
        assert!(value <= 9);
        let value = NonZeroU8::try_from(value)?;
        Ok(Self(value))
    }

    /// Uses [`NonZeroU8::new_unchecked`] to construct the value.
    #[inline]
    const unsafe fn new_unchecked(value: u8) -> Self {
        debug_assert!(value > 0 && value <= 9);
        Self(NonZeroU8::new_unchecked(value))
    }

    /// Gets the underlying [`u8`] value.
    const fn get(&self) -> u8 {
        self.0.get()
    }
}

impl Deref for Value {
    type Target = NonZeroU8;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.get())
    }
}

impl TryFrom<u8> for Value {
    type Error = ValueOutOfRangeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 0 || value > 9 {
            Err(ValueOutOfRangeError(value))
        } else {
            Ok(Self(unsafe { NonZeroU8::new_unchecked(value) }))
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("The specified value `{0}` is out of range")]
pub struct ValueOutOfRangeError(u8);

/// A simple bitset for storing regular Sudoku-sized (i.e., up to 9) cell values.
///
/// ## Technical Notes
/// Practically this implementation allows for storing up to 65535 different indexes.
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ValueBitSet {
    /// We anticipate at most 9 distinct values on a standard Sudoku game.
    /// We use a 16-bit type here to directly encode the field values,
    /// even though this wastes 7 bits.
    state: u16,
}

impl ValueBitSet {
    /// The mask for storing the actual values.
    const MASK: u16 = 0b111111111u16;

    pub const fn empty() -> Self {
        Self { state: 0 }
    }

    pub const fn all_values() -> Self {
        Self::empty()
            .with_value(Value::ONE)
            .with_value(Value::TWO)
            .with_value(Value::THREE)
            .with_value(Value::FOUR)
            .with_value(Value::FIVE)
            .with_value(Value::SIX)
            .with_value(Value::SEVEN)
            .with_value(Value::EIGHT)
            .with_value(Value::NINE)
    }

    #[inline]
    pub const fn with_value(mut self, value: Value) -> Self {
        debug_assert!(value.get() <= 9);
        let value = value.get() as u16;
        // Since the value is a non-zero u8 we subtract one for the first bit.
        self.state |= (1u16 << (value - 1)) & Self::MASK;
        self
    }

    #[inline]
    pub fn insert(&mut self, value: Value) -> &mut Self {
        debug_assert!(value.get() <= 9);
        let value = value.get() as u16;
        // Since the value is a non-zero u8 we subtract one for the first bit.
        self.state |= (1u16 << (value - 1)) & Self::MASK;
        self
    }

    #[inline]
    pub const fn without_value(mut self, value: Value) -> Self {
        debug_assert!(value.get() <= 9);
        let value = value.get() as u128;
        // Since the value is a non-zero u8 we subtract one for the first bit.
        self.state &= (!(1u16 << (value - 1))) & Self::MASK;
        self
    }

    #[inline]
    pub fn remove(&mut self, value: Value) -> &mut Self {
        debug_assert!(value.get() <= 9);
        let value = value.get() as u16;
        // Since the value is a non-zero u8 we subtract one for the first bit.
        self.state &= (!(1u16 << (value - 1))) & Self::MASK;
        self
    }

    #[inline]
    pub fn remove_many(&mut self, values: &ValueBitSet) -> &mut Self {
        self.state &= (!values.state) & Self::MASK;
        self
    }

    /// Sets the possible values to only the specified value.
    #[inline]
    pub fn set_to(&mut self, value: Value) -> &mut Self {
        debug_assert!(value.get() <= 9);
        let value = value.get() as u16;
        // Since the value is a non-zero u8 we subtract one for the first bit.
        self.state = (1u16 << (value - 1)) & Self::MASK;
        self
    }

    #[inline]
    pub const fn with_union(mut self, other: &ValueBitSet) -> Self {
        self.state |= other.state & Self::MASK;
        self
    }

    #[inline]
    pub fn union(&mut self, other: &ValueBitSet) -> &mut Self {
        self.state |= other.state & Self::MASK;
        self
    }

    #[inline]
    pub const fn contains(&self, value: Value) -> bool {
        debug_assert!(value.get() <= 9);
        let value = value.get() as u16;
        // Since the value is a non-zero u8 we subtract one for the first bit.
        let flag = self.state & (1u16 << (value - 1));
        flag != 0
    }

    #[inline]
    pub const fn is_exactly(&self, value: Value) -> bool {
        debug_assert!(value.get() <= 9);
        let value = value.get() as u16;
        // Since the value is a non-zero u8 we subtract one for the first bit.
        let flag = self.state & (1u16 << (value - 1));
        flag == self.state
    }

    #[inline]
    pub const fn contains_all(&self, values: &ValueBitSet) -> bool {
        (self.state & values.state) == values.state
    }

    #[inline]
    pub const fn contains_some(&self, values: &ValueBitSet) -> bool {
        (self.state & values.state) != 0
    }

    #[inline]
    pub const fn len(&self) -> usize {
        let masked = self.state & Self::MASK;
        masked.count_ones() as _
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.state & Self::MASK == 0
    }

    #[inline]
    pub const fn iter(&self) -> ValueBitSetIter {
        ValueBitSetIter {
            value: *self,
            index: 0,
        }
    }

    /// Reduces this set to a single value.
    ///
    /// ## Returns
    /// Returns [`Some`] value or [`None`] if this set encodes zero or more than one value.
    pub const fn as_single_value(&self) -> Option<Value> {
        // Need to test here, will produce attempt to left shift with overflow otherwise.
        if self.state == 0 {
            return None;
        }

        let pow2 = self.state.trailing_zeros() as u16;

        // Ensure that exactly one bit is set.
        let test = (1u16 << pow2) & Self::MASK;
        if self.state != test {
            return None;
        }

        // Zero is disallowed, so we add one.
        Some(unsafe { Value::new_unchecked(pow2 as u8 + 1) })
    }
}

impl From<&[u8]> for ValueBitSet {
    #[inline]
    fn from(values: &[u8]) -> Self {
        let mut state = 0u16;
        for value in values {
            debug_assert_ne!(*value, 0);
            state |= 1 << (value - 1);
        }
        Self { state }
    }
}

pub struct ValueBitSetIter {
    value: ValueBitSet,
    index: u8,
}

impl IntoIterator for ValueBitSet {
    type Item = Value;
    type IntoIter = ValueBitSetIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Iterator for ValueBitSetIter {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let state = self.value.state;
        let mut index = self.index;
        while index < 9 {
            let test = (state >> index) & 0b1;
            index += 1;
            if test != 0 {
                self.index = index;
                return Some(unsafe { Value::new_unchecked(index) });
            }
        }

        self.index = 10;
        None
    }
}

impl From<&[Value]> for ValueBitSet {
    #[inline]
    fn from(values: &[Value]) -> Self {
        let mut state = 0u16;
        for value in values {
            // Since the value is a non-zero u8 we subtract one for the first bit.
            state |= 1u16 << (value.get() - 1);
        }
        Self { state }
    }
}

impl From<&[ValueOption]> for ValueBitSet {
    #[inline]
    fn from(values: &[ValueOption]) -> Self {
        let mut state = 0u16;
        for value in values {
            if let Some(value) = value {
                // Since the value is a non-zero u8 we subtract one for the first bit.
                state |= 1u16 << (value.get() - 1);
            }
        }
        Self { state }
    }
}

impl Debug for ValueBitSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, value) in self.iter().enumerate() {
            if i > 0 {
                write!(f, " ").ok();
            }
            write!(f, "{}", value.0.get()).ok();
        }
        write!(f, "")
    }
}

pub trait IntoValueOptions {
    fn into(self) -> [ValueOption; 81];
}

impl IntoValueOptions for [u8; 81] {
    fn into(self) -> [ValueOption; 81] {
        let mut values = [None; 81];
        for (i, v) in self.into_iter().enumerate() {
            match v {
                10.. => panic!("An invalid value was specified"),
                0 => values[i] = None,
                x => values[i] = Some(unsafe { Value::new_unchecked(x) }),
            }
        }
        values
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn with_value() {
        let a = Value::try_from(9).unwrap();
        let b = Value::try_from(5).unwrap();
        let c = Value::try_from(2).unwrap();

        let bitset = ValueBitSet::default().with_value(a).with_value(b);

        assert!(bitset.contains(a));
        assert!(bitset.contains(b));
        assert!(!bitset.contains(c));

        assert_eq!(bitset.len(), 2);
        assert!(!bitset.is_empty());
    }

    #[test]
    fn set() {
        let a = Value::try_from(9).unwrap();
        let b = Value::try_from(5).unwrap();
        let c = Value::try_from(2).unwrap();

        let mut bitset = ValueBitSet::default().with_value(a).with_value(b);
        bitset.set_to(c);

        assert!(!bitset.contains(a));
        assert!(!bitset.contains(b));
        assert!(bitset.contains(c));

        assert_eq!(bitset.len(), 1);
        assert!(!bitset.is_empty());
    }

    #[test]
    fn union() {
        let a = Value::try_from(9).unwrap();
        let b = Value::try_from(5).unwrap();
        let c = Value::try_from(2).unwrap();

        let bitset_a = ValueBitSet::default().with_value(a);
        let bitset_b = ValueBitSet::default().with_value(b);
        let bitset = bitset_a.with_union(&bitset_b);

        assert!(bitset.contains(a));
        assert!(bitset.contains(b));
        assert!(!bitset.contains(c));
    }

    #[test]
    fn without_value() {
        let a = Value::try_from(9).unwrap();
        let b = Value::try_from(5).unwrap();
        let c = Value::try_from(2).unwrap();

        let bitset = ValueBitSet::default()
            .with_value(a)
            .with_value(b)
            .with_value(c);
        let bitset = bitset.without_value(a).without_value(b);

        assert!(!bitset.contains(a));
        assert!(!bitset.contains(b));
        assert!(bitset.contains(c));
    }

    #[test]
    fn from_u8_slice() {
        let a = Value::try_from(9).unwrap();
        let b = Value::try_from(5).unwrap();
        let c = Value::try_from(2).unwrap();

        let bitset = ValueBitSet::from([a, b].as_slice());

        assert!(bitset.contains(a));
        assert!(bitset.contains(b));
        assert!(!bitset.contains(c));
    }

    #[test]
    fn iter() {
        let a = Value::try_from(9).unwrap();
        let b = Value::try_from(5).unwrap();

        let bitset = ValueBitSet::default().with_value(a).with_value(b);
        let mut iter = bitset.into_iter();

        assert_eq!(iter.next(), Some(b));
        assert_eq!(iter.next(), Some(a));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_test() {
        let bitset = ValueBitSet { state: 16 };
        let mut iter = bitset.into_iter();

        assert_eq!(iter.next(), Some(Value::FIVE));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    pub fn all_values() {
        let set = ValueBitSet::all_values();
        assert_eq!(set.len(), 9);

        assert!(set.contains(Value::ONE));
        assert!(set.contains(Value::TWO));
        assert!(set.contains(Value::THREE));
        assert!(set.contains(Value::FOUR));
        assert!(set.contains(Value::FIVE));
        assert!(set.contains(Value::SIX));
        assert!(set.contains(Value::SEVEN));
        assert!(set.contains(Value::EIGHT));
        assert!(set.contains(Value::NINE));

        let values: Vec<_> = set.into_iter().collect();
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
    pub fn len_works() {
        let set = ValueBitSet::empty()
            .with_value(Value::FIVE)
            .with_value(Value::NINE);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn remove_many() {
        let a = Value::try_from(9).unwrap();
        let b = Value::try_from(5).unwrap();
        let c = Value::try_from(2).unwrap();

        let mut bitset = ValueBitSet::default()
            .with_value(a)
            .with_value(b)
            .with_value(c);
        let remove = ValueBitSet::default().with_value(a).with_value(b);
        bitset.remove_many(&remove);

        assert!(!bitset.contains(a));
        assert!(!bitset.contains(b));
        assert!(bitset.contains(c));
    }

    #[test]
    fn is_exactly() {
        let a = Value::try_from(9).unwrap();
        let b = Value::try_from(5).unwrap();
        let c = Value::try_from(2).unwrap();

        let mut bitset = ValueBitSet::default()
            .with_value(a)
            .with_value(b)
            .with_value(c);
        assert!(bitset.contains(c));
        assert!(!bitset.is_exactly(c));

        let remove = ValueBitSet::default().with_value(a).with_value(b);
        bitset.remove_many(&remove);

        assert!(bitset.contains(c));
        assert!(bitset.is_exactly(c));
    }

    #[test]
    fn as_single_value() {
        let a = Value::try_from(9).unwrap();
        let b = Value::try_from(5).unwrap();
        let c = Value::try_from(2).unwrap();

        let mut bitset = ValueBitSet::default()
            .with_value(a)
            .with_value(b)
            .with_value(c);
        assert!(bitset.as_single_value().is_none());

        let remove = ValueBitSet::default().with_value(a).with_value(b);
        bitset.remove_many(&remove);

        assert_eq!(bitset.as_single_value(), Some(c));
    }
}
