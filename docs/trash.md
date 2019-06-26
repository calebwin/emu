<!--# ![a picture of a real-world emu](https://i.imgur.com/nwfkPIx.jpg) Emu -->
<!--# The Emu Programming Language-->
<!--# Emu is a language for programming GPUs-->
# emu
[![](https://img.shields.io/crates/d/em.svg)](https://crates.io/crates/em) [![](https://img.shields.io/crates/v/em.svg)](https://crates.io/crates/em) [![](https://img.shields.io/crates/l/em.svg)](https://crates.io/crates/em) [![](https://img.shields.io/gitter/room/calebwin/emu-yellowgreen.svg)](https://gitter.im/talk-about-emu/thoughts) [![](https://img.shields.io/badge/book-v0.2.0-yellow.svg)](https://github.com/calebwin/emu/tree/master/book#table-of-contents)

Emu is a small language for programming GPUs to do big data computation.

# Programs with holes

Emu lets you forget all about what GPUs do to make your code run fast. All you have to do is write your data computation algorithm as you normally would and then just put holes in your code. All the parallelism and multi-threading is implicit. Here's what a matrix multiplication function would look like.

```rust
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

# Functions, not kernels

Emu lets you work with functions instead of kernel source code. Normally you would write your kernel source code, store it in a string and then try to run it. In Rust, it would look something like the following.
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

In fact, this example is pulled directly from the most popular Rust binding to OpenCL - ocl (which, as you will see, is used behind the scenes by Emu). ocl does quite a bit to remove complexity when doing GPU programming. But there is still quite a bit of stuff here that I don't want to know what they are. What are dims? What are buffers? I just have data and function and want to compute.


- `cargo build` for importing functions
- `cargo test` for testing functions
- `cargo doc` for documenting functions
- `rustc` for validating and parsing function code
- `rustc` for verifying safe data transfer to GPU
- `rustc` for familiar error reporting
- `crates.io` for hosting function code
- `docs.rs` for hosting function documentation

...that let it provide a far more streamlined system for programming GPUs. Consequently, Emu makes Rust ideal - compared to Python/Julia/C++ - for writing minimalistic programs that do robust, data-intensive computation.

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
```
```rust
fn main() {
    // Vector of data to be operated on
    let mut my_data = vec![0.9, 3.8, 3.9, 8.2, 2.5];
    
    // Multiply data by 10 and take sigmoid
    multiply(&mut my_data, &10.0)
    sig(&mut my_data);
}

```

To get started programming GPUs with Emu, check out [**the book**](https://github.com/calebwin/emu/tree/master/book#table-of-contents), [**the examples**](https://github.com/calebwin/emu/tree/master/examples), and [**the crate**](https://crates.io/crates/em) itself.
