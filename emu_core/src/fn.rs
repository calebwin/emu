use crate::device::*;
use crate::error::*;
use crate::pool::*;

use std::collections::hash_map::{DefaultHasher, HashMap};
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Read, Seek};
use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;

#[cfg(feature = "spirv-compile")]
extern crate spirv_headers as spirv;
#[cfg(feature = "spirv-compile")]
extern crate spirv_reflect;
#[cfg(feature = "spirv-compile")]
use spirv::{AddressingModel, ExecutionModel, MemoryModel};
#[cfg(feature = "spirv-compile")]
use spirv_reflect::types::descriptor::ReflectDescriptorType;
#[cfg(feature = "spirv-compile")]
use spirv_reflect::*;

#[cfg(feature = "glsl-compile")]
extern crate spirv_headers as spirv;
#[cfg(feature = "glsl-compile")]
extern crate spirv_reflect;
#[cfg(feature = "glsl-compile")]
use shaderc::{};
#[cfg(feature = "glsl-compile")]
use spirv::{AddressingModel, ExecutionModel, MemoryModel};
#[cfg(feature = "glsl-compile")]
use spirv_reflect::types::descriptor::ReflectDescriptorType;
#[cfg(feature = "glsl-compile")]
use spirv_reflect::*;

//
// COMPILE
//

// implement this trait for any compiler of high-level source language to SPIR-V
// the compiler should be able to output a module with single entry point (i.e. - a single compute kernel)
//
// generic over its inpput
pub trait Compile<I: Hash> {
    type Output: Read + Seek;
    fn compile(src: I) -> (DeviceFnMutParams, String, Self::Output);
}

// this doesn't support **all** SPIR-V
// rather, it only supports SPIR-V programs that contain a single compute (GL) kernel and only a single descriptor set which contains only storage buffers
//
// so if you're using SpirvCompile to compile from a high-level program representation down to SPIR-V or even to hand-write SPIR-V, there are 3 things you should ensure
// 1. your SPIR-V program only has a single entrypoint - that of a GL compute kernel
// 2. you only use a single description set
// 3. all the bindings inside that description set are storage buffers (no push constants, just use buffers)
//
// by having these restrictions, we can use DeviceFnMut as a compact way of expressing a compute kernel
#[cfg(feature = "spirv-compile")]
pub struct SpirvCompile;

#[cfg(feature = "spirv-compile")]
impl Compile<Vec<u8>> for SpirvCompile {
    type Output = Cursor<Vec<u8>>;

    fn compile(mut src: Vec<u8>) -> (DeviceFnMutParams, String, Self::Output) {
        let mut num_params = 0;
        let mut kernel_name = String::new();

        // // attempt using rspirv
        // let mut loader = Loader::new();
        // {
        //     let p = Parser::new(&src_into, &mut loader);
        //     p.parse().unwrap();
        // }
        // let module = loader.module();

        // attempt using spirv-reflect-rs
        let module = ShaderModule::load_u8_data(&src).unwrap();
        let entry_point = module
            .enumerate_entry_points()
            .unwrap()
            .iter()
            .find(|entry_point| entry_point.spirv_execution_model == ExecutionModel::GLCompute)
            .expect("the given SPIR-V program doesn't contain any compute kernels")
            .clone();
        kernel_name = entry_point.name.clone();
        num_params = entry_point.descriptor_sets[0]
            .bindings
            .iter()
            .filter(|binding| binding.descriptor_type == ReflectDescriptorType::StorageBuffer)
            .count();

        (
            DeviceFnMutParams::new(num_params),
            kernel_name,
            Cursor::new(src),
        )
    }
}

// once again, we have requirements for what kind of GLSL works
// all the requirements from above apply as well as a fourth one
// 4. the name of the entry point must be "main"
#[cfg(feature = "glsl-compile")]
pub struct GlslCompile;

#[cfg(feature = "glsl-compile")]
impl Compile<String> for GlslCompile {
    type Output = Cursor<Vec<u8>>;

