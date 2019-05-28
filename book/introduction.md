# Introduction
Emu is a language for embedding GPU-accelerated numerical computation in Rust. Particulary, it provides a single procedural macro called `emu!` for writing functions in the Emu language. All of the code for your functions gets automatically translated into OpenCL code at compile time and the code is stored in a `const` `&'static str` global constant called `EMU`. You can call functions using a binding to Opencl such as [`ocl`](https://github.com/cogciprocate/ocl) or [`rust-opencl`](https://github.com/luqmana/rust-opencl) and functions can also call each other.

You can get started by adding the following to your `Cargo.toml` file...
```toml
[dependencies]
em = "0.1.0"
```
Then add the following to the file where you would like to use the `emu!` macro...
```rust
extern crate em;
use em::emu;

emu! { }
```
Functions that don't return something are structured as follows...
```rust
emu! {
	move_particles(particles [f32]) {
		// more code here...
	}
}
```
As you can see there are three main parts - (1) the name, (2) the parameters of the function, (3) the body code of the function. Function that do returns something are pretty similar...
```rust
emu! {
	collapse(x f32, y f32, z f32) f32 {
		return x + y + z
	}
}
```
The next few chapters go into more details on the parameters and body of functions you can write in Emu.
