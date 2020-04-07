//! `emu_core` is a library that serves as a compute-focused abstraction over
//! [WebGPU](https://github.com/gfx-rs/wgpu-rs). Despite its name, WebGPU
//! allows Emu to support most platforms (through Vulkan, Metal, DX) and
//! eventually even the web itself (through WebAssembly - API changes to
//! support this should be minimal).
//!
//! You can see [the crate](https://crates.io/emu_core) for how to add Emu to 
//! your project (`emu_core = "*"`) and [the examples](https://github.com/calebwin/emu/tree/master/emu_core/examples)
//! for how to use Emu. The following link to documentation for the building 
//! blocks of Emu.
//! - See [`Device`](device/struct.Device.html) and [`pool`](pool/index.html) for the lowest-level, core primitives abstracting directly over WebGPU
//! - See [`DeviceBox<T>`](device/struct.DeviceBox.html), [`AsDeviceBoxed`](boxed/trait.AsDeviceBoxed.html), [`IntoDeviceBoxed`](boxed/trait.IntoDeviceBoxed.html) for 
//! [boxing](https://en.wikipedia.org/wiki/Object_type_(object-oriented_programming)#Boxing) data on the GPU
//! - See [`compile`](compile/fn.compile.html) for compiling some source language to `SpirvOrFinished` and then finishing to `DeviceFnMut`
//! - See [`spawn`](spawn/struct.spawn.html) for spawning threads on GPU and launching compiled kernels (`DeviceFnMut`s)
//!
//! Note that `Device` and `pool` are the lowest-level building blocks for the
//! rest of Emu and as such, you could technically use either just `Device` and
//! `pool` or just the rest of Emu. In practice though, you will probably do 
//! both. You will use the rest of Emu for most of your application/library and
//! then drop down to low-level `Device`-and-`pool` usage in rare cases when
//! you want to work with the underlying WebGPU data (maybe for interop with 
//! graphics) structures or to have finer control over certain parameters.

#[macro_use]
extern crate lazy_static; // we use lazy_static for global cache of JITed programs and pool of devices

// the high-level compile-cache-spawn-launch functionality
pub mod cache; // includes the Cache trait for implementing disk/in-memory caches of JIT compiled programs
pub mod compile; // includes the Compile trait for implementing source language inputs to Emu (e.g. - XLA, Halide, GLSL, Swift SIL, Julia IR, etc.)
pub mod compile_impls;
pub mod spawn; // use for spawning threads and launching a DeviceFnMut
                // a set of traits and functions for working with DeviceBox's
pub mod boxed;
// a pool of devices to reduce some boilerplate, use for a CUDA-ish API where a global device pool is shared by all Emu users
pub mod pool;
// a set of types for errors in device usage
pub mod error;
// the lowest-level abstraction over wgpu-rs, use this for easy zero-cost interop with wgpu-rs data structures
pub mod device;

macro_rules! pub_use {
	($($module:ident),*) => ($(pub use crate::$module::*;)*)
}

pub mod prelude {
    #[macro_use]
    pub use crate::call;
    pub_use! {compile, compile_impls, cache, spawn, boxed, device, error, pool}
}
