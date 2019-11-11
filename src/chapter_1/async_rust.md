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
{{#include ../../examples/chapter_1/src/lib.rs:ping_mars}}
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
{{#include ../../examples/chapter_1/src/lib.rs:fib}}
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
{{#include ../../examples/chapter_1/src/lib.rs:multiplexed_iter}}
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
