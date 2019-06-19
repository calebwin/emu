# Table of contents
- Chapter 0 - [Introduction](https://github.com/calebwin/emu/tree/master/book/introduction.md#table-of-contents)
- Chapter 1 - [Language](https://github.com/calebwin/emu/tree/master/book/language.md#table-of-contents)
- Chapter 2 - [Types](https://github.com/calebwin/emu/tree/master/book/types.md#table-of-contents)
- Chapter 3 - [Holes](https://github.com/calebwin/emu/tree/master/book/holes.md#table-of-contents)
- Chapter 4 - [Numbers](https://github.com/calebwin/emu/tree/master/book/numbers.md#table-of-contents)

# Holes
Most functions you write in the Emu language will be written with holes in them. Here's an example.
```rust
function foo(x [f32]) {
    x[..] *= 10;
    x[..] += 1;
}
```
These holes in the program let you define computation symbolically. The function takes in an array as input but the computation it does can be different depending on what values fill the holes.
But what's the point? To get Emu to actually generate a useful Rust function, you write the following.
```rust
fn foo(x: &mut Vec<f32>);
```
The Rust function that Emu generates has the above signature and - importantly - what it does is it calls the Emu function multiple times filling the holes each time with all possible values.

At a more conceptual level, you define your Emu functions to be "for any" and they get compiled to Rust functions that are "for each". And by having Emu force you to write functions that are "for any", it turns out the functions you write are easy for the GPU to use to operate on data in parallel.
