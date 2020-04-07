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

use wgpu::read_spirv;

// TODO in the future, generalize this to other types, not just struct
/// A trait for structures that can exist in both Rust and GLSL
pub trait GlslStruct {
    /// Provides the GLSL structure definition code to define this structure in GLSL
    fn as_glsl() -> String;
}

/// The trait to implement when adding support for a new source language (HLSL, XLA, Swift SIL, etc.).
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
/// The wrapper adds a few important details including parameters and the name of the relevant entry point in the bytecode
#[derive(Hash)]
pub struct Spirv<P: BorrowMut<[u32]>> {
    pub params: DeviceFnMutParams,
    pub name: String,
    pub code: P,
}

/// A builder for constructing a [`Spirv`](struct.spirv.html)
pub struct SpirvBuilder<P: BorrowMut<[u32]>> {
    params_builder: ParamsBuilder,
    name: String,
    code: Option<P>,
}

impl<P: BorrowMut<[u32]>> SpirvBuilder<P> {
    /// Sets the name of the point in this chunk of SPIR-V where it should be entered
    ///
    /// If you are compiling from GLSL, for example, your entry point name might be "main" if you have a "void main" function.
    pub fn set_entry_point_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    /// Add a new constant parameter to the kernel in this SPIR-V
    pub fn add_param(mut self) -> Self {
        self.params_builder = self.params_builder.param(Mutability::Const);
        self
    }

    /// Add a new mutable parameter to the kernel in this SPIR-V
    pub fn add_param_mut(mut self) -> Self {
        self.params_builder = self.params_builder.param(Mutability::Mut);
        self
    }

    /// Set the actual code itself from an owned or borrowed slice of `u32`
    pub fn set_code_with_u32(mut self, code: P) -> Result<Self, std::io::Error> {
        self.code = Some(code);
        Ok(self)
    }

    /// Build the `Spirv`
    pub fn build(self) -> Spirv<P> {
        Spirv {
            params: self.params_builder.build(),
            name: self.name,
            code: self
                .code
                .expect("no SPIR-V code was provided in the process of using SpirvBuilder"),
        }
    }
}

impl SpirvBuilder<Vec<u32>> {
    /// Set the actual code itself from an owned `Vec<u32>``
    pub fn set_code_with_u8(mut self, code: impl Read + Seek) -> Result<Self, std::io::Error> {
        self.code = Some(read_spirv(code)?);
        Ok(self)
    }
}

/// Compiles the given source to `SpirvOrFinished`
///
/// The returned `SpirvOrFinished` is a finished `DeviceFnMut` if the source was in the cache or just the compiled SPIR-V if not.
/// You can then call `finish` on the result to finish the compiled SPIR-V to a `DeviceFnMut` in the case that source _wasn't_ in cache.
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
pub enum SpirvOrFinished<P: BorrowMut<[u32]>, C: Cache> {
    SpirvAndHash((Spirv<P>, u64, std::marker::PhantomData<C>)), // we need to pass in the hash of the source so that we can store the finished result in the cache
    Finished(Arc<DeviceFnMut>),
}

impl<P: BorrowMut<[u32]>, C: Cache> SpirvOrFinished<P, C> {
    pub fn get_code_mut(&mut self) -> Option<&mut [u32]> {
        match self {
            SpirvOrFinished::SpirvAndHash((spirv, _, _)) => Some(spirv.code.borrow_mut()),
            _ => None,
        }
    }

    pub fn get_name_mut(&mut self) -> Option<&mut String> {
        match self {
            SpirvOrFinished::SpirvAndHash((spirv, _, _)) => Some(&mut spirv.name),
            _ => None,
        }
    }

    pub fn get_params_mut(&mut self) -> Option<&mut DeviceFnMutParams> {
        match self {
            SpirvOrFinished::SpirvAndHash((spirv, _, _)) => Some(&mut spirv.params),
            _ => None,
        }
    }

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
