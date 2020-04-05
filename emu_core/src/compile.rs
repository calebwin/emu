use crate::cache::*;
use crate::device::*;
use crate::error::*;
use crate::pool::*;

use std::borrow::BorrowMut;
use std::collections::hash_map::{DefaultHasher, HashMap};
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Read, Seek};
use std::sync::{Arc, RwLock};

use wgpu::read_spirv;

// PODO in the future, generalize this to other types, not just struct
pub trait GlslStruct {
    fn as_glsl() -> String;
}

// implement this trait for any compiler of high-level source language to SPIR-V
// the compiler should be able to output a module with single entry point (i.e. - a single compute kernel)
//
// generic over its inpput
//
// any implementor must provide either owned slice or a mutable reference
// we don't allow reference because we want to ensure that _any_ outputted SPIR-V can be modified
// for example, we want to be able to define Compile for PF Graph IR and then let people do various linting, optimization, compression, etc. before finishing compilation to DeviceFnMut
pub trait CompileToSpirv<I: Hash, P: BorrowMut<[u32]>> {
    fn compile_to_spirv(src: I) -> Result<Spirv<P>, CompileError>;
}

// a wrapper for SPIR-V code
// the wrapper adds a few important details including parameters and the name of the entry point to use in the code
#[derive(Hash)]
pub struct Spirv<P: BorrowMut<[u32]>> {
    pub params: DeviceFnMutParams,
    pub name: String,
    pub code: P,
}

pub struct SpirvBuilder<P: BorrowMut<[u32]>> {
    params_builder: ParamsBuilder,
    name: String,
    code: Option<P>,
}

impl<P: BorrowMut<[u32]>> SpirvBuilder<P> {
    // set the name of the entry point in this blob of SPIR-V where it should be entered
    pub fn set_entry_point_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    pub fn add_param(mut self) -> Self {
        self.params_builder = self.params_builder.param(Mutability::Const);
        self
    }

    pub fn add_param_mut(mut self) -> Self {
        self.params_builder = self.params_builder.param(Mutability::Mut);
        self
    }

    pub fn set_code_with_u32(mut self, code: P) -> Result<Self, std::io::Error> {
        self.code = Some(code);
        Ok(self)
    }

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
    pub fn set_code_with_u8(mut self, code: impl Read + Seek) -> Result<Self, std::io::Error> {
        self.code = Some(read_spirv(code)?);
        Ok(self)
    }
}

pub fn compile<I: Hash, U: CompileToSpirv<I, P>, P, C: Cache>(
    src: I,
) -> Result<SpirvOrFinished<P, C>, CompileError>
where
    P: BorrowMut<[u32]>,
{
    let mut hasher = DefaultHasher::new();
    src.hash(&mut hasher);
    let hash = hasher.finish();

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
