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

Now I'm sure that opening a book about asynchronous Rust with a whole chapter dedicated to iterators and closures might seem like a dubious choice but, as we'll see, iterators, closures, streams & futures are all very much alike in crucial ways.  
In fact, I simply cannot fathom how we'd go about demystifying asynchronous Rust without first lifting all of the magic behind iterators and closures. That might just be me, though.

Hence in this first chapter, we'll do just that, demystify iterators and closures:
- What they are and what they're not.
- Their memory representation and execution model.
- Their interesting similarities.
- And last but not least: why would we need and/or want something else to begin with?
