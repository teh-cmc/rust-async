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
{{#include ../../examples/chapter_1/src/lib.rs:range}}
```
that we used like this:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_range}}
```

Could we express `Range` in terms of a closure instead?  
Well of course we can, what's an iterator but a `FnMut() -> Option<T>` closure, really?
```rust
{{#include ../../examples/chapter_1/src/lib.rs:range_closure}}
```
which we can basically use the same way as an iterator:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_range_closure}}
```
But what about combinators, you say?

### 1.3.b. Closure combinators

Remember our `Range` iterator could be combined with a `Bounds` iterator, allowing us to express something like the following:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_bounds}}
```

We can apply the same pattern to closures: by moving ownership of the wrappee inside the state of the wrapper, we can delegate the state-machinery from the wrapper and into the wrappee.
```rust
{{#include ../../examples/chapter_1/src/lib.rs:bounds_closure}}
```
Again, using our closure combinator is pretty similar to using our iterator combinator:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_bounds_closure}}
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
{{#include ../../examples/chapter_1/src/lib.rs:test_bounds_ext}}
```

Well closures are a trait too, aren't they? Surely we can extend it!
```rust
{{#include ../../examples/chapter_1/src/lib.rs:bounds_ext_closure}}
```
Ta-daaaa!
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_bounds_ext_closure}}
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
{{#include ../../examples/chapter_1/src/lib.rs:bounds_ext}}
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
{{#include ../../examples/chapter_1/src/lib.rs:iter_to_closure}}
```
Going the other way is even more straightforward:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:closure_to_iter}}
```
And in fact, it seems so natural to want to express an iterator as a closure that libcore provides the exact same code via [`core::iter::from_fn`](https://doc.rust-lang.org/core/iter/fn.from_fn.html).

And, voila! From iterators to closures and back again.  
No magic tricks, no hidden allocs, no nothing.
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_iter_to_closure_to_iter}}
```
