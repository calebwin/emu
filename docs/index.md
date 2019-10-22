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

Emu lets you work with 
