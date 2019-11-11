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
{{#include ../../examples/chapter_1/src/lib.rs:range}}
```
In action:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_range}}
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
{{#include ../../examples/chapter_1/src/lib.rs:bounds}}
```

In action:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_bounds}}
```

Once again, no magic here.  
`Bounds` simply takes ownership of a `Range`, and then the wrapper (`Bounds`) is free to delegate the polling to the wrappee (`Range`), injecting its own rules in the process (i.e. filtering values, in this case).

From the programmer's point-of-view, nothing changes: we get a _something_ that implements the `Iterator` trait and we can poll it as needed.  
Thanks to monomorphization, everything is still sitting on the stack here; we just have a bigger, self-contained struct is all:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_size}}
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
{{#include ../../examples/chapter_1/src/lib.rs:bounds_ext}}
```
And just like that, we're able to express our intent in a much more natural fashion:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_bounds_ext}}
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
