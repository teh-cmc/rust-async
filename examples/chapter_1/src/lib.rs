#![feature(unboxed_closures, fn_traits)]

// ANCHOR: ping_mars
pub struct PingMars;

impl Iterator for PingMars {
    type Item = &'static str;

    fn next(&mut self) -> Option<Self::Item> {
        fn ping_mars() -> &'static str {
            use std::{thread::sleep, time::Duration};
            sleep(Duration::from_secs(2)); // simulating network
            "Hello from Mars!"
        }

        ping_mars().into()
    }
}
// ANCHOR_END: ping_mars

// ANCHOR: fib
pub struct Fibonacci {
    cur: usize,
    until: usize,
}

impl Fibonacci {
    pub fn new(until: usize) -> Self {
        Self { cur: 0, until }
    }
}

impl Iterator for Fibonacci {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur > self.until {
            return None;
        }
        let n = self.cur;
        self.cur += 1;

        fn fib(n: usize) -> usize {
            match n {
                v if v == 0 => 0,
                v if v == 1 => 1,
                v => fib(v - 1) + fib(v - 2),
            }
        }
        (n, fib(n)).into()
    }
}
// ANCHOR_END: fib

// ANCHOR: range
pub struct Range<T> {
    cur: T,
    end: T,
    incr: T,
}

impl<T> Range<T> {
    pub fn new(start: T, end: T, incr: T) -> Self {
        Self {
            cur: start,
            end,
            incr,
        }
    }
}

impl<T> Iterator for Range<T>
where
    T: std::ops::AddAssign + PartialOrd + Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match &self.cur {
            v if *v < self.end => {
                let ret = self.cur.clone();
                self.cur += self.incr.clone();
                ret.into()
            }
            _ => None,
        }
    }
}
// ANCHOR_END: range

// ANCHOR: range_closure
pub mod range_fn {
    pub fn new<T>(mut start: T, end: T, incr: T) -> impl FnMut() -> Option<T>
    where
        T: std::ops::AddAssign + PartialOrd + Clone,
    {
        move || {
            if start < end {
                let ret = start.clone();
                start += incr.clone();
                return ret.into();
            }
            None
        }
    }
}
// ANCHOR_END: range_closure

// ANCHOR: bounds
pub struct Bounds<I, T> {
    inner: I,
    min: T,
    max: T,
}

impl<I, T> Bounds<I, T> {
    pub fn new(inner: I, min: T, max: T) -> Self {
        Self { inner, min, max }
    }
}

impl<I> Iterator for Bounds<I, I::Item>
where
    I: Iterator,
    I::Item: PartialOrd,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some(v) if v >= self.min && v < self.max => return v.into(),
                Some(_) => {}
                None => return None,
            }
        }
    }
}
// ANCHOR_END: bounds

// ANCHOR: bounds_closure
pub mod bounds_fn {
    pub fn new<T, F>(mut inner: F, min: T, max: T) -> impl FnMut() -> Option<T>
    where
        T: PartialOrd,
        F: FnMut() -> Option<T>,
    {
        move || loop {
            match inner() {
                Some(v) if v >= min && v < max => return v.into(),
                Some(_) => {}
                None => return None,
            }
        }
    }
}
// ANCHOR_END: bounds_closure

// ANCHOR: bounds_ext
pub trait BoundsExt: Iterator
where
    Self: Sized,
{
    fn bounds<T>(self, min: T, max: T) -> Bounds<Self, T> {
        Bounds::new(self, min, max)
    }
}

impl<I: Iterator> BoundsExt for I {}
// ANCHOR_END: bounds_ext

// ANCHOR: bounds_ext_closure
trait BoundsExtFn<'a, T>: FnMut() -> Option<T>
where
    Self: 'a + Sized,
    T: 'a + std::cmp::PartialOrd,
{
    fn bounds(self, min: T, max: T) -> Box<dyn FnMut() -> Option<T> + 'a> {
        Box::new(bounds_fn::new(self, min, max))
    }
}

impl<'a, F, T> BoundsExtFn<'a, T> for F
where
    F: 'a + FnMut() -> Option<T>,
    T: 'a + std::cmp::PartialOrd,
{
}
// ANCHOR_END: bounds_ext_closure

// ANCHOR: filter
pub struct Filter<I, P> {
    inner: I,
    predicate: P,
}

impl<I, P> Filter<I, P> {
    pub fn new(inner: I, predicate: P) -> Self {
        Self { inner, predicate }
    }
}

impl<I, P> Iterator for Filter<I, P>
where
    I: Iterator,
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some(v) if (self.predicate)(&v) => return v.into(),
                Some(_) => {}
                None => return None,
            }
        }
    }
}
// ANCHOR_END: filter

// ANCHOR: filter_ext
pub trait FilterExt: Iterator
where
    Self: Sized,
{
    fn filter_with<P>(self, predicate: P) -> Filter<Self, P>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        Filter::new(self, predicate)
    }
}

