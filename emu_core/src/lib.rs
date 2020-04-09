#![doc(html_logo_url = "https://i.imgur.com/CZEkdK1.png")]

//! `emu_core` is a library that serves as a compute-focused abstraction over
//! [WebGPU](https://github.com/gfx-rs/wgpu-rs). Despite its name, WebGPU
//! allows Emu to support most platforms (through Vulkan, Metal, DX) and
//! eventually even the web itself (through WebAssembly - API changes to
//! support this should be minimal).
//!
//! You can see [the crate](https://crates.io/crates/emu_core) for how to add Emu to
//! your Rust project (`emu_core = "*"`) and [the examples](https://github.com/calebwin/emu/tree/master/emu_core/examples)
//! for how to use Emu. The following link to documentation of what are essentially the building
//! blocks of Emu.
//! - See [`Device`](device/struct.Device.html) and [`pool`](pool/index.html) for the lowest-level, core primitives abstracting directly over WebGPU
//! - See [`DeviceBox<T>`](device/struct.DeviceBox.html), [`AsDeviceBoxed`](boxed/trait.AsDeviceBoxed.html), [`IntoDeviceBoxed`](boxed/trait.IntoDeviceBoxed.html) for
//! [boxing](https://en.wikipedia.org/wiki/Object_type_(object-oriented_programming)#Boxing) data on the GPU
//! - See [`SpirvBuilder`](compile/struct.SpirvBuilder.html), [`Glsl`](compile_impls/struct.Glsl.html), [`GlslKernel`](compile_impls/struct.GlslKernel.html) for simple source
//! languages to use for writing compute kernels
//! - See [`compile`](compile/fn.compile.html) for compiling source language to `SpirvOrFinished` and then finishing to `DeviceFnMut`
//! - See [`spawn`](spawn/fn.spawn.html) for spawning threads on GPU and launching compiled kernels (`DeviceFnMut`s)
//! - See [`pool`](pool/index.html)'s [`pool`](pool/fn.pool.html)/[`select`](pool/fn.select.html)/[`take`](pool/fn.take.html) for
//! managing the global pool of devices
//! - See [`assert_device_pool_initialized`](pool/fn.assert_device_pool_initialized.html)
//!
//! Note that `Device` and `pool` are the lowest-level building blocks for the
//! rest of Emu and as such, you could technically use either just `Device` and
//! `pool` or just the rest of Emu. In practice though, you will probably do
//! both. You will use the rest of Emu for most of your application/library and
//! then drop down to low-level `Device`-and-`pool` usage in rare cases when
//! you want to work with the underlying WebGPU data (maybe to mix in graphics with your
//! compute) structures or to have finer control over certain parameters.
//!
//! And about features - there is 1 feature that by default is switched off - `glsl-compile`.
//! You should [enable this feature](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#choosing-features) if you would like to use
//! [`Glsl`](compile_impls/struct.Glsl.html) or [`GlslKernel`](compile_impls/struct.GlslKernel.html). This feature has one important dependency -
//! [`shaderc`](https://docs.rs/shaderc/0.6.2/shaderc/index.html). In the future, when a Rust-based GLSL-to-SPIR-V compiler is finished (there is work going towards this),
//! there will be a simpler pure-Rust dependency but until then, you should follow [steps here](https://docs.rs/shaderc/0.6.2/shaderc/index.html) to ensure the platforms you
//! target will have `shaderc`.
//! Of course, if you really don't want to use `shaderc`, you could always [compile your code to SPIR-V at compile time](https://crates.io/crates/glsl-to-spirv-macros) and
//! then use SPIR-V as input to Emu.
//!
//! Also, some basic guides that will likely be helpful in using Emu are the following.
//! - [How to use CUDA](https://www.nvidia.com/docs/IO/116711/sc11-cuda-c-basics.pdf) - This explains the idea of launching kernels on a 3-dimensional space of threads, which Emu
//! and CUDA share
//! - [How to write GLSL compute shaders](https://www.khronos.org/opengl/wiki/Compute_Shader) - This explains some of the stuff that is specific to SPIR-V, which Emu uses as input

#[macro_use]
extern crate lazy_static; // we use lazy_static for global device pool and global kernel cache

// the high-level compile-cache-spawn-launch functionality
pub mod cache; // includes the Cache trait for implementing disk/in-memory caches of JIT compiled programs
pub mod compile; // includes the Compile trait for implementing source language inputs to Emu (e.g. - XLA, Halide, GLSL, Swift SIL, Julia IR, etc.)
pub mod compile_impls;
pub mod spawn; // use for spawning threads and launching a DeviceFnMut
               // a set of traits and functions for working with DeviceBox's
pub mod boxed;
// a pool of devices to reduce some boilerplate, use for a CUDA-esque API where a global device pool is shared by all Emu users
pub mod pool;
// a set of types for errors in device usage
pub mod error;
// the lowest-level abstraction over wgpu-rs, use this for easy zero-cost interop with wgpu-rs data structures
pub mod device;

macro_rules! pub_use {
	($($module:ident),*) => ($(pub use crate::$module::*;)*)
}

pub mod prelude {
    //! The module to import to import everything else
    pub use crate::call;
    pub_use! {compile, compile_impls, cache, spawn, boxed, device, error, pool}
}
