//! The whole source-to-`DeviceFnMut` compilation pipeline

use crate::cache::*;
use crate::device::*;
use crate::error::*;
use crate::pool::*;

use std::borrow::BorrowMut;
use std::collections::hash_map::DefaultHasher;

use std::hash::{Hash, Hasher};
use std::io::{Read, Seek};
use std::sync::Arc;

// TODO in the future, generalize this to other types, not just struct
/// A trait for structures that can exist in both Rust and GLSL
pub trait GlslStruct {
    /// Provides the GLSL structure definition code to define this structure in GLSL
    fn as_glsl() -> String;
}

/// The trait to implement when adding support for a new source language (e.g. - HLSL, XLA, Swift SIL, etc.).
///
/// This trait is generic over the input language (which must be hash-able so we can do caching) and the target bytecode (which can be a `Vec<u32>` or `&mut [u32]` for example).
/// We want the target bytecode to be mutable for a reason. We want people to be able to mutate the bytecode later on to
/// do - for example - various linting, optimization, compression, etc. before finishing compilation to DeviceFnMut.
///
/// Also, the target bytecode should be a GLSL compute shader with a single entry point of interest.
pub trait CompileToSpirv<I: Hash, P: BorrowMut<[u32]>> {
    /// Compiles the source language into SPIR-V
    fn compile_to_spirv(src: I) -> Result<Spirv<P>, CompileError>;
}

/// A wrapper for SPIR-V bytecode
///
/// The wrapper adds a few important details including parameters and the name of the relevant entry point in the bytecode.
/// You can construct a `Spirv` using a [`SpirvBuilder`](struct.SpirvBuilder.html).
#[derive(Hash)]
pub struct Spirv<P: BorrowMut<[u32]>> {
    pub params: DeviceFnMutParams,
    pub name: String,
    pub code: P,
}

/// A builder for constructing a [`Spirv`](struct.Spirv.html)
///
/// You can use it in the case where you are starting from either `u8` bytes or 4-byte `u32` words.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*, std::io::Cursor};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let mut device = &mut futures::executor::block_on(Device::all())[0];
/// # let data = vec![0.0; 2048];
/// # let mut data_on_gpu: DeviceBox<[f32]> = device.create_from_mut(data.as_slice());
/// let kernel: Vec<u8> = vec![
///     // Magic number.           Version number: 1.0.
///     0x03, 0x02, 0x23, 0x07,    0x00, 0x00, 0x01, 0x00,
///     // Generator number: 0.    Bound: 0.
///     0x00, 0x00, 0x00, 0x00,    0x00, 0x00, 0x00, 0x00,
///     // Reserved word: 0.
///     0x00, 0x00, 0x00, 0x00,
///     // OpMemoryModel.          Logical.
///     0x0e, 0x00, 0x03, 0x00,    0x00, 0x00, 0x00, 0x00,
///     // GLSL450.
///     0x01, 0x00, 0x00, 0x00];
///
/// // if we actually compile to a `DeviceFnMut` later on with `finish`, it will panic
/// // at runtime because our SPIR-V doesn't actually include a main function
//  // nor does it have a mutable and a constant storage buffer
/// let spirv: Spirv<Vec<u32>> = SpirvBuilder::new()
///     .set_entry_point_name("main")
///     .add_param_mut::<[f32]>()
///     .add_param::<[f32]>()
///     .set_code_with_u8(Cursor::new(kernel))?
///     .build();
/// # Ok(())
/// # }
/// ```
pub struct SpirvBuilder<P: BorrowMut<[u32]>> {
    params_builder: ParamsBuilder,
    name: String,
    code: Option<P>,
}

impl<P: BorrowMut<[u32]>> SpirvBuilder<P> {
    /// Creates a new builder
    pub fn new() -> Self {
        Self {
            params_builder: ParamsBuilder::new(),
            name: String::from("main"),
            code: None,
        }
    }

    /// Sets the name of the point in this chunk of SPIR-V where it should be entered
    ///
    /// If you are compiling from GLSL, for example, your entry point name might be "main" if you have a "void main" function.
    pub fn set_entry_point_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Appends a new constant parameter to the kernel in this SPIR-V
    pub fn add_param<T: ?Sized>(mut self) -> Self {
        self.params_builder = self.params_builder.param::<T>(Mutability::Const);
        self
    }

    /// Appends a new mutable parameter to the kernel in this SPIR-V
    pub fn add_param_mut<T: ?Sized>(mut self) -> Self {
        self.params_builder = self.params_builder.param::<T>(Mutability::Mut);
        self
    }

