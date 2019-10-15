<p align="center">
<img width="250px" src="https://i.imgur.com/kTap42K.png"/>
</p>

[![Gitter](https://badges.gitter.im/talk-about-emu/thoughts.svg)](https://gitter.im/talk-about-emu/thoughts?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)

Emu is a framework for accelerating code by using GPUs. It is a procedural macro that accept pure, safe Rust code as input, identifies portions to attempt to accelerate, and automatically writes in code to run portions on the GPU instead of the CPU.

# features

- ease of use
    - download a library, not a whole new compiler
    - work with `cargo test`, `cargo doc`, `crates.io`
    - work with `rustfmt`, `racer`, `rls`
    - switch between CPU and GPU with 1 line
- safety guarantees
    - no null pointer errors
    - no type mismatch errors
    - no syntax errors
- more fun
    - up to 80% less code
    - up to 300x speedup
    - as fast as single-GPU, single-threaded, idiomatic usage of OpenCL

# an example

```rust
#[macro_use]
extern crate em;
use em::*;

#[gpu_use]
fn main() {
    let mut x = vec![0.0; 1000];
    gpu_do!(load(x)); // load data to the GPU

    // this code gets identified as accelerate-able
    // the compiler will insert calls to OpenCL to run this on the GPU
    gpu_do!(launch());
    for i in 0..1000 {
        x[i] = x[i] * 10;
    }

    gpu_do!(read(x)); // read data back from the GPU
    println!("{:?}", x);
}
```

# contributing

Emu currently works very well (robust, well-documented, OK-ish baseline performance) but only with a small subset of Rust. The roadmap for what to do next is pretty straightforward - expand that subset of Rust that we look at. Here is an up-to-date (but not necessarily complete) list of things to work on.

- [ ] Constant address space by default
- [ ] Data race safety with Rayon
- [ ] Multiple GPU usage
- [ ] Support for functions with `&self` or `&mut self`
- [ ] Support for block algorithms
- [ ] Support for reduction algorithms
- [ ] Support for `for x in &data`
- [ ] Support for `for x in &mut data`
- [ ] Support for variables
- [ ] Support for if statements
- [ ] Support for if/else-if/else statements
- [ ] Support for all Rust with NVPTX
- [ ] *insert your super-cool idea here*

We want people to be able to implement all sorts of cool things (simulations, AI, image processing) with Rust + Emu. If you are excited about building a framework for accelerating Rust code with GPUs, create a GitHub issue for whatever you want to work on and/or discuss on Gitter.
