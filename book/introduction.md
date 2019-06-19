# Table of contents
- Chapter 0 - [Introduction](https://github.com/calebwin/emu/tree/master/book/introduction.md#table-of-contents)
- Chapter 1 - [Language](https://github.com/calebwin/emu/tree/master/book/language.md#table-of-contents)
- Chapter 2 - [Types](https://github.com/calebwin/emu/tree/master/book/types.md#table-of-contents)
- Chapter 3 - [Holes](https://github.com/calebwin/emu/tree/master/book/holes.md#table-of-contents)
- Chapter 4 - [Numbers](https://github.com/calebwin/emu/tree/master/book/numbers.md#table-of-contents)

# Introduction

Emu is a language for programming GPUs. Unlike other GPU programming languages such as OpenCL/Halide/Futhark, Emu is embedded in Rust, leveraging the language's ecosystem to provide a more streamlined GPU programming experience.

This book assumes familiarity with the Rust language. As a Rust crate, Emu provides a single procedural macro that accepts a chunk of Emu code and generates Rust functions that can be called to execute computation on the GPU.

You can get started by adding the following to your `Cargo.toml` file.

```toml
[dependencies]
em = "0.2.0"
```

Then add the following to the file where you would like to use the `emu!` macro.

```rust
extern crate em;
use em::emu;
```

You are now ready to start writing Emu functions yourself. In Emu, there are two types of functions, ones that just do stuff and ones that do stuff and then also return some sort of result. If your function just does stuff, you can write it easily with the `emu!` macro.

```rust
emu! {
  multiply(data [f32], coeff f32) {
    data[..] *= coeff;
  }
}
```

The second (and only other) kind of function in Emu is the kind the returns a value. The syntax for doing this is pretty straightforward.

```rust
emu! {
  multiply(data [f32], coeff f32) {
    data[..] *= coeff;
  }
  
  collapse(x f32, y f32, z f32) f32 {
    return x + y + z;
  }
}
```

But what you've done so far will only define Emu functions. To get `emu!` to generate Rust functions that you can then call to execute computation on GPUs, you will need to add one more thing.

```rust
emu! {
  multiply(data [f32], coeff f32) {
    data[..] *= coeff;
  }
  
  collapse(x f32, y f32, z f32) f32 {
    return x + y + z;
  }
  
  fn multiply(data: &mut Vec<f32>, coeff: &f32);
}
```
This is essentially the signature of the Rust function you want to generate. One thing you should notice is that the parameters to the Rust functions you generate must be mutable references to `Vec` for arrays and references to scalar value types for scalar. Also, the names and positions of the parameters match those of the Emu function.

Now that you have a generated Rust function, there are a few more cool things you can do. You can make your functions public.
```rust
// private function
fn multiply(data: &mut Vec<f32>, coeff: &f32);

// public function
pub fn multiply(data: &mut Vec<f32>, coeff: &f32);
```

This makes it possible to create entire libraries of functions that run on the GPU. Such libraries of functions can be hosted online with `crates.io` and included in seperate projects with `cargo build`. You can also add documentation comments to the function.
```rust
/// Multiplies each number in given data by given coefficient
pub fn multiply(data: &mut Vec<f32>, coeff: &f32);
```

You can then use `cargo doc` to generate HTML documentation for functions. Also, note that these are ordinary Rust functions that are generated with signatures exactly matching what you specify. So you can do anything with them that you would typically do with ordinary Rust functions including writing tests.
