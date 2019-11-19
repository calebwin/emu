<p align="center">
<!-- <img width="250px" src="https://i.imgur.com/kTap42K.png"/> -->
    <img width="250px" src="https://i.imgur.com/CZEkdK1.png"/>
</p>

[![Gitter](https://badges.gitter.im/talk-about-emu/thoughts.svg)](https://gitter.im/talk-about-emu/thoughts?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)
[![](http://meritbadge.herokuapp.com/em)](https://crates.io/crates/em)
[![](https://docs.rs/em/badge.svg)](https://docs.rs/em)

Emu is a framework/compiler for GPU acceleration of Rust, GPU programming. It is a procedural macro (`#[gpu_use]`) that walks through your Rust code and off-loads parts of it to the GPU. It looks for annotations (`gpu_do!()`) embedded in your code to know where to add GPU-specific action (like moving data and launching computation).

Ultimately, Emu helps you develop single-source, GPU-accelerated applications in Rust, taking advantage of Rust's tooling, ecosystem, and safety guarantees.

# features

- ease of use
  - download a library, not a whole new compiler
  - work with `cargo test`, `cargo doc`, `crates.io`
  - work with `rustfmt`, `racer`, `rls`
  - switch between CPU and GPU with 1 line
  - seamlessly drop down to low-level OpenCL, SPIR-V
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
    
    gpu_do!(load(x)); // move data to the GPU
    gpu_do!(launch()); // off-load to run on the GPU
    for i in 0..1000 {
        x[i] = x[i] * 10.0;
    }
    gpu_do!(read(x)); // move data back from the GPU
    
    println!("{:?}", x);
}
```

# usage

You can use Emu in your Rust projects by doing the following-

1. Add `em = 0.3.0` to `Cargo.toml`
2. Confirm that an OpenCL library [is installed]() for your platform

Learn how to get started with Emu by looking at [the documentation](https://docs.rs/em).

# contributing

Emu currently works very well (robust, well-documented, OK-ish baseline performance) but only with a small subset of Rust. The roadmap for what to do next is pretty straightforward - expand that subset of Rust that we look at. Here is an up-to-date (but not necessarily complete) list of things to work on.

- [ ] Constant address space by default
- [ ] Data race safety with Rayon
- [ ] Multiple GPU usage
- [ ] Multiple thread usage (from host)
- [ ] Support for methods (details in [`CONTRIBUTING.md`](https://github.com/calebwin/emu/blob/master/CONTRIBUTING.md))
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
