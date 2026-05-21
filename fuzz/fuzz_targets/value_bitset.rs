#![no_main]

use libfuzzer_sys::fuzz_target;
use sudoku2::{Value, ValueBitSet};

fn to_value(byte: u8) -> Option<Value> {
    Value::try_from((byte % 9) + 1).ok()
}

fuzz_target!(|data: &[u8]| {
    let mut set = ValueBitSet::empty();
    let mut iter = data.iter();

    while let (Some(op), Some(arg)) = (iter.next(), iter.next()) {
        let value = match to_value(*arg) {
            Some(v) => v,
            None => continue,
        };

        match op % 6 {
            0 => {
                set.insert(value);
                assert!(set.contains(value));
            }
            1 => {
                set.remove(value);
                assert!(!set.contains(value));
            }
            2 => {
                set.set_to(value);
                assert!(set.contains(value));
                assert!(set.is_exactly(value));
                assert_eq!(set.len(), 1);
                assert_eq!(set.as_single_value(), Some(value));
            }
            3 => {
                let before = set;
                let after = before.with_value(value);
                assert!(after.contains(value));
                assert!(after.contains_all(ValueBitSet::empty().with_value(value)));
            }
            4 => {
                let before = set;
                let after = before.without_value(value);
                assert!(!after.contains(value));
            }
            5 => {
                let other = ValueBitSet::empty().with_value(value);
                let unioned = set.with_union(other);
                let intersected = set.with_intersection(other);
                assert!(unioned.contains_all(set));
                assert!(unioned.contains_all(other));
                assert!(set.contains_all(intersected));
                assert!(other.contains_all(intersected));
            }
            _ => unreachable!(),
        }

        let collected: Vec<Value> = set.iter().collect();
        assert_eq!(collected.len(), set.len());
        assert_eq!(set.is_empty(), set.len() == 0);

        for v in &collected {
            assert!(set.contains(*v));
        }

        match set.as_single_value() {
            Some(v) => {
                assert_eq!(set.len(), 1);
                assert!(set.is_exactly(v));
            }
            None => {
                assert!(set.len() != 1);
            }
        }
    }
});