impl<I: Iterator> FilterExt for I {}
// ANCHOR_END: filter_ext

// ANCHOR: iter_to_closure
pub fn iter_to_closure<I: Iterator>(inner: I) -> impl FnMut() -> Option<I::Item> {
    // We cannot implement Fn* traits directly on `I: Iterator` because of
    // coherence.
    struct Iter<I>(I);

    impl<I> FnOnce<()> for Iter<I>
    where
        I: Iterator,
    {
        type Output = Option<I::Item>;

        extern "rust-call" fn call_once(mut self, _args: ()) -> Self::Output {
            Iterator::next(&mut self.0)
        }
    }
    impl<I> FnMut<()> for Iter<I>
    where
        I: Iterator,
    {
        extern "rust-call" fn call_mut(&mut self, _args: ()) -> Self::Output {
            Iterator::next(&mut self.0)
        }
    }

    Iter(inner)
}
// ANCHOR_END: iter_to_closure

// ANCHOR: closure_to_iter
pub fn closure_to_iter<T, F: FnMut() -> Option<T>>(inner: F) -> impl Iterator<Item = T> {
    struct Iter<F>(F);

    impl<F, T> Iterator for Iter<F>
    where
        F: FnMut() -> Option<T>,
    {
        type Item = T;

        fn next(&mut self) -> Option<Self::Item> {
            self.0()
        }
    }

    Iter(inner)
}
// ANCHOR_END: closure_to_iter

// ANCHOR: multiplexed_iter
pub enum Poll<T> {
    Ready(Option<T>),
    NotReady,
}

pub struct Notifier {/* ... */}

pub trait MultiplexedIterator {
    type Item;

    /// Advances the iterator and returns the next value as a `Poll::Ready(T)`
    /// if it's ready to be yielded, otherwise returns `Poll::NotReady`.
    /// The poller is responsible for polling again and again, until an actual
    /// value can be returned.
    ///
    /// Returns `Poll::Ready(None)` when iteration is finished.
    /// Individual iterator implementations must notify the poller when it can
    /// poll again after having returned `Poll::NotReady`.
    fn next(&mut self, n: Notifier) -> Poll<Self::Item>;
}
// ANCHOR_END: multiplexed_iter

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;

#[test]
fn range() {
// ANCHOR: test_range
let mut j = 1;
for i in Range::new(1usize, 4, 1) {
    assert_eq!(j, i);
    j += 1;
}
// ANCHOR_END: test_range
}

#[test]
fn range_closure() {
// ANCHOR: test_range_closure
let mut f = range_fn::new(1, 4, 1);
assert_eq!(Some(1), f());
assert_eq!(Some(2), f());
assert_eq!(Some(3), f());
assert_eq!(None, f());
// ANCHOR_END: test_range_closure
}