    fn compile(mut src: String) -> (DeviceFnMutParams, String, Self::Output) {
        let mut num_params = 0;
        let mut kernel_name = String::new();

        // (1) compile to SPIR-V
        let mut compiler = shaderc::Compiler::new().unwrap();
        let binary_result = compiler
            .compile_into_spirv(
                &src,
                shaderc::ShaderKind::Compute,
                "a compute shader",
                "main",
                None,
            )
            .unwrap();
        let spirv_bytes = Vec::from(binary_result.as_binary_u8());

        // (2) extract info

        let module = ShaderModule::load_u8_data(&spirv_bytes).unwrap();
        let entry_point = module
            .enumerate_entry_points()
            .unwrap()
            .iter()
            .find(|entry_point| entry_point.spirv_execution_model == ExecutionModel::GLCompute)
            .expect("the given SPIR-V program doesn't contain any compute kernels")
            .clone();
        kernel_name = entry_point.name.clone();
        num_params = entry_point.descriptor_sets[0]
            .bindings
            .iter()
            .filter(|binding| binding.descriptor_type == ReflectDescriptorType::StorageBuffer)
            .count();

        (
            DeviceFnMutParams::new(num_params),
            kernel_name,
            Cursor::new(spirv_bytes),
        )
    }
}

//
// CACHE
//

// in the future, we may not need to use a cache because caching is done automatically by wgpu

pub trait Cache {
    // key is derived from the source language
    // source language what is compiled to SPIR-V and then to machine code (stored in a DeviceFnMut)
    fn contains(key: u64) -> bool;
    fn get(key: u64) -> Arc<DeviceFnMut>;
    fn insert(key: u64, device_fn_mut: Arc<DeviceFnMut>);
}

lazy_static! {
    // RwLock and Arc are expensive, yes, but it's probably worth it since the performance penalty is dwarfed by compile time
    static ref GLOBAL_KERNEL_CACHE: RwLock<HashMap<u64, Arc<DeviceFnMut>>> = RwLock::new(HashMap::new());
    static ref GLOBAL_KERNEL_CACHE_LRU: RwLock<VecDeque<u64>> = RwLock::new(VecDeque::new()); // this "lru list" keeps track of which keys are most recently used
    static ref GLOBAL_KERNEL_CACHE_CAPACITY: RwLock<usize> = RwLock::new(0);
}

fn maybe_initialize_global_kernel_cache() {
    if *GLOBAL_KERNEL_CACHE_CAPACITY.read().unwrap() == 0 {
        *GLOBAL_KERNEL_CACHE_CAPACITY.write().unwrap() = 32;
    }
}

pub struct GlobalCache;

impl GlobalCache {
    pub fn reserve(additional: usize) {
        *GLOBAL_KERNEL_CACHE_CAPACITY.write().unwrap() += additional;
    }
}

impl Cache for GlobalCache {
    fn contains(key: u64) -> bool {
        maybe_initialize_global_kernel_cache();
        GLOBAL_KERNEL_CACHE.read().unwrap().contains_key(&key)
    }

    fn get(key: u64) -> Arc<DeviceFnMut> {
        maybe_initialize_global_kernel_cache();

        // move key to front of lru list
        let key_location_in_lru = GLOBAL_KERNEL_CACHE_LRU
            .read()
            .unwrap()
            .iter()
            .position(|&x| x == key)
            .unwrap();
        GLOBAL_KERNEL_CACHE_LRU
            .write()
            .unwrap()
            .swap(0, key_location_in_lru);

        // return DeviceFnMut with key from cache
        GLOBAL_KERNEL_CACHE
            .read()
            .unwrap()
            .get(&key)
            .map(|v| Arc::clone(v))
            .unwrap()
    }

