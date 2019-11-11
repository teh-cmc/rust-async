# Demystifying Asynchronous Rust

(You will find an mdBook version of this book [here](https://teh-cmc.github.io/rust-async/html/), if that's more your thing.)

## Who is this book for?

This book is targeted towards experienced programmers that already feel somewhat comfortable with vanilla Rust (you definitely do not need to be an "expert" though, I certainly am not) and would like to dip their toes into its async ecosystem.

As the title indicates, this is not so much a book about _how to use async Rust_ as much as it is about trying to build a solid understanding of how it all works under the hood. From there, efficient usage should come naturally.  
As such, we'll try to answer the usual fundamental questions that arise from any piece of sufficiently complex technology:
- How and why did we get to this?
- What are the layers that make up the stack?
- What are their respective roles?
- How and why do they work the way they do?
- How do they fit together?
- What are the upsides & drawbacks of this approach?
- What are the semantics of the overall execution model?
- How is everything represented in memory?
- Etc...

On our way to answering all of those, we will encounter lots and lots of abstractions that will look like complete magic at first.  
We won't hesitate to wander off the main road and take as long a detour as needed, until we've successfully suppressed every last bit of hidden magic.  
Digression will be the norm here, not the exception.

My hope is that, after reading this book, one would be able to A) dig into any arbitrarily complex async codebase and B) decipher any error message that the compiler might throw at them.

## Why this book when there already is <insert_name>?

[Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/01_getting_started/01_chapter.html) & [Async programming in Rust with async-std](https://book.async.rs/) are two examples of great upcoming resources to learn about Rust's asynchronous stack, and I'd highly suggest that you read through them in addition to this book.

That being said, there are several reasons that have pushed me to write this material on top of what already exists:
- First, I am learning myself, and writing these notes has helped me greatly in processing the ridiculous amount of information one has to digest to fully comprehend this stack. Might as well share them while I'm it.
- Second, I feel like there definitely is value to be had, as a community, in having the same subject covered from different angles. The approach taken by this book should hopefully be unique enough to warrant its own text.
- And last but not least, who doesn't want to be writing about Rust these days?

## Can I help?

Yep, you sure can.

I am not an async Rust expert by any means. I'm not even a Rust expert to begin with.  
There will be errors, misconceptions, inaccuracies and other awful, awful things in this book.  
Please report them via Github's issues.

***

1. [Towards less blocking pastures](#1-towards-less-blocking-pastures)  
    1.1. [Iterators](#11-iterators)  
    1.2. [Closures](#12-closures)  
    1.3. [Iterators are closures are iterators](#13-iterators-are-closures-are-iterators)  
    1.4. [The case for asynchronous Rust](#14-the-case-for-asynchronous-rust)  
2. [Chapter II](#2-chapter-ii)  

***

# 1. Towards less blocking pastures

```ignore
last-edited: 2019-11-11
rustc-version: rustc 1.40.0-nightly (b520af6fd 2019-11-03)
```

So what's the deal here? Why do we need asynchronous Rust in the first place?

In my experience, more often than not, the first answers to come up are along the lines of "to workaround the limitations of synchronous I/O".  
I'm not a big fan of this answer; while async I/O is undoubtedly the poster child of all async applications, I reckon it is still just that: _an_ application, out of many.

In particular, async I/O is a very tricky beast that brings with it A) its own set of very specific problems and B) a gazillon of moving pieces that have nothing to do with Rust's async primitives per-se.  
For these reasons, I'd like to instead try and approach the matter from a different, and perhaps more familiar angle: start from the beginning with iterators and closures, and see where that takes us.  
Of course, we'll cover async I/O when the time is right.

Now I'm sure that opening a book about asynchronous Rust with a whole chapter dedicated to iterators and closures might seem like a dubious choice but, as we'll see, iterators, closures, streams & futures are all very much alike in more than one ways.  
In fact, I simply cannot fathom how we'd go about demystifying asynchronous Rust without first lifting all of the magic behind iterators and closures. That might just be me, though.

Hence in this first chapter, we'll do just that, demystify iterators and closures:
- What they are and what they're not.
- Their memory representation and execution model.
- Their interesting similarities.
- And last but not least: why would we need and/or want something else to begin with?

## 1.1. Iterators

An iterator is a state-machine that you can move forward by calling its `.next` method.  
Doing so will simultaneously yield its current value as well as mutate it towards its next state (hence `next(&mut self)`).  
It's _that_ simple.

```rust
pub trait Iterator {
    type Item;

    /// Advances the iterator and returns the next value.
    ///
    /// Returns None when iteration is finished. Individual iterator
    /// implementations may choose to resume iteration, and so calling next()
    /// again may or may not eventually start returning Some(Item) again at some
    /// point.
    fn next(&mut self) -> Option<Self::Item>;
}
```

Here's an implementation of a simple `Range` iterator that yields all the values between two `T`s:
```rust
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
```
In action:
```rust
let mut j = 1;
for i in Range::new(1usize, 4, 1) {
    assert_eq!(j, i);
    j += 1;
}
```

Straighforward stuff. This does demonstrate a couple important characteristics of iterators, though.

**Laziness**

Iterators are lazy, they don't do anything unless _polled_.  
In this case the for loop is doing the polling, as it desugars to something along these lines:
```rust
let mut it = Range::new(10, 20, 1).into_iter();
while let Some(i) = it.next() {
    /* ... */
}
```

All the usual laziness-related goodies apply, e.g. in this specific case we never dynamically allocate anything: we can represent an arbitrarily large range of numbers without ever allocating a single byte of heap space.

Similarly, one can effectively _cancel_ the execution of an iterator at any time _between two yields_: you just stop polling it is all.

**Zero magic, zero allocs**

Our iterator is nothing more than a vanilla structure (its state) with a `.next` method defined on it.  
In this example, the iterator sits on `main`'s stack, and answers to the same rules as any other struct: ownership, lifetimes, marker traits, etc.

There really isn't any kind of magic going on here: no hidden allocations, no codegen, no nothing; it's as dull as it gets.

### 1.1.a. Combinators

Iterators can be defined in terms of other iterators, making it possible to _combine_ (hence "iterator combinators") them into arbitrarily complex state-machines by wrapping iterators into iterators into iterators.. and so on and so forth.

Here's a `Bounds` combinator that makes sure our `Range` never yields results that are out of bounds:
```rust,no_run
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
```

In action:
```rust
let mut it = Bounds::new(Range::new(1usize, 20, 1), 5, 8);
assert_eq!(Some(5), it.next());
assert_eq!(Some(6), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(None, it.next());
```

Once again, no magic here.  
`Bounds` simply takes ownership of a `Range`, and then the wrapper (`Bounds`) is free to delegate the polling to the wrappee (`Range`), injecting its own rules in the process (i.e. filtering values, in this case).

From the programmer's point-of-view, nothing changes: we get a _something_ that implements the `Iterator` trait and we can poll it as needed.  
Thanks to monomorphization, everything is still sitting on the stack here; we just have a bigger, self-contained struct is all:
```rust
use std::mem::size_of_val;

let it = Range::new(1usize, 20, 1).into_iter();
assert_eq!(24, size_of_val(&it));

let it = Bounds::new(Range::new(1usize, 20, 1), 5,8).into_iter();
assert_eq!(40, size_of_val(&it));
```

Random tip: `-Zprint-size-types` is a great tool to know what monomorphization has been up to:
```sh
$ cargo rustc --lib -- --test -Zprint-type-sizes
# [...]
`Range<usize>`: 24 bytes, alignment: 8 bytes
`Bounds<Range<usize>, usize>`: 40 bytes, alignment: 8 bytes
```

**What we have so far**

We can now build up complex state-machines by composing smaller, simpler parts.  
While this is great in an of itself, one will quickly disenchant when trying to build a sufficiently complex combinator:
```rust
let it = MyCombinator1::new(MyCombinator2::new(MyCombinator3::new(MyCombinator4::new(MyCombinator5::new(MyIterator::new())))));
```

Yikes. Enter extensions.

## 1.1.b. Extensions

Extension traits allow us to define some behavior and implement it for both local and external types (provided you respect trait coherence rules.. a topic for another day).  
They come in very handy in the case of iterator combinators, as they allow us to express our compound state-machines using something akin to the familiar builder pattern.

Here we define a `BoundsExt` trait and provide a default implementation for everything that is an `Iterator` (provided that their `Item`s are `PartialOrd`, of course!):
```rust
pub trait BoundsExt: Iterator
where
    Self: Sized,
{
    fn bounds<T>(self, min: T, max: T) -> Bounds<Self, T> {
        Bounds::new(self, min, max)
    }
}

impl<I: Iterator> BoundsExt for I {}
```
And just like that, we're able to express our intent in a much more natural fashion:
```rust
let mut it = Range::new(1usize, 20, 1).bounds(1, 20).bounds(3, 13).bounds(5, 8);
assert_eq!(Some(5), it.next());
assert_eq!(Some(6), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(None, it.next());
```

And, again, no magic in sight. This is effectively just syntactic sugar.  
In fact, it's all so _not_ magic that the compiler did not even realize that our combinator chain is absolute non-sense and a complete waste of resources:
```sh
$ cargo rustc --lib -- --test -Zprint-type-sizes
# [...]
`Bounds<Bounds<Bounds<Range<usize>, usize>, usize>, usize>`: 72 bytes, alignment: 8 bytes
```
How could it? It's just blindly monomorphizing structs inside of other structs, that's all there is to it!

**What we have so far**

We can now build up complex state-machines by composing them from smaller, simpler parts... and what's more, we can even do it in an expressive, readable and maintainable way.

Still, if we had to implement an iterator combinator from scratch everytime we wanted to achieve a slightly different behavior for our overall state-machine, things would get very tedious, very fast; which is why iterators are almost always used in tandem with their close friends, closures.

## 1.2. Closures

While iterators are pretty straightforward both from a usage and an implementation standpoint, closures are anything but.  
In fact, I'd argue they're one of the most complex pieces of "standard" synchronous Rust.  
Their very expressive nature, thanks to a lot of magical sugar exposed by the compiler, make them a prime tool to push the type system into very complex corners, whether voluntarily.. or not.

Closures also happen to be the cornerstone of any serious asynchronous codebase, where their incidental complexity tends to skyrocket as a multitude of issues specific to asynchronous & multi-threaded code join in on the party.

### 1.2.a. A better `Bounds`

We'll kick off this section by turning our `Bounds` filter into a filter of.. well, anything, really:
```rust
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
```
Might as well provide an extension for it too:
```rust
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
```
Lo and behold:
```rust
let mut it = Range::new(1usize, 20, 1).filter_with(|&v| v >= 5 && v < 8);
assert_eq!(Some(5), it.next());
assert_eq!(Some(6), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(None, it.next());
```
Yep, that does it.

So that's nice and all but.. how does our final state-machine ends up being implemented?
```sh
$ cargo rustc --lib -- --test -Zprint-type-sizes
# [...]
`Filter<Range<usize>, [closure]>`: 24 bytes, alignment: 8 bytes
`Range<usize>`: 24 bytes, alignment: 8 bytes
```
Wait, wat? How come a monomorphized `Filter<Range<usize>, <[closure]>>` is the same size as a `Range<usize>`?

The only way this is possible is if storing our closure costs a whopping 0 byte which.. doesn't seem plausible?  
Let's take a minute to try and understand what's going on here.

### 1.2.b. What's a closure, anyway?

A closure is nothing but a structure (its captured environment) that implements one (or more) trait from the `Fn*` family of traits (`FnOnce`, `FnMut` and `Fn`):
```rust
pub trait FnOnce<Args> {
    type Output;
    extern "rust-call" fn call_once(self, args: Args) -> Self::Output;
}

pub trait FnMut<Args>: FnOnce<Args> {
    extern "rust-call" fn call_mut(&mut self, args: Args) -> Self::Output;
}

pub trait Fn<Args>: FnMut<Args> {
    extern "rust-call" fn call(&self, args: Args) -> Self::Output;
}
```
What that structure looks like will vary depending on the environment that the closure captures.  
For that reason, every closure has a different type (!), and every closure requires a proper structure declaration in order to carry its state.  
Obviously, having to manually declare a proper definition for your closure's captured state every time would be way too cumbersome, to the point of rendering closures completely useless.

To cope with that, the compiler automatically generates an appropriate anonymous structure every time you create a closure.  
Consider e.g. the following code:
```rust
let a = 42;
let b = 100;
let f = |v: i32| v + a + b;
```
Behind the scenes, the compiler will generate something along these lines to store the state of the closure:
```rust
struct __anonymous_e3b0105<'a> {
    a: &'a i32,
    b: &'a i32,
}
```

Now, if we instead had specified that we wanted to _move_ (i.e. take ownership of) the captured variables into the closure's state rather than just keep references to them, i.e. this:
```rust
let a = 42;
let b = 100;
let f = move |v: i32| v + a + b;
```
would then turn into this:
```rust
struct __anonymous_e3b0105 {
    a: i32,
    b: i32,
}
```
Don't take my word for it, ask the compiler:
```sh
$ cargo rustc --lib -- --test -Zprint-type-sizes
# [...]
`[closure<a:&i32, b:&i32>]`: 16 bytes ## let f = |v: i32| v + a + b;
`[closure<a:i32, b:i32>]`: 8 bytes ## let f = move |v: i32| v + a + b;
```

And so that covers the issue of generating an appropriate structure to hold the state (or captured environment) of the closure.  
What about implementing `FnOnce`/`FnMut`/`Fn` on that generated structure, though?

Similarly, manually providing the right `Fn*` trait(s) implementations for each and every closure would be unmanageable, and so, once again, the compiler got our backs and does it for us.  
To see what these implementations might look like, we could either A) have a look at the generated IR and/or assembly, or better yet, B) handcraft our own closure out of thin air.

Let's go with option B.

### 1.2.c. Handcrafted closures

Remember we had this:
```rust
let a = 42;
let b = 100;
let f = |v: i32| v + a + b;
```
Now what we'd like to do is to implement `f` without any help from the compiler.

First, we need to store our state somewhere. In this case, the capture is made by reference and so our structure should reflect that:
```rust
struct MyClosure<'a> {
    a: &'a i32,
    b: &'a i32,
}
```
Then, we need to implement the right `Fn*` trait(s). This part is a bit trickier.

---

### Aside: The many, many faces of Closures

When you create a closure, the compiler will always try to implement the most versatile of all the `Fn*` traits that it can, i.e. the trait that will allow you to use the closure in as many situations as possible.  
Whether or not a `Fn*` trait can be implemented depends solely on how the closure interacts with its state.  

**`FnOnce`**

If the closure moves anything out of its state, then its state (i.e. `self`) will have to be _consumed_ to perform the call, in which case the only trait that it can implement is `FnOnce`:
```rust
fn call_once(self, args: Args) -> Self::Output // `self`
```

/!\\ A common misconception is that whether a closure is or isn't `FnOnce` has anything to do with the use of `move`. It does _**not**_.

This closure is `Fn`, as demonstrated by the multiple calls to it:
```rust
let a = 42;
let b = 100;
let f: &dyn Fn(i32) -> i32 = &|v: i32| v + a + b; // Compiles!
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
```
And so is this one:
```rust
let a = 42;
let b = 100;
let f: &dyn Fn(i32) -> i32 = &move |v: i32| v + a + b; // Compiles still!
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
```
It doesn't matter that the second closure moves `a` & `b` into its state (well it certainly matters to the enclosing scope, which can't refer to these variables anymore, but that's besides the point).

What matters is how the closure interacts with its state when it gets called.  
In the example above, that interaction is just a read through a reference, and so a shared reference to the state (i.e. `&self`) is enough to perform the call: the compiler makes sure that this closure is `Fn`.

Now if you were to do this on the other hand..:
```rust
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
```
Now that's a big no-no. `drop(a)` moves `a` out of the closure's state, and so the only way to perform the call is to consume its state (i.e. `self`). The compiler makes sure that this closure is `FnOnce`, and thus uncommenting the second call won't compile.  
Notice that we're even capturing `a` & `b` by reference in this case and it doesn't matter, because this has nothing to do with the use of `move`!

**`FnMut`**

If the closure needs to modify its state during execution, but doesn't need to move anything out of it, then it's gonna need a mutable reference to `self` to perform the call; i.e. it implements `FnMut`:
```rust
fn call_mut(&mut self, args: Args) -> Self::Output // `&mut self`
```
Of course, if our `FnMut` closure can be called N times, then it would certainly make sense that we should be able to call it only once. Indeed, `FnMut` is a supertrait of `FnOnce` (hence `FnMut<Args>: FnOnce<Args>`).  
This is easier to visualize with an example:
```rust
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
```
And the reason why this works is because of this little jewel in libcore:
```rust
#[stable(feature = "rust1", since = "1.0.0")]
impl<A, F: ?Sized> FnOnce<A> for &mut F
where
    F: FnMut<A>,
{
    type Output = F::Output;
    extern "rust-call" fn call_once(self, args: A) -> F::Output {
        (*self).call_mut(args)
    }
}
```

**`Fn`**

Finally, if the closure just reads from its environment without ever modifying it, all it's gonna need to perform a call is a shared refence to `self`: it implements `Fn`.
```rust
fn call(&self, args: Args) -> Self::Output // `&self`
```
Once again, no reason why a `Fn` closure couldn't behave as a `FnMut`; if a closure can be executed N times while modifying its state, it certainly can be executed N times without modifying it (hence `Fn<Args>: FnMut<Args>`).  
And, as we know, if a closure is `FnMut`, then it is `FnOnce` too:
```rust
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
```
Once again, we can thank libcore for this:
```rust
#[stable(feature = "rust1", since = "1.0.0")]
impl<A, F: ?Sized> FnOnce<A> for &F
where
    F: Fn<A>,
{
    type Output = F::Output;

    extern "rust-call" fn call_once(self, args: A) -> F::Output {
        (*self).call(args)
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<A, F: ?Sized> FnMut<A> for &F
where
    F: Fn<A>,
{
    extern "rust-call" fn call_mut(&mut self, args: A) -> F::Output {
        (**self).call(args)
    }
}
```

And that concludes our aside regarding the `Fn*` family of traits.

---

Back to our original business.  
We were wondering how to implement the right `Fn*` traits for our closure's state:
```rust
struct MyClosure<'a> {
    a: &'a i32,
    b: &'a i32,
}
```
Our closure only references its environment: it never modifies it nor does it ever move it somewhere else, therefore the most versatile implementation that we can provide is `Fn`, which should allow it to be run pretty much anywhere.  
As we've seen, `Fn` is a supertrait of `FnMut` is a supertrait of `FnOnce`, and so we need to implement the entire family tree in this case:
```rust
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
```
Lo and behold, we've got ourselves a closure:
```rust
let a = 42;
let b = 100;
let f = MyClosure { a: &a, b: &b };
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
assert_eq!(150, f(8)); // really `Fn::call(&f, (8,))`
```

So that's great and all, but it still doesn't explain why this:
```rust
let mut it = Range::new(1usize, 20, 1).filter_with(|&v| v >= 5 && v < 8);
assert_eq!(Some(5), it.next());
assert_eq!(Some(6), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(None, it.next());
```
yields this:
```
`Filter<Range<usize>, [closure]>`: 24 bytes, alignment: 8 bytes
`Range<usize>`: 24 bytes, alignment: 8 bytes
```
I.e. how a `Range<usize>` happens to be the same size as a `Filter<Range<usize>`.
The first thing to take note of is that this closure never captures anything, and so it'd make sense that its state is 0-byte sized:
```sh
$ cargo rustc --lib -- --test -Zprint-type-sizes
# [...]
`[closure]`: 0 bytes, alignment: 1 bytes
```
In fact, the compiler won't even bother generating an anonymous structure for it, and so our closure lives entirely in the code section of the executable: it has no associated data.  
Effectively, it is just a plain function pointer:
```rust
let _f: fn(usize) -> bool = |v: usize| v >= 5 && v < 8; // compiles!
```

That explains why our closure is 0 byte, but it certainly doesn't explain why a `Filter<Range<usize>, [closure]>` is the same size as a `Range<usize>`. Even if the closure itself is 0 byte, `Filter` still has has to hold a function pointer to the code portion of the closure, which is 8 bytes on a 64bit platform such as mine.  
What are we missing?

Consider the following code where we instantiate a `Filter` using an empty closure (i.e. an anonymous function):
```rust
#[cfg(feature = "empty_closure")]
fn empty_closure() {
    let range = Range::new(10usize, 20, 1).into_iter();
    assert_eq!(24, size_of_val(&range));

    let mut filter = range.filter_with(|&v| v >= 7 && v < 15);
    assert_eq!(24, size_of_val(&filter)); // 24 bytes!

    let x = filter.next();
    println!("{:?}", x);
}
```
To understand what's actually going on here, we need to have a direct look at the assembly generated for our `Filter`'s `.next` method:
```sh
$ cargo asm --features empty_closure \
            --asm-style att \
            --build-type debug \
            '<chapter_1::Filter<I,P> as core::iter::traits::iterator::Iterator>::next'
```
We'll specifically focus on the indirect call to the predicate (i.e. `(self.predicate)(&v)`):
```assembly
;; (self.predicate)(&v)
leaq    64(%rsp), %rax
movq    %rax, 80(%rsp)
movq    32(%rsp), %rdi
movq    80(%rsp), %rax
movq    %rax, 88(%rsp)
movq    88(%rsp), %rsi
callq   closure_filters::empty_closure::{{closure}}
movb    %al, 31(%rsp)
```
Don't worry too much about all these `mov` instructions for now, the only relevant piece of information is in fact written in plain english: `callq closure_filters::empty_closure::{{closure}}`.  
The compiler has completely optimized out the indirect call through `self.predicate`: the address of the closure is hardcoded right there into the `.next` method!  
We have monomorphization to thank for that, it generated a `.next` function specialized for `I = Range<usize>` and `P = [closure]`, where `[closure]` denotes the unique, anonymous type of our closure (remember, _each and every_ closure gets its own anonymous type).  
Since `self.predicate` is a `P`, and the compiler knows that `P` is nothing but a function pointer (i.e. `P: FnMut`), it therefore knows that it can eliminate the runtime dispatch in favor of what we're seeing here.

What if our closure _did_ capture some state, then?
```rust
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
```
We can see here that we capture two references to `usize`, i.e. 16 bytes:
```sh
$ cargo rustc --bin closure_filters --features capturing_closure -- -Zprint-type-sizes
# [...]
`[closure<min:&usize, max:&usize>]`: 16 bytes, alignment: 8 bytes
```
And so our `Filter<Range<usize>, [closure<&usize,&usize>]` should be
```sh
sizeof(Range<usize>) + ## 24
sizeof([closure<&usize,&usize>]) + ## 16
sizeof(&dyn FnMut(&I::Item) -> bool) ## 8
```
i.e. 50 bytes.  
But of course, the same optimization applies:
```sh
$ cargo rustc --bin closure_filters --features capturing_closure -- -Zprint-type-sizes
# [...]
`Filter<Range<usize>, [closure<min:&usize, max:&usize>]>`: 40 bytes, alignment: 8 bytes
```
Once again monomorphization has eliminated the extra indirection:
```sh
$ cargo asm --features capturing_closure \
            --asm-style att \
            --build-type debug \
            '<chapter_1::Filter<I,P> as core::iter::traits::iterator::Iterator>::next'
```
```assembly
;; (self.predicate)(&v)
leaq    64(%rsp), %rax
movq    %rax, 80(%rsp)
movq    32(%rsp), %rax
addq    $24, %rax       ;; 24 bytes offset from the start of `Filter<I, P>` is `self.predicate`,
                        ;; i.e. the captured state, aka `self`.
movq    80(%rsp), %rcx
movq    %rcx, 88(%rsp)
movq    88(%rsp), %rsi
movq    %rax, %rdi
callq   closure_filters::capturing_closure::{{closure}}
movb    %al, 31(%rsp)
```
The attentive reader shall notice the two extra instructions this time: the compiler is properly setting up the stack so that our closure can access its state (which is made to point to `self.predicate`, using a 24 bytes offset).

### 1.2.c. Usual complications

When working in single-threaded environments, closures are usually a breathe to work with. The compiler gets to do its magic and you rarely seem to get into trouble, if at all.  
Once we get into async code, though, some concepts that are usually mostly invisible will start becoming very apparent as Rust compile-time safeties start kicking in.

**Higher Ranked Trait Bounds**

The first complication that I want to mention has nothing to do with neither multi-threading nor asynchronous code, but you're bound to face it at one point or another if you start digging into any closure-heavy codebase (which is true of any async codebase, so..), so I'd rather mention it in passing.

TL;DR, you _will_ encounter this syntax at one point or another:
```rust
// Notice the `for<'a>` in that trait bound.
fn my_func<F: for<'a> Fn(&'a str) -> bool>(f: F) -> bool { /* ... */ }
```
which is meant to denote the higher-kindness of a lifetime trait bound, meaning that `&str` cannot outlive `'a`, where `'a` is _any_ lifetime, i.e. it is left unconstrained.

While I would love to talk about Generic Associated Types, Higher Ranked Types/Lifetimes and all that fun at some point, now is nor the time nor the place.  
For now, just keep in mind that this syntax exists, that you will most likely encouter it at some point, and that you'll find all the information you'll ever need in [the original RFC](https://rust-lang.github.io/rfcs/0387-higher-ranked-trait-bounds.html) as well as in [the corresponding nomicon entry](https://doc.rust-lang.org/nomicon/hrtb.html).

**Auto marker traits and inferred lifetimes**

Always keep in mind that closures are just structures, and thus the usual rules regarding compound types and auto & marker traits as well as lifetimes apply.  
I.e. the lifetime and intrinsic properties of a state-machine built up from the combination of iterators and closures will be a direct result of both its explicitly _and_ implicitly captured enviroments.

Consider our `Filter` combinator, for example:
```rust
let min = 5;
let max = 8;
let mut it = Range::new(1usize, 20, 1).filter_with(|&v| v >= min && v < max);
assert_eq!(Some(5), it.next());
assert_eq!(Some(6), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(None, it.next());
```

In this case, the resulting state-machine's (`it`) lifetime is bounded by the lifetimes of `min` & `max`.  
Similarly, whether `it` can or cannot be moved between threads (i.e. `Send`) depends on whether `min` & `max` can be sent between threads.

Obviously, in a state-machine as simple as this one, this won't ever cause you any trouble.  
In a massive asynchronous state-machine, built-up from many many parts (that may even cross module boundaries), and that will be arbitrarily moved back and forth between threads by some executor that you might or might not control, on the other hand... Let's just say that it can be easy to lose track of who requires what and for how long.  

But, hey, that's precisely why we're using Rust in the first place!  
Compiler errors for these hard cases have become insanely good too, if quite verbose.

## 1.3. Iterators are closures are iterators

_And now for the fun part._

Let's recap the first two sections of this chapter, in as many sentences:
- Iterators are state-machines that are implemented using a structure to keep their state and a `.next()` method that takes a mutable reference to said state (`&mut self`) in order to move the machine forward.
- Closures are state-machines that are implemented using a structure to hold their state (i.e. their captured environment) and a `call_once`/`call_mut`/`call` method that takes said state by move/mutable reference/shared reference (respectively) in order to drive the machine forward.

If you're thinking to yourself that these two sound similar, that's because they are.  
In this section we're going to have us a little fun by digressing into these similarities.

### 1.3.a. Iterators as closures

Consider our `Range` iterator from earlier:
```rust
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
```
that we used like this:
```rust
let mut j = 1;
for i in Range::new(1usize, 4, 1) {
    assert_eq!(j, i);
    j += 1;
}
```

Could we express `Range` in terms of a closure instead?  
Well of course we can, what's an iterator but a `FnMut() -> Option<T>` closure, really?
```rust
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
```
which we can basically use the same way as an iterator:
```rust
let mut f = range_fn::new(1, 4, 1);
assert_eq!(Some(1), f());
assert_eq!(Some(2), f());
assert_eq!(Some(3), f());
assert_eq!(None, f());
```
But what about combinators, you say?

### 1.3.b. Closure combinators

Remember our `Range` iterator could be combined with a `Bounds` iterator, allowing us to express something like the following:
```rust
let mut it = Bounds::new(Range::new(1usize, 20, 1), 5, 8);
assert_eq!(Some(5), it.next());
assert_eq!(Some(6), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(None, it.next());
```

We can apply the same pattern to closures: by moving ownership of the wrappee inside the state of the wrapper, we can delegate the state-machinery from the wrapper and into the wrappee.
```rust
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
```
Again, using our closure combinator is pretty similar to using our iterator combinator:
```rust
let mut f = bounds_fn::new(range_fn::new(1usize, 20, 1), 5, 8);
assert_eq!(Some(5), f());
assert_eq!(Some(6), f());
assert_eq!(Some(7), f());
assert_eq!(None, f());
```
And, as we'd expect based on everything we've learned so far, what gets generated here is pretty much the same thing, both from a memory representation and execution model standpoints, as what got generated for the equivalent iterator combinator.  
```sh
$ cargo rustc --lib -- --test -Zprint-type-sizes
# [...]
## Iterator combinator
`Bounds<Range<usize>, usize>`: 40 bytes, alignment: 8 bytes
## Closure combinator
`[closure<f:[closure<start:usize, end:usize, incr:usize>], min:usize, max:usize>]`: 40 bytes, alignment: 8 bytes
```
What about extensions, though? Those were _the_ true killer feature of iterator combinators!

### 1.3.c. Closure extensions

Remember how we were able to do this?
```rust
let mut it = Range::new(1usize, 20, 1).bounds(1, 20).bounds(3, 13).bounds(5, 8);
assert_eq!(Some(5), it.next());
assert_eq!(Some(6), it.next());
assert_eq!(Some(7), it.next());
assert_eq!(None, it.next());
```

Well closures are a trait too, aren't they? Surely we can extend it!
```rust
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
```
Ta-daaaa!
```rust
let mut f = range_fn::new(1usize, 20, 1).bounds(1, 20).bounds(3, 13).bounds(5, 8);
assert_eq!(Some(5), f());
assert_eq!(Some(6), f());
assert_eq!(Some(7), f());
assert_eq!(None, f());
```
Ok, that's not really "ta-da" worthy, actually. I lied.

While what we've created here does indeed provide similar functional behavior as our hand-crafted combinator defined above, it has a _completely different_ memory representation and execution model (not to mention that the code itself looks way more complex).  
And by different, I actually mean _worse in every way_.  
We've just brought heap allocations and pointer indirections upon ourselves. Oh noes.

```sh
$ cargo rustc --lib -- --test -Zprint-type-sizes
# [...]
`[closure<f:Box<dyn FnMut() -> Option<usize>>, min:usize, max:usize>]`: 32 bytes, alignment: 8 bytes
```

All of our issues stem from the use of a `Box` there (`Box<dyn FnMut() -> Option<T> + 'a>`), which begs the question: why did we reach for a `Box` in the first place?

The reason is actually a limitation in Rust's type system, namely the lack of Generic Associated Types, which prevents us from expressing a trait method that returns a `impl FnMut() -> Option<T>`, i.e. an unconstrained generic type (GATs are in fact a limited form of HKTs which, once again, are a topic for another day).

But wait a minute, why didn't we face this issue back when we implemented `BoundsExt` for iterators?
```rust
pub trait BoundsExt: Iterator
where
    Self: Sized,
{
    fn bounds<T>(self, min: T, max: T) -> Bounds<Self, T> {
        Bounds::new(self, min, max)
    }
}

impl<I: Iterator> BoundsExt for I {}
```
That right here is the magical part: `Bounds<Self, T>`.  
I.e. we never had the problem before because we were actually capable of referring to a `Bounds<Self, T>` by its name.

Unfortunately, one of the first thing we've learned about closures is that we cannot name them; they're all different, and they're all anonymous. Thus we _have_ to return a generic type here, and it certainly cannot be constrained by the caller, since they couldn't possibly name the type of a closure that doesn't even yet exist!

Therefore, what we're left to work with is an unconstrained generic type to return, and an unfortunate solution to the problem: boxing the return value into a trait object, which is exactly what we did there.  
Of course, in doing so we've also sacrificed any chance at monomorphization and all the good things that can come out of it: inlining, code elimination, recursive optimizations, etc...

### 1.3.d. Back and forth

We've shown that we can express iterators as closures and vice-versa, thus we should be able to freely turn one into the other and back again on the fly, shouldn't we?

Turning an iterator into a closure is just a matter of implementing `FnMut` for `I: Iterator`, where the implementation does nothing but delegate the call to `Iterator::next`:
```rust
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
```
Going the other way is even more straightforward:
```rust
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
```
And in fact, it seems so natural to want to express an iterator as a closure that libcore provides the exact same code via [`core::iter::from_fn`](https://doc.rust-lang.org/core/iter/fn.from_fn.html).

And, voila! From iterators to closures and back again.  
No magic tricks, no hidden allocs, no nothing.
```rust
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
```

## 1.4. The case for asynchronous Rust

And with that, our little demystifying tour of iterators and closures comes to an end.  
So, what was the point of all of this? What do iterators and closures have to do with anything?

Actually, beyond iterators and closures, what we've really looked at during this chapter are the various ways of expressing state-machines using Rust's native tools.  
Coincidentally, a lot (most?) of idiomatic Rust code comes down to just that: building up complex state machines by combining iterators and closures, and then polling these state-machines at the edge of the software, where errors will be dealt with properly.  

What's with asynchronous Rust, then? What can we express in async Rust that we couldn't convey with these tools? The answer is multiplexing.. kind of.

### 1.4.a. What are we trying to fix?

A standard iterator will hold an entire OS thread from the time it's polled and until it yields its next value. Whether this iterator actually does something useful with that OS thread is irrelevant.  
Consider this over-used example of an iterator that sends a packet to a station on Mars when it gets polled, and yields a welcoming message when an answer comes back from the network:
```rust
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
```
Once we start polling it, this iterator will keep hold of the underlying OS thread for as long as it takes for Mars to respond.  
Obviously, in this case, the overwhelming majority of the CPU time will be spent idling, waiting for data from the network.
```rust
let mut mars_com = PingMars;
for msg in mars_com { // Blocking an entire OS thread :(
    println!("received message from Mars: {}!", msg);
}
```

That's the textbook case of synchronous/blocking I/O, which has been thrown around and around for the last decades.

But the issue isn't really confined to I/O, is it?  
What about a program that has to sleep, e.g. to wait for an external piece of hardware to get ready?  
What about a program that is stuck waiting for an intra-program signal, e.g. a channel or a mutex?

It seems that what we're getting at is that the issue isn't specific to I/O, but rather generalizes to any non-CPU intensive code.  
Actually, I'd argue that it encompasses even more than "just" non-CPU intensive code.

What if we had a state-machine that needed to do some CPU-heavy computation on every poll, but we'd still very much like for it _not_ to hijack an entire OS thread until its next yield; i.e. we'd like to be able to pause the computation of a value at arbitrary points, so that another state-machine could make some progress of its own in the meanwhile.  
Heck, what if we were running on some kind of embedded platform that doesn't provide OS threads in the first place?  

Consider this (ridiculously bad) Fibonacci iterator:
```rust
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
```
For big enough values of `n`, every poll is going to take so much CPU-time to compute, maybe we'd rather let some other state-machine progress from time to time, hm?  
(Yes, Fibonacci is a very contrived example. Yes, memoization would be a much better solution in this case. Bear with me.)

The real issue here is neither blocking I/O, or non-CPU intensive code, or anything specific like that.  
The real issue simply is that we need a way to express _multiplexing_ as part of our state-machines, and more specifically as part of our polling mechanism.

Question: _Haven't we fixed that issue already, though? Like decades ago?_  
That's exactly what OS threads are for, multiplexing N programs onto M physical threads, and we've had those for who-knows how long.  
In fact they work so well that you can spawn millions of them without breaking a sweat on modern hardware.

Answer: _Yes, there is in fact nothing that you could express with Futures/Streams that you wouldn't be able to convey with Closures/Iterators and a bunch of good ol' OS threads._  
In fact, both Rust's stdlib and ecosystem offer very powerful tools for working with OS threads ([`crossbeam`](https://github.com/crossbeam-rs/crossbeam)) and multi-threaded iterators ([`rayon`](https://github.com/rayon-rs/rayon)); these tools should most likely always be your first weapon of choice, unless you fall into either of those two categories:
- You have hard performance constraints.  
Async code can achieve A) much better performance and B) more efficient CPU usage than OS threads thanks to the lack of context-switching overhead.  
At large enough scale, this will more than likely manifests itself as A) smoother tail latencies and B) much cheaper CPU bills.
- You have hard environment constraints.  
What if your platform simply doesn't provide OS threads? What if it does but you cannot use them for some reason (e.g. some determinism contraints)?

Of course, those gains don't come for free.  
As we'll see in the rest of this book, asynchronous Rust ships with a metric ton of added complexity, a tradeoff that may or may not be worth it depending on your constraints.

### 1.4.b. How'd we go about fixing it?

Let's take a minute to think about how'd we go about fixing the lack of multiplexing capability of closures and iterators.  

In the case of the `PingMars` iterator, the solution is obvious: we would need to make use of non-blocking I/O so that we could give back control of the OS thread to the poller in case the underlying network device isn't ready yet.  
Somehow, we'll also need to find a way to notify the poller when the underlying device finally turns ready again, otherwise how could they know when they should start polling again?

For the `Fibonacci` example, we'd need a way to give control of the thread back to the poller in case the current value is taking too long to compute (for an arbitrary definition of "too long").  
Interestingly, we don't need to ever notify the poller in this case: they're free to start polling again whenever they want, the iterator only released the OS thread for the sake of politeness anyway; i.e. it's always ready.  
What we're definitely going to need, though, is a way to know exactly where we stopped in the computation back when we released the OS thread, so that we can continue from that point on when the polling restarts.

We could go on and on, but already a pattern starts emerging here:
- The state-machine must be able to give back control of the OS thread to the poller, even from the middle of a polling cycle.
- The state-machine must have a way of notifying the poller when it's a good time to start polling again.
- The state-machine must keep track of the progress made during the last polling cycle, so that it can start again from there.

Say we were to take the definition of Iterator and encode those constraints in it, we'd probably end up with something like this:
```rust
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
```
Guess what, we've essentially just reinvented `Stream` (..almost)!

When you take Closures and Iterators, and engineer multiplexing-support into them, what you get back are Futures and Streams.  

### 1.4.c. Conclusion

**Asynchronous Rust is about expressing state-machines that can be multiplexed onto a single OS thread**.

The main reasons to do so are A) better overall performance and B) environment constraints, at the cost of a massive increase in complexity, both from a usage and implementation standpoints.

`Future`s and `Stream`s are logical extensions to closures and iterators, giving them the ability to be multiplexed onto a single OS thread.  
As we'll see, the four of them all share many of the same properties and design principles, which is why we've spent this entire chapter covering every last details of closures and iterators in the first place.

Iterators and closures are, as I like to say, the gateway drugs to Futures and Streams.  
In fact, as we'll see later in this book, these four can (and will be) all be expressed in terms of the mother of all state-machines: Generators.

***

# 2. Chapter II

TODO
