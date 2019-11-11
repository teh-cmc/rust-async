## 1.2. Closures

While iterators are pretty straightforward both from a usage and an implementation standpoint, closures are anything but.  
In fact, I'd argue they're one of the most complex pieces of "standard" synchronous Rust.  
Their very expressive nature, thanks to a lot of magical sugar exposed by the compiler, make them a prime tool to push the type system into very complex corners, whether voluntarily.. or not.

Closures also happen to be the cornerstone of any serious asynchronous codebase, where their incidental complexity tends to skyrocket as a multitude of issues specific to asynchronous & multi-threaded code join in on the party.

### 1.2.a. A better `Bounds`

We'll kick off this section by turning our `Bounds` filter into a filter of.. well, anything, really:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:filter}}
```
Might as well provide an extension for it too:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:filter_ext}}
```
Lo and behold:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_filter_ext}}
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
{{#include ../../examples/chapter_1/src/lib.rs:handmade_decl}}
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
{{#include ../../examples/chapter_1/src/lib.rs:test_handmade_decl}}
{{#include ../../examples/chapter_1/src/lib.rs:test_handmade_native}}
```
And so is this one:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_handmade_decl}}
{{#include ../../examples/chapter_1/src/lib.rs:test_handmade_native_move}}
```
It doesn't matter that the second closure moves `a` & `b` into its state (well it certainly matters to the enclosing scope, which can't refer to these variables anymore, but that's besides the point).

What matters is how the closure interacts with its state when it gets called.  
In the example above, that interaction is just a read through a reference, and so a shared reference to the state (i.e. `&self`) is enough to perform the call: the compiler makes sure that this closure is `Fn`.

Now if you were to do this on the other hand..:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_handmade_illegal}}
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
{{#include ../../examples/chapter_1/src/lib.rs:fnmut_as_fnonce}}
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
{{#include ../../examples/chapter_1/src/lib.rs:fn_as_fnmut_as_fnonce}}
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
{{#include ../../examples/chapter_1/src/lib.rs:handmade_decl}}
```
Our closure only references its environment: it never modifies it nor does it ever move it somewhere else, therefore the most versatile implementation that we can provide is `Fn`, which should allow it to be run pretty much anywhere.  
As we've seen, `Fn` is a supertrait of `FnMut` is a supertrait of `FnOnce`, and so we need to implement the entire family tree in this case:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:handmade_impl}}
```
Lo and behold, we've got ourselves a closure:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_handmade_decl}}
{{#include ../../examples/chapter_1/src/lib.rs:test_handmade}}
```

So that's great and all, but it still doesn't explain why this:
```rust
{{#include ../../examples/chapter_1/src/lib.rs:test_filter_ext}}
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
{{#include ../../examples/chapter_1/src/lib.rs:test_empty_closure}}
```

That explains why our closure is 0 byte, but it certainly doesn't explain why a `Filter<Range<usize>, [closure]>` is the same size as a `Range<usize>`. Even if the closure itself is 0 byte, `Filter` still has has to hold a function pointer to the code portion of the closure, which is 8 bytes on a 64bit platform such as mine.  
What are we missing?

Consider the following code where we instantiate a `Filter` using an empty closure (i.e. an anonymous function):
```rust
{{#include ../../examples/chapter_1/src/bin/closure_filters.rs:empty_closure}}
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
{{#include ../../examples/chapter_1/src/bin/closure_filters.rs:capturing_closure}}
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