    /// Set the actual code itself using an owned or borrowed slice of `u32`
    pub fn set_code_with_u32(mut self, code: P) -> Result<Self, std::io::Error> {
        self.code = Some(code);
        Ok(self)
    }

    /// Finish building
    pub fn build(self) -> Spirv<P> {
        Spirv {
            params: self.params_builder.build(),
            name: self.name,
            code: self
                .code
                .expect("no SPIR-V code was given to this SpirvBuilder with either `set_code_with_u8` or `set_code_with_u32`"),
        }
    }
}

impl SpirvBuilder<Vec<u32>> {
    /// Set the actual code itself using an owned `Vec<u32>``
    pub fn set_code_with_u8(mut self, code: impl Read + Seek) -> Result<Self, std::io::Error> {
        self.code = Some(gfx_auxil::read_spirv(code)?);
        Ok(self)
    }
}

/// Compiles the given source to `SpirvOrFinished`
///
/// There are 4 things this function is generic over.
/// 1. The source language which must be `Hash`able for caching
/// 2. The compiler implementing [`CompileToSpirv`](trait.CompileToSpirv.html)
/// 3. The target bytecode, a mutable borrow of a `u32` slice
/// 4. The cache, implementing [`Cache`](../cache/trait.Cache.html)
///
/// The returned [`SpirvOrFinished`](enum.SpirvOrFinished.html) is a finished `DeviceFnMut` if the source was in the cache or just the compiled SPIR-V if not.
/// You can then call `finish` on the result to finish the compiled SPIR-V to a `DeviceFnMut` in the case that source _wasn't_ in cache.
///
/// Here's how you might use it.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*, std::io::Cursor};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let mut device = &mut futures::executor::block_on(Device::all())[0];
/// # let data = vec![0.0; 2048];
/// # let mut data_on_gpu: DeviceBox<[f32]> = device.create_from(data.as_slice());
/// let kernel: Vec<u32> = convert_to_spirv(Cursor::new(vec![
///     // Magic number.           Version number: 1.0.
///     0x03, 0x02, 0x23, 0x07,    0x00, 0x00, 0x01, 0x00,
///     // Generator number: 0.    Bound: 0.
///     0x00, 0x00, 0x00, 0x00,    0x00, 0x00, 0x00, 0x00,
///     // Reserved word: 0.
///     0x00, 0x00, 0x00, 0x00,
///     // OpMemoryModel.          Logical.
///     0x0e, 0x00, 0x03, 0x00,    0x00, 0x00, 0x00, 0x00,
///     // GLSL450.
///     0x01, 0x00, 0x00, 0x00]))?;
///
/// // later on, if we finish by compiling to a `DeviceFnMut`, it will panic
/// // at runtime just because our SPIR-V doesn't actually include a main function
//  // nor does it have a mutable and a constant storage buffer
/// let spirv: Spirv<Vec<u32>> = SpirvBuilder::new()
///     .set_entry_point_name("main")
///     .add_param_mut::<[f32]>()
///     .add_param::<[f32]>()
///     .set_code_with_u32(kernel)?
///     .build();
///
/// // in this case, `compile` doesn't do much other than caching
/// // but where `Spirv` is replaced with `Glsl` or `GlslKernel`,
/// // actual compilation takes place here
/// let spirv_or_finished = compile::<Spirv<_>, SpirvCompile, Vec<u32>, GlobalCache>(spirv)?;
/// // now at this point you can call `finish` to turn `spirv_or_finished` into
/// // a finished `DeviceFnMut`
/// # Ok(())
/// # }
/// ```
pub fn compile<I: Hash, U: CompileToSpirv<I, P>, P, C: Cache>(
    src: I,
) -> Result<SpirvOrFinished<P, C>, CompileError>
where
    P: BorrowMut<[u32]>,
{
    // get the hash of the source
    let mut hasher = DefaultHasher::new();
    src.hash(&mut hasher);
    let hash = hasher.finish();

    // check if source is in cache
    // if not, compile to SPIR-V before returning
    if C::contains(hash) {
        Ok(SpirvOrFinished::Finished(C::get(hash)))
    } else {
        let spirv = U::compile_to_spirv(src)?;
        Ok(SpirvOrFinished::SpirvAndHash((
            spirv,
            hash,
            std::marker::PhantomData,
        )))
    }
}