    fn insert(key: u64, device_fn_mut: Arc<DeviceFnMut>) {
        maybe_initialize_global_kernel_cache();

        // check if our cache is out of space
        if GLOBAL_KERNEL_CACHE.read().unwrap().len()
            == *GLOBAL_KERNEL_CACHE_CAPACITY.read().unwrap()
        {
            // remove the least recently used
            let lru_location_in_cache = (*GLOBAL_KERNEL_CACHE_LRU.read().unwrap())
                .back()
                .unwrap()
                .clone();
            GLOBAL_KERNEL_CACHE
                .write()
                .unwrap()
                .remove(&lru_location_in_cache);
            // we're out of space so we need to remove the least recently used and insert this as most recently used
            GLOBAL_KERNEL_CACHE_LRU.write().unwrap().pop_back();
            GLOBAL_KERNEL_CACHE_LRU.write().unwrap().push_front(key);
        } else {
            // if not we just add this newly inserted key into the lru list
            GLOBAL_KERNEL_CACHE_LRU.write().unwrap().push_front(key);
        }

        // finally, insert into cache
        GLOBAL_KERNEL_CACHE
            .write()
            .unwrap()
            .insert(key, device_fn_mut);
    }
}

// TODO add a module for easily caching kernels
// TODO instead of Compile and compile, create module for dynamically defining kernels (or maybe even computation with kernels cached)

// Var, Const, Type (derived from structs), Assign, ForLoop, WhileLoop
// goal: a low-level thin abstraction over GLSL syntax using Rusty types

// this module only provides an API for simple JIT compilation from a source language
// in fact, it doesn't even do the JIT itself (relies on a trait implemented elsewhere)
// and stuff like providing macros that do AOT compilation of structs, functions won't be in this module
// just simple interface for JIT compilation

// this function returns a scary looking Arc (automatic reference counting pointer)
// but you can easily convert it to a reference with &
// we use Arc because DeviceFnMut's are loaded from global caches that can be used from various different threads
pub unsafe fn compile<I: Hash, U: Compile<I, Output = O>, O, C: Cache>(
    src: I,
) -> Result<Arc<DeviceFnMut>, CompileOrNoDeviceError>
where
    O: Read + Seek,
{
    let src_into = src; //.into();

    // hash to check in cache
    let mut hasher = DefaultHasher::new();
    src_into.hash(&mut hasher);
    let hash = hasher.finish();

    if C::contains(hash) {
        Ok(C::get(hash))
    } else {
        let compiled = U::compile(src_into);
        C::insert(
            hash,
            Arc::new(
                take()
                    .map_err(|_| CompileOrNoDeviceError::NoDevice)?
                    .lock()
                    .unwrap()
                    .compile::<String, _>(compiled.0, compiled.2, compiled.1)
                    .map_err(|_| CompileOrNoDeviceError::Compile)?,
            ),
        );
        Ok(C::get(hash))
    }
}

// TODO add function to compile SPIR-V to DeviceFnMut
// TODO add ability to use RAM-based cache
// TODO add ability to use disk-based cache

pub fn spawn(num_threads: u32) -> Spawner {
    Spawner {
        work_space_dim: vec![num_threads],
    }
}

pub struct Spawner {
    work_space_dim: Vec<u32>,
}

impl Spawner {
    pub fn spawn(mut self, num_threads: u32) -> Self {
        self.work_space_dim.push(num_threads);
        self
    }

    fn get_work_space_dim(&self) -> Result<(u32, u32, u32), LaunchError> {
        match self.work_space_dim.len() {
            0 => Ok((0, 0, 0)),
            1 => Ok((self.work_space_dim[0], 1, 1)),
            2 => Ok((self.work_space_dim[0], self.work_space_dim[1], 1)),
            3 => Ok((
                self.work_space_dim[0],
                self.work_space_dim[1],
                self.work_space_dim[2],
            )),
            _ => Err(LaunchError::TooManyThreads),
        }
    }

    pub unsafe fn launch<'a>(
        &self,
        device_fn_mut_with_args: (Arc<DeviceFnMut>, DeviceFnMutArgs<'a>),
    ) -> Result<(), LaunchError> {
        take()
            .map_err(|_| LaunchError::NoDevice)?
            .lock()
            .unwrap()
            .call(
                &device_fn_mut_with_args.0,
                self.get_work_space_dim()?,
                device_fn_mut_with_args.1,
            )
    }
}

#[macro_export]
macro_rules! call {
	($fn_mut:expr $( ,$fn_mut_arg:expr )*) => (
		{
            (
            	$fn_mut,
            	ArgBuilder::new()$(
                	.arg($fn_mut_arg)
            	)*.build()
            )
        }
	)
}
