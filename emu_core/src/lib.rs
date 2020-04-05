// emu_core is a stack of abstraction layers that abstract over WebGPU

#[macro_use]
extern crate lazy_static;

// a bunch of utilities for compiling/launching programs (with a cache for JITed programs)
// pub mod r#fn;
pub mod cache;
pub mod compile;
pub mod compile_impls;
pub mod launch;
// a set of traits and functions for working with DeviceBox's
pub mod boxed;
// a pool of devices to reduce some boilerplate, use for a CUDA-ish API that feels high-level
pub mod pool;
// a type for errors in device usage
pub mod error;
// the lowest-level abstraction over wgpu-rs, use this for easy zero-cost interop with wgpu-rs
pub mod device;

macro_rules! pub_use {
    (module) => {};
}
macro_rules! pub_use {
	($($module:ident),*) => ($(pub use crate::$module::*;)*)
}

// TODO add a prelude module
pub mod prelude {
    #[macro_use]
    pub use crate::call;
    pub_use! {compile, compile_impls, cache, launch, boxed, device, error, pool}
}
