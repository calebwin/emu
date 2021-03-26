//! Emu is a framework/compiler for GPU acceleration, GPU programming. It is
//! first and foremost a procedural macro that looks at a subset of safe Rust
//! code and attempts to offload parts of it to a GPU. As an example of how you could use Emu, let's start off with a simple
//! vector-by-scalar multiplication. We can implement this in pure Rust as
//! follows.
//! ```
//! fn main() {
//!     let mut data = vec![0.1; 1000];
//!
//!     for i in 0..1000 {
//!         data[i] = data[i] * 10.0;
//!     }
//! }
//! ```
//! Emu let's you run parts of your program on the GPU by declaring
//! things you want the GPU to do. These declarations can tell the GPU to
//! load data, launch computation, etc. Here are appropriate declarations for
//! this program.
//! ```
//! # extern crate em;
//! # use em::*;
//! fn main() {
//!     let mut data = vec![0.1; 1000];
//!
//!     gpu_do!(load(data));
//!     gpu_do!(launch());
//!     for i in 0..1000 {
//!         data[i] = data[i] * 10.0;
//!     }
//!     gpu_do!(read(data));
//! }
//! ```
//! But these declarations don't actually do anything. They are really just
//! declarations. To actually have Emu interpret these declarations and do
//! something, we need to be able to tell Emu to use the GPU for a piece of
//! code. We do this by tagging the function we are working on with
//! `#[gpu_use]`. Here's how to do that.
//! ```
//! # extern crate em;
//! # use em::*;
//! #[gpu_use]
//! fn main() {
//!     let mut data = vec![0.1; 1000];
//!
//!     gpu_do!(load(data));
//!     gpu_do!(launch());
//!     for i in 0..1000 {
//!         data[i] = data[i] * 10.0;
//!     }
//!     gpu_do!(read(data));
//! }
//! ```
//! And now Emu will actually look through your code, load data onto the GPU,
//! launch code on the GPU, and read back from the GPU. This example should
//! give you a sense of how you can use Emu and what Emu tries to do.
//! Ultimately, what Emu comes down to is to parts.
//! 1. Passing - passing the GPU around (function to function), with `#[gpu_use]`
//! 2. Accelerating - using the GPU to load/read data, launch with `gpu_do!()`
//!
//! You've actually seen both passing and accelerating in the above example.
//! But to get a better idea of how to do passing and accelerating you should
//! look at the documentation for `#[gpu_use]` and `gpu_do!()` respectively.
//! 1. Passing - look at docs for `#[gpu_use]`
//! 2. Accelerating - look at docs for `gpu_do!()`
//!
//! Once you understand, passing and accelerating, you understand Emu. These
//! are the main high-level ideas of GPU programming with Emu. Looking at their
//! documentation should help you understand them better.

pub use emu_macro::gpu_use;
pub use ocl;

/// A container that holds information needed for interacting with a GPU using OpenCL.
///
/// You should really only use this if you intend to drop down to low-level OpenCL for maximum performance
/// Buffers and programs are stored in hash tables. Programs are indexed by their source code.
/// Buffers are indexed by a `*const [f32]`. Given a value `data`, you can get the `*const [f32]` index with `get_buffer_key!(data)`.
///
/// Note that `data` must have an `as_slice()` method defined for its type. As an example `data` could be of type `Vec`.
pub struct Gpu {
    pub device: ocl::Device,
    pub context: ocl::Context,
    pub queue: ocl::Queue,
    pub buffers: std::collections::HashMap<*const [f32], ocl::Buffer<f32>>,
    pub programs: std::collections::HashMap<String, ocl::Program>, // TODO cache kernels instead of programs if possible
                                                                   // kernels can be cached instead of programs, if it is easy to change the dims and args of a kernel
}

/// A macro for getting key to access a `Buffer` in the `buffers` field of a `Gpu`.
///
/// Given a value `data`, you can get the `*const [f32]` index with `get_buffer_key!(data)`.
/// Note that `data` must have an `as_slice()` method defined for its type. As an example `data` could be of type `Vec`.
/// This should really only be used if you want to drop down to low-level OpenCL for maximum performance gain.
///
/// Here's a quick example.
/// ```
/// # extern crate em;
/// # use em::*;
/// #[gpu_use] // this inserts a "let gpu = Gpu { ... };" at the start of the main function
/// fn main() {
///     let data = vec![0.0; 1000];
///     gpu_do!(load(data));
///     let buffer: &ocl::Buffer<f32> = gpu.buffers.get(&get_buffer_key!(data)).unwrap();
///
///     // do something with buffer...
/// }
/// ```
#[macro_export]
macro_rules! get_buffer_key {
    ($i:ident) => {
        ($i.as_slice() as *const [f32])
    };
}

/// A macro for declaring a thing that the GPU should do.
///
/// By declaring things that the GPU should do, this macro essentially serves
/// as the "accelerating" part of Emu. It assumes a GPU is in scope and
/// focuses on simply using that GPU to accelerate. Here's an example of usage.
///
/// ```
/// # extern crate em;
/// # use em::*;
/// #[gpu_use] // removing this will effectively switch to "no GPU"
/// fn main() {
///     let mut data = vec![0.1; 1000];
///
///     gpu_do!(load(data)); // load data to the GPU
///     // now that data is loaded, we should not re-allocate it (by changing
///     // its size) in between launches, reads that use the data
///     gpu_do!(launch()); // launch the next thing encountered by the compiler
///     // the next thing is a for loop so Emu compiles it into a "kernel" and
///     // launches the kernel on the GPU
///     for i in 0..1000 {
///         data[i] = data[i] * 10.0;
///     }
///     gpu_do!(read(data)); // read data back from GPU
/// }
/// ```
/// Concretely, there are 3 (only 3 at the moment) commands to the GPU that
/// can be declared.
/// 1. Loading to the GPU with `gpu_do!(load(data))`
/// 2. Reading from the GPU with `gpu_do!(read(data))`
/// 3. Launching on the GPU with `gpu_do!(launch())`
///
/// Note that data must be an identifier. The only hard requirement for data is
/// that it must have the 2 following methods.
/// - `fn as_slice(&self) -> &[f32]`
/// - `fn as_mut_slice(&mut self) -> &mut [f32]`
///
/// There is a soft requirement that the data should be representing a list of
/// `f32`s and indexing it with `data[i]` should return an `f32`. But this is
/// really just to ensure that when we lift code from CPU to GPU it is
/// functionally equivalent in a sane way. Also, note that no invocation of
/// `gpu_do!()` will ever expand to anything, unless the function it's being
/// used in is tagged with `#[gpu_use]`
///
/// There is also a requirement that once data is loaded, it should not be
/// re-allocated on the CPU in-between launches, reads that make use of it.
/// So basically just make sure you don't resize it.
///
/// And in case the example doesn't make
/// this clear, `gpu_do!(launch())` basically attempts to launch the following
/// expression/piece of code on the GPU. Now, you can't just put any code you
/// want there. There is a very, very small subset of Rust code that can
/// be launched. Anything outside of this subset will result in a compile-time
/// error that will explain to you what was outside of the subset.
#[macro_export]
macro_rules! gpu_do {
    (load($i:ident)) => {};
    (read($i:ident)) => {};
    (launch()) => {};
}
