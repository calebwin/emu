![a picture of a real-world emu](https://i.imgur.com/vijwEix.jpg)
# the emu programming language
<!--# The Emu Programming Language-->
<!--# Emu is a language for programming GPUs-->
<!--# the emu programming language-->
Emu is a language, framework for programming GPUs. Emu and Rust together let you write expressive code that compiles to efficient binaries to perform hardware-accelerated numerical computation.

[![](https://img.shields.io/crates/d/em.svg)](https://crates.io/crates/em) [![](https://img.shields.io/crates/v/em.svg)](https://crates.io/crates/em) [![](https://img.shields.io/crates/l/em.svg)](https://crates.io/crates/em) [![](https://img.shields.io/gitter/room/calebwin/emu-yellowgreen.svg)](https://gitter.im/talk-about-emu/thoughts) [![](https://img.shields.io/badge/book-v0.2.0-yellow.svg)](https://github.com/calebwin/emu/tree/master/book#table-of-contents) <!--[![Sourcegraph](https://sourcegraph.com/github.com/calebwin/emu/-/badge.svg)](https://sourcegraph.com/github.com/calebwin/emu?badge)-->

- **`0. Overview...........................`** [**`goto`**](https://github.com/calebwin/emu/new/master/docs#Example)
- **`1. Example............................`** [**`goto`**](https://github.com/calebwin/emu/new/master/docs#Example)
- **`2. Parallelism is implicit............`** [**`goto`**](https://github.com/calebwin/emu/new/master/docs#Parallelism-is-implicit)
- **`3. Code is procedurally generated.....`** [**`goto`**](https://github.com/calebwin/emu/new/master/docs#Code-is-procedurally-generated)
- **`4. You work with functions............`** [**`goto`**](https://github.com/calebwin/emu/new/master/docs#You-work-with-functions)
- **`5. You work with components...........`** [**`goto`**](https://github.com/calebwin/emu/new/master/docs#You-work-with-components)
- **`6. Usage..............................`** [**`goto`**](https://github.com/calebwin/emu/new/master/docs#Usage)

# an example

It's often nice to start off with a quick example. Here's an example where you have a bunch of data, a function for multiplying, a function for taking sigmoid and you want to call the multiplication and sigmoid on the data.

```rust
// The "emu!" macro accepts a chunk of Emu code and
// generates Rust functions that can be called to perform computation on the GPU
emu! {

    // Multiply any element in given data by given coefficient
    // Data and coefficient must be floats
    function multiply(data [f32], coeff f32) {
        data[..] *= coeff;
    }
    
    // Apply sigmoid function to any element in given data
    // Data must be floats
    function sig(data [f32]) {
        let elem: f32 = data[..];
        let res: f32 = 1 / (1 + pow(E, -elem));
        data[..] = res;
    }
    
    /// Multiplies each element in given data by given coefficient
    pub fn multiply(data: &mut Vec<f32>, coeff: &f32);
    /// Applies sigmoid to each element in given data
    pub fn sig(data: &mut Vec<f32>);
    
}
 
fn main() {
    // Vector of data to be operated on
    let mut my_data = vec![0.9, 3.8, 3.9, 8.2, 2.5];
    
    // Multiply data by 10 and take sigmoid
    multiply(&mut my_data, &10.0)
    sig(&mut my_data);
}
```

# parallelism is implicit

Emu lets you forget all about what GPUs do to make your code run fast. All you have to do is write your data computation algorithm as you normally would and then just put holes in your code. All the parallelism and multi-threading is implicit. Here's what a matrix multiplication function would look like.

```rust
// A function that operates on 2 given matrices (a and b) and puts result in given c
function multiply_square_matrices(m i32, n i32, k i32, a [f32], b [f32], c [f32]) {
  // current row and
  // current column
  let row: i32 = enumerate(a)[..];
  let col: i32 = enumerate(a)[..];

  // perform algorithm at current row and column
  let acc: f32 = 0.0;
  for i in 0..k {
    acc += a[i*m + row] * b[col*k + i];
  }
  
  // store result
  c[col * m + row] = acc;
}
```

Emu will automatically insert calls to appropriate work functions that the OpenCL API provides. In the above matrix multiplication example, the first hole would be filled with `get_global_id(0)` and the second hole would be filled with `get_global_id(1)`.

# code is procedurally generated

Emu procedurally generates a lot of code you would otherwise have to write. And it all happens at compile time. Without Emu, you would have to write a lot of code.

```rust
extern crate ocl;
use ocl::ProQue;

fn trivial() -> ocl::Result<()> {
    let src = r#"
        __kernel void add(__global float* buffer, float scalar) {
            buffer[get_global_id(0)] += scalar;
        }
    "#;

    let pro_que = ProQue::builder()
        .src(src)
        .dims(1 << 20)
        .build()?;

    let buffer = pro_que.create_buffer::<f32>()?;

    let kernel = pro_que.kernel_builder("add")
        .arg(&buffer)
        .arg(10.0f32)
        .build()?;

    unsafe { kernel.enq()?; }

    let mut vec = vec![0.0f32; buffer.len()];
    buffer.read(&mut vec).enq()?;

    println!("The value at index [{}] is now '{}'!", 200007, vec[200007]);
    Ok(())
}
```

In fact, this example is pulled directly from the most popular Rust binding to OpenCL - ocl (which, as you will see, is used behind the scenes by Emu). And ocl actually does quite a bit shown above to remove complexity and abstract away things when doing GPU programming. The above script would be even more complicated if you used the more low-level bindings the crate provides for working with OpenCL.

But there is still quite a bit of stuff here that I don't want to know what they are. What are dims/dimensions? What are buffers? I just have data and function and want to compute. Emu takes advantage of a really cool thing that Rust has: procedural macros. Procedural macros are kind of like functions; except instead of doing computation, they generate code. So what Emu provides is a single one of these procedural macros - called `emu!` that you can pass in a chunk of Emu code and have it generate a bunch of Rust code for you.

```rust
emu! {
    function add(buffer [f32], scalar f32) {
        buffer[..] += scalar;
    }
    
    fn add(buffer: &mut Vec<f32>, scalar: &f32);
}
```

What this generates is a Rust function with the signature `fn add(buffer: Vec<f32>, scalar: f32);` that you can then call to do the computation as described in the Emu function defined above. You can then call this Rust function very simply.

```rust
fn main() {
    let mut my_data = vec![0.0, 3.0, 2.5, 2.7, 3.2, 3.9, 1.3, 9.8];
    add(&mut my_data, &100.0);
}
```

# you work with functions

Emu lets you work with functions instead of kernel source code.

```rust
let src = r#"
    __kernel void add(__global float* buffer, float scalar) {
        buffer[get_global_id(0)] += scalar;
    }
"#;
```

Previously, you would have to work with something like the above, a multiline string containing the source code of the code you want to run. But using the procedural code generation explained in the last section, you instead have a Rust function you can call to do stuff.

```rust
// signature of WIP function that adds on GPU
fn add(buffer: &mut Vec<f32>, scalar: f32);

// signature of reference function that adds on CPU
fn add_on_cpu(buffer: &mut Vec<f32>, scalar: f32);
```

By letting you work immediately with functions instead of kernel code, you can adopt something similar to the following process of software development for GPUs. It's a simple variation on waterfall method.

```
1. define requirements of software
2. define design of sofwater
3. develop reference implementation for CPU
4. develop tests
5. develop implementation for GPU
6. maintenance
```

# you work with components

Emu lets you work with components of software that can neatly work together. Particularly, there are 3 things Emu provides that facilitate component-based software engineering: (1) importing/exporting components, (2) testing components, (3) documenting components.

## importing/exporting components

Since Emu generates Rust functions, you can import and export them in the same way as any other Rust function. You can tell Emu to generate a public Rust function by prefixing the function signature with the `pub` keyword.

```rust
pub fn add(buffer: &mut Vec<f32>, scalar: f32);
```

Optionally, you can then package your function into a Rust crate and publish it on crates.io so it can be easily imported into other crates that people develop. The really nice thing about being deeply embedded in Rust is that Emu can take advantage of the ecosystem (e.g. - cargo, crates.io) to make it easier to work with components of software.

## documenting components

Emu makes writing documentation easy. The way you write documentation for Rust functions is the same way you write documentation for the Rust functions that Emu generates.

```rust
/// Adds given scalar to given Vec of data
pub fn add(buffer: &mut Vec<f32>, scalar: f32);
```

Rust's Cargo tool can then generate pretty HTML documentation when invoked with `cargo doc`.

## testing components

Because Emu is generating Rust functions, you can test the generated Rust functions in the same way you would test Rust functions you write by hand. 

```rust
#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_add_hundred() {
        let mut dummy_data = vec![0.0, 3.0, 2.5, 2.7, 3.2, 3.9, 1.3, 9.8];
        add(&mut dummy_data, &100.0);
    
        assert_eq!(dummy_data, vec![100.0, 103.0, 102.5, 102.7, 103.2, 103.9, 101.3, 109.8]);
    }
}
```

You can then run your tests by invoking `cargo test`.

# usage

Emu is being designed for use in a project with data-intensive computation. Emu can be included in any Rust project with the following. The only other dependency is OpenCL so make sure you have that installed - https://www.eriksmistad.no/getting-started-with-opencl-and-gpu-computing/.

```toml
[dependencies]
em = "0.2.0"
```
