# Demystifying Asynchronous Rust

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
