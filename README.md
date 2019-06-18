![a picture of a real-world emu](https://i.imgur.com/8CeUiar.jpg) 

# The Emu Programming Language
![](https://img.shields.io/crates/d/em.svg) ![](https://img.shields.io/crates/v/em.svg) ![](https://img.shields.io/crates/l/em.svg)

Emu is a high-level language for programming GPUs.

```rust
emu! {
    // Multiply any element in given data by given coefficient
    function multiply(data [f32], coeff f32) {
        data[...] *= coeff;
    }
    
    // Apply sigmoid function to any element in given data
    function sig(data [f32]) {
        let elem: f32 = data[...];
        let res: f32 = 1 / (1 + E ^ -elem);
        data[...] = res;
    }

    /// Multiplies each element in given data by given coefficient
    fn multiply(data: &mut Vec<f32>, coeff: &f32);
    /// Applies sigmoid to each element in given data
    fn sig(data: &mut Vec<f32>);
}
```
Emu - unlike OpenCL/CUDA/Halide/Futhark - is embedded in Rust and takes advantage of the ecosystem (cargo build, cargo test, cargo doc, rustc, crates.io, docs.rs) in ways that let it provide a far more streamlined system for programming GPUs. Emu makes Rust ideal (compared to Python/Julia/C++) for writing minimalistic programs that do robust, data-intensive computation.
```rust
fn main() {
    // Vector of data to be operated on
    let mut my_data = vec![0.9, 3.8, 3.9, 8.2, 2.5];
    
    // Multiply data by 10 and take sigmoid
    multiply(&mut my_data, &10.0)
    sig(&mut my_data);
}
```

To get started programming GPUs with Emu, check out [**the book**](https://github.com/calebwin/emu/tree/master/book#the-emu-book), [**the examples**](https://github.com/calebwin/emu/tree/master/examples), the showcase, the tutorials, and [**the crate**](https://crates.io/crates/em) itself.
