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
/// ```compile_fail
/// # extern crate em;
/// # use em::*;
///
/// #[gpu_use] // this inserts a "let gpu = Gpu { ... };" at the start of the main function
/// fn main() {
///     let data = vec![0.0; 1000];
///     let buffer: ocl::Buffer<f32> = *gpu.buffers.get(&((get_buffer_key!(data)))).unwrap();
///
///     // do something with buffer...
/// }
/// ```
#[macro_export]
macro_rules! get_buffer_key {
    ($i:ident) => {
        $i.as_slice() as *const [f32]
    };
}

/// A macro for specifying a thing that the GPU should do.
///
/// Note that this will not do anything if the function the macro is invoked in isn't tagged with `#[gpu_use]`
/// Here's an example of usage.
///
/// ```no_run
/// # extern crate em;
/// # use em::*;
///
/// #[gpu_use] // removing this will make the "gpu_do" invocations expand to nothing
/// fn main() {
///     let mut data = vec![0.1; 1000];
///
///     gpu_do!(load(data)); // load data to the GPU
///     gpu_do!(launch()); // launch the next thing encountered by the compiler
///     // the next thing is a for loop so Emu compiles it into a "kernel" and
///     // launches the kernel on the GPU
///     for i in 0..1000 {
///         data[i] = data[i] * 10.0;
///     }
///     gpu_do!(read(data)); // read data back from GPU
/// }
#[macro_export]
macro_rules! gpu_do {
    (load($i:ident)) => {};
    (read($i:ident)) => {};
    (launch()) => {};
}