/// Either a finished `DeviceFnMut` or compiled SPIR-V
///
/// You can either call `finish` on this to get your final compiled `DeviceFnMut` or you can inspect/mutate the inner SPIR-V before finishing.
/// ```should_panic
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*, std::io::Cursor};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # let mut device = &mut futures::executor::block_on(Device::all())[0];
/// # let data = vec![0.0; 2048];
/// # let mut data_on_gpu: DeviceBox<[f32]> = device.create_from(data.as_slice());
/// let kernel: Vec<u8> = vec![
///     // Magic number.           Version number: 1.0.
///     0x03, 0x02, 0x23, 0x07,    0x00, 0x00, 0x01, 0x00,
///     // Generator number: 0.    Bound: 0.
///     0x00, 0x00, 0x00, 0x00,    0x00, 0x00, 0x00, 0x00,
///     // Reserved word: 0.
///     0x00, 0x00, 0x00, 0x00,
///     // OpMemoryModel.          Logical.
///     0x0e, 0x00, 0x03, 0x00,    0x00, 0x00, 0x00, 0x00,
///     // GLSL450.
///     0x01, 0x00, 0x00, 0x00];
/// let spirv: Spirv<Vec<u32>> = SpirvBuilder::new()
///     .set_entry_point_name("main")
///     .add_param_mut::<[f32]>()
///     .add_param::<[f32]>()
///     .set_code_with_u8(Cursor::new(kernel))?
///     .build();
///
/// // in this case, `compile` doesn't do much other than caching
/// // but where `Spirv` is replaced with `Glsl` or `GlslKernel`,
/// // actual compilation takes place here
/// let mut spirv_or_finished = compile::<Spirv<_>, SpirvCompile, Vec<u32>, GlobalCache>(spirv)?;
/// // print out the SPIR-V and finish
/// if let Some(code) = spirv_or_finished.get_code_mut() {
///     println!("{:?}", code);
/// }
///
/// // the returned result is an ARC of a `DeviceFnMut`
/// // ARC just stands for "automatic reference counting"
/// // it means that there is runtime reference counting to
/// // ensure the compiled kernel can be safely used by multiple threads simultaneously
/// let finished_device_fn_mut = spirv_or_finished.finish()?;
/// // the above `finish` completes the compilation and in this case it
/// // will panic because our SPIR-V doesn't actually include a main function
//  // nor does it have a mutable and a constant storage buffer
/// # Ok(())
/// # }
/// ```
pub enum SpirvOrFinished<P: BorrowMut<[u32]>, C: Cache> {
    SpirvAndHash((Spirv<P>, u64, std::marker::PhantomData<C>)), // we need to pass in the hash of the source so that we can store the finished result in the cache
    Finished(Arc<DeviceFnMut>),
}

impl<P: BorrowMut<[u32]>, C: Cache> SpirvOrFinished<P, C> {
    /// Get a mutable reference to the code stored here
    ///
    /// This is useful when inspecting or linting the bytecode somehow before finishing compilation.
    /// If you decide to pass the SPIR-V through some sort of bytecode optimizer or such, this is the place
    pub fn get_code_mut(&mut self) -> Option<&mut [u32]> {
        match self {
            SpirvOrFinished::SpirvAndHash((spirv, _, _)) => Some(spirv.code.borrow_mut()),
            _ => None,
        }
    }

    /// Mutate the entry point name
    pub fn get_name_mut(&mut self) -> Option<&mut String> {
        match self {
            SpirvOrFinished::SpirvAndHash((spirv, _, _)) => Some(&mut spirv.name),
            _ => None,
        }
    }

    /// Mutate the parameters
    pub fn get_params_mut(&mut self) -> Option<&mut DeviceFnMutParams> {
        match self {
            SpirvOrFinished::SpirvAndHash((spirv, _, _)) => Some(&mut spirv.params),
            _ => None,
        }
    }

    /// Finish the compilation and return a `DeviceFnMut`
    pub fn finish(&self) -> Result<Arc<DeviceFnMut>, CompileOrNoDeviceError> {
        match self {
            SpirvOrFinished::SpirvAndHash((spirv, src_hash, _)) => {
                // compile SPIR-V to machine code (DeviceFnMut)
                // then put it in the cache and return it
                C::insert(
                    *src_hash,
                    Arc::new(
                        take()
                            .map_err(|_| CompileOrNoDeviceError::NoDevice)?
                            .lock()
                            .unwrap()
                            .compile::<_, &[u32]>(
                                spirv.params.clone(),
                                spirv.name.clone(),
                                spirv.code.borrow(),
                            )
                            .map_err(|_| CompileOrNoDeviceError::Compile)?,
                    ),
                );
                Ok(C::get(*src_hash))
            }
            SpirvOrFinished::Finished(device_fn_mut) => Ok(device_fn_mut.clone()),
        }
    }
}
