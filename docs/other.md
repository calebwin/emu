Emu is a free, open-source library for general-purpose GPU programming. It aims to provide a productive single-source environment for GPGPU programming while guaranteeing safety and enabling big speedups.

## Low effort, big speedup

Emu lets you start with sequential pure Rust software that runs on CPU and add just a few lines of code to have portions be automatically off-loaded to the GPU. For example, we can start with a simple scalar-vector multiplication.

```rust
fn main() {
    let mut x = vec![0.1; 1000];
    for i in 0..1000 {
        x[i] = x[i] * 5.0;
    }
}
```

We can then add a few lines of code to declare things the GPU should do - things like moving data, launching computation.

```rust
fn main() {
    let mut x = vec![0.1; 1000];
    
    gpu_do!(load(x));
    gpu_do!(launch());
    for i in 0..1000 {
        x[i] = x[i] * 5.0;
    }
}
```

Finally, we can tag the function with `#[gpu_use]` to tell Emu to interpret the declarations and off-load the for loop to the GPU.

```rust
#[gpu_use]
fn main() {
    let mut x = vec![0.1; 1000];
    
    gpu_do!(load(x));
    gpu_do!(launch());
    for i in 0..1000 {
        x[i] = x[i] * 5.0;
    }
}
```

## Fewer bugs, more productivity

Emu also wants to make GPU programming safer and more productive. We make programming safer by eliminating entire classes of runtime errors.

- No null pointer errors
- No type mismatch errors
- No syntax errors

We also make GPGPU more productive.

- Download a software library (just 1 line of TOML), not an entirely different compiler
- Switch between CPU and GPU (just 1 line of Rust)
- Boilerplate code is inferred from your code, automatically generated
- Testing and documentation with standard tools (`cargo test`, `cargo doc`, `Crates.io`)

## Comparison

If you're coming from a different GPU technology, here are some of the ways in which Emu may be better for GPGPU than what you're using right now.

| What you currently use...                                    | Emu...                 |
| ------------------------------------------------------------ | ---------------------- |
| **OpenCL** allows null-pointer, type mismatch, syntax errors     | Fewer errors           |
| **OpenACC** requires a seperate compiler, allows null-pointer errors | Just a library         |
| **CUDAnative.jl** requires understanding of threads, memory hierarchy | Just for loops         |
| **CUDA** requires understanding ^ and a seperate compiler        | Nope                   |
| **ArrayFire** provides a limited set of composable functions     | Just write code        |
| **Numba** has slow base-line                                     | Base-line is fast LLVM |



## Getting started

You can get started with Rust + Emu by doing the following-

1. Add `em = 0.3.0` to `Cargo.toml` (in the folder of your Rust project)
2. Confirm that an OpenCL library [is installed]() for your platform

Learn more about Emu by looking at [the documentation](https://docs.rs/em).
