#![allow(unused_imports)]

use chapter_1::{FilterExt, Range};
use std::mem::size_of_val;

// ANCHOR: empty_closure
#[cfg(feature = "empty_closure")]
fn empty_closure() {
    let range = Range::new(10usize, 20, 1).into_iter();
    assert_eq!(24, size_of_val(&range));

    let mut filter = range.filter_with(|&v| v >= 7 && v < 15);
    assert_eq!(24, size_of_val(&filter)); // 24 bytes!

    let x = filter.next();
    println!("{:?}", x);
}
// ANCHOR_END: empty_closure

// ANCHOR: capturing_closure
#[cfg(feature = "capturing_closure")]
fn capturing_closure() {
    let range = Range::new(10usize, 20, 1).into_iter();
    assert_eq!(24, size_of_val(&range));

    let min = 7;
    let max = 15;
    let mut filter = range.filter_with(|&v| v >= min && v < max);
    assert_eq!(40, size_of_val(&filter)); // 40 bytes!

    let x = filter.next();
    println!("{:?}", x);
}
// ANCHOR_END: capturing_closure

fn main() {
    #[cfg(feature = "empty_closure")]
    empty_closure();

    #[cfg(feature = "capturing_closure")]
    capturing_closure();
}
