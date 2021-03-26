<p align="center">
<!-- <img width="250px" src="https://i.imgur.com/kTap42K.png"/> -->
    <img width="250px" src="https://i.imgur.com/CZEkdK1.png"/>
</p>

[![Gitter](https://badges.gitter.im/talk-about-emu/thoughts.svg)](https://gitter.im/talk-about-emu/thoughts?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)
[![](http://meritbadge.herokuapp.com/em)](https://crates.io/crates/em)
[![](https://docs.rs/em/badge.svg)](https://docs.rs/em)

Emu is a framework/compiler for GPU acceleration of Rust, GPU programming. It is a procedural macro that accept pure, safe Rust code as input, identifies portions to attempt to accelerate, and automatically writes in code to run portions on the GPU instead of the CPU.

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
        x[i] = x[i] * 10.0;
    }

    gpu_do!(read(x)); // read data back from the GPU
    println!("{:?}", x);
}
```

# usage

You can use Emu in your Rust projects by doing the following-

1. Add `em = "0.3.0"` to `Cargo.toml`
2. Confirm that an OpenCL library [is installed]() for your platform

Learn how to get started with Emu by looking at [the documentation](https://docs.rs/em).