#[test]
fn bounds() {
// ANCHOR: test_bounds
let mut it = Bounds::new(Range::new(1usize, 20, 1), 5, 8);
assert_eq!(Some(5), it.next());
assert_eq!(Some(6), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(None, it.next());
// ANCHOR_END: test_bounds
}

#[test]
fn bounds_closure() {
// ANCHOR: test_bounds_closure
let mut f = bounds_fn::new(range_fn::new(1usize, 20, 1), 5, 8);
assert_eq!(Some(5), f());
assert_eq!(Some(6), f());
assert_eq!(Some(7), f());
assert_eq!(None, f());
// ANCHOR_END: test_bounds_closure
}

#[test]
fn size() {
// ANCHOR: test_size
use std::mem::size_of_val;

let it = Range::new(1usize, 20, 1).into_iter();
assert_eq!(24, size_of_val(&it));

let it = Bounds::new(Range::new(1usize, 20, 1), 5,8).into_iter();
assert_eq!(40, size_of_val(&it));
// ANCHOR_END: test_size
}

#[test]
fn bounds_ext() {
// ANCHOR: test_bounds_ext
let mut it = Range::new(1usize, 20, 1).bounds(1, 20).bounds(3, 13).bounds(5, 8);
assert_eq!(Some(5), it.next());
assert_eq!(Some(6), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(None, it.next());
// ANCHOR_END: test_bounds_ext
}

#[test]
fn bounds_ext_closure() {
// ANCHOR: test_bounds_ext_closure
let mut f = range_fn::new(1usize, 20, 1).bounds(1, 20).bounds(3, 13).bounds(5, 8);
assert_eq!(Some(5), f());
assert_eq!(Some(6), f());
assert_eq!(Some(7), f());
assert_eq!(None, f());
// ANCHOR_END: test_bounds_ext_closure
}

#[test]
fn filter_ext() {
// ANCHOR: test_filter_ext
let mut it = Range::new(1usize, 20, 1).filter_with(|&v| v >= 5 && v < 8);
assert_eq!(Some(5), it.next());
assert_eq!(Some(6), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(None, it.next());
// ANCHOR_END: test_filter_ext
}

#[test]
fn empty_closure() {
// ANCHOR: test_empty_closure
let _f: fn(usize) -> bool = |v: usize| v >= 5 && v < 8; // compiles!
// ANCHOR_END: test_empty_closure
}

#[test]
fn handmade() {
// ANCHOR: handmade_decl
struct MyClosure<'a> {
    a: &'a i32,
    b: &'a i32,
}
// ANCHOR_END: handmade_decl

// ANCHOR: handmade_impl
impl<'a> FnOnce<(i32,)> for MyClosure<'a> {
    type Output = i32;

    extern "rust-call" fn call_once(self, _args: (i32,)) -> Self::Output {
        unreachable!()
    }
}

impl<'a> FnMut<(i32,)> for MyClosure<'a> {
    extern "rust-call" fn call_mut(&mut self, _args: (i32,)) -> Self::Output {
        unreachable!()
    }
}

impl<'a> Fn<(i32,)> for MyClosure<'a> {
    extern "rust-call" fn call(&self, (v,): (i32,)) -> Self::Output {
        v + self.a + self.b
    }
}
// ANCHOR_END: handmade_impl

// ANCHOR: test_handmade_decl
let a = 42;
let b = 100;
// ANCHOR_END: test_handmade_decl

// ANCHOR: test_handmade_native
let f: &dyn Fn(i32) -> i32 = &|v: i32| v + a + b; // Compiles!
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
// ANCHOR_END: test_handmade_native

// ANCHOR: test_handmade
let f = MyClosure { a: &a, b: &b };
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
// ANCHOR_END: test_handmade

// ANCHOR: test_handmade_native_move
let f: &dyn Fn(i32) -> i32 = &move |v: i32| v + a + b; // Compiles still!
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
// ANCHOR_END: test_handmade_native_move

// ANCHOR: test_handmade_illegal
struct MyNonCopyType(i32);
let a = MyNonCopyType(42);
let b = MyNonCopyType(100);
let f = |v: i32| {
    let ret = v + a.0 + b.0;
    drop(a);
    ret
};
assert_eq!(150, f(8)); // really `FnOnce::call_once(f, (8,))`
// assert_eq!(150, f(8)); // Won't compile: value used after move
// ANCHOR_END: test_handmade_illegal
}

#[test]
fn fnmut_as_fnonce() {
// ANCHOR: fnmut_as_fnonce
fn run_once<F>(f: F) -> i32 // `f` isn't even marked as `mut`..
where
    F: FnOnce() -> i32,
{
    f() // ..but `self` is really `&mut self`, because tricks!
}

fn run_mut<F>(mut f: F) -> i32
where
    F: FnMut() -> i32,
{
    f()
}

let mut a = 10;
let mut fmut = || {
    a += 1;
    a
};

assert_eq!(11, run_once(&mut fmut));
assert_eq!(12, run_once(&mut fmut));
assert_eq!(13, run_mut(&mut fmut));
assert_eq!(14, run_mut(&mut fmut));
// ANCHOR_END: fnmut_as_fnonce
}

#[test]
fn fn_as_fnmut_as_fnonce() {
// ANCHOR: fn_as_fnmut_as_fnonce
fn run_once<F>(f: F, b: i32) -> i32
where
    F: FnOnce(i32) -> i32,
{
    f(b)
}

fn run_mut<F>(mut f: F, b: i32) -> i32
where
    F: FnMut(i32) -> i32,
{
    f(b)
}

fn run<F>(f: F, b: i32) -> i32
where
    F: Fn(i32) -> i32,
{
    f(b)
}

let a = 10;
let f = |b: i32| a + b;

assert_eq!(52, run_once(&f, 42));
assert_eq!(52, run_once(&f, 42));
assert_eq!(52, run_mut(&f, 42));
assert_eq!(52, run_mut(&f, 42));
assert_eq!(52, run(&f, 42));
assert_eq!(52, run(&f, 42));
// ANCHOR_END: fn_as_fnmut_as_fnonce
}

#[test]
fn iter_to_closure_to_iter() {
// ANCHOR: test_iter_to_closure_to_iter
let mut it = Range::new(1usize, 20, 1).into_iter().bounds(5, 14);
assert_eq!(Some(5), it.next());
assert_eq!(Some(6), it.next());
assert_eq!(Some(7), it.next());

let mut f = iter_to_closure(it);
assert_eq!(Some(8), f());
assert_eq!(Some(9), f());
assert_eq!(Some(10), f());

let mut it = closure_to_iter(f);
assert_eq!(Some(11), it.next());
assert_eq!(Some(12), it.next());
assert_eq!(Some(13), it.next());

assert_eq!(None, it.next());
// ANCHOR_END: test_iter_to_closure_to_iter
}
}
