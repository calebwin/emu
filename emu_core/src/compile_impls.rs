use crate::cache::*;
use crate::compile::*;
use crate::device::*;
use crate::error::*;
use crate::pool::*;

use std::borrow::BorrowMut;
use std::collections::hash_map::{DefaultHasher, HashMap};
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Read, Seek};
use std::sync::{Arc, RwLock};

//
// Spirv made using SpirvBuilder
//

impl<P: Hash + BorrowMut<[u32]>> CompileToSpirv<Spirv<P>, P> for Spirv<P> {
    fn compile_to_spirv(src: Spirv<P>) -> Result<Spirv<P>, CompileError> {
        Ok(src)
    }
}

// 
// Glsl
// 

pub struct GlslCompile;

#[derive(Hash)]
pub struct Glsl {
    name: String,
    params_builder: ParamsBuilder,
    code: String
}

impl Glsl {
    pub fn new() -> Self {
        Glsl {
            name: String::from("main"),
            params_builder: ParamsBuilder::new(),
            code: String::from("#version 450\nvoid main() {}")
        }
    }

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

    pub fn set_code_with_glsl(mut self, code: impl Into<String>) -> Self {
        self.code = code.into();
        self
    }
}

#[cfg(feature = "glsl-compile")]
impl CompileToSpirv<Glsl, Vec<u32>> for GlslCompile {
    fn compile_to_spirv(mut src: Glsl) -> Result<Spirv<Vec<u32>>, CompileError> {
        // (6) compile to SPIR-V
        let mut compiler = unsafe { shaderc::Compiler::new().unwrap() };
        let binary_result = unsafe {
            compiler
                .compile_into_spirv(
                    &src.code,
                    shaderc::ShaderKind::Compute,
                    "a compute shader",
                    &src.name,
                    None,
                )
                .unwrap()
        };

        // yes, copying the binary over into a vec is expensive
        // but it's necessary so that we can allow users to mutate binary later on
        // and the copying of the binary is dwarfed by many other operations of this library
        // also, we cache anyway
        Ok(Spirv {
            params: src.params_builder.build(),
            name: src.name,
            code: binary_result.as_binary().to_vec(),
        })
    }
}

//
// GlslKernel
//

#[derive(Hash)]
pub struct GlslKernel {
    code: String,
    params: Vec<String>,
    params_mutability: Vec<Mutability>,
    structs: Vec<String>,
    consts: Vec<(String, String)>,
    shared: Vec<String>,
    local_size: Vec<u32>,
    helper_code: String,
    kernel_code: String,
}

impl GlslKernel {
    pub fn new() -> Self {
        Self {
            code: String::from("#version 450\n"),
            params: vec![],
            params_mutability: vec![],
            structs: vec![],
            consts: vec![],
            shared: vec![],
            local_size: vec![],
            helper_code: String::new(),
            kernel_code: String::new(),
        }
    }

    pub fn spawn(mut self, num_threads: u32) -> Self {
        self.local_size.push(num_threads);
        self
    }

    pub fn with_struct<T: GlslStruct>(mut self) -> Self {
        self.structs.push(T::as_glsl());
        self
    }

    pub fn with_const(mut self, left_hand: impl Into<String>, right_hand: impl Into<String>) -> Self {
        self.consts.push((left_hand.into(), right_hand.into()));
        self
    }

    pub fn share(mut self, shared: impl Into<String>) -> Self {
        self.shared.push(shared.into());
        self
    }

    pub fn param(mut self, param: impl Into<String>) -> Self {
        self.params.push(param.into());
        self.params_mutability.push(Mutability::Const);
        self
    }

    pub fn param_mut(mut self, param: impl Into<String>) -> Self {
        self.params.push(param.into());
        self.params_mutability.push(Mutability::Mut);
        self
    }

    pub fn with_helper_code(mut self, code: impl Into<String>) -> Self {
        self.helper_code = code.into();
        self
    }

    pub fn with_kernel_code(mut self, code: impl Into<String>) -> Self {
        self.kernel_code = code.into();
        self
    }
}

pub struct GlslKernelCompile;

#[cfg(feature = "glsl-compile")]
impl CompileToSpirv<GlslKernel, Vec<u32>> for GlslKernelCompile {
    fn compile_to_spirv(mut src: GlslKernel) -> Result<Spirv<Vec<u32>>, CompileError> {
        let mut num_params = src.params.len();
        let mut kernel_name = String::from("main");

        // (1) local size
        if src.local_size.len() == 0 {
            src.local_size = vec![1];
        }
        src.code += "\nlayout(";
        if src.local_size.len() == 1 {
            src.code += "local_size_x = ";
            src.code += &src.local_size[0].to_string();
        }
        if src.local_size.len() == 2 {
            src.code += "local_size_x = ";
            src.code += &src.local_size[0].to_string();
            src.code += ", local_size_y = ";
            src.code += &src.local_size[1].to_string();
        }
        if src.local_size.len() == 3 {
            src.code += "local_size_x = ";
            src.code += &src.local_size[0].to_string();
            src.code += ", local_size_y = ";
            src.code += &src.local_size[1].to_string();
            src.code += ", local_size_z = ";
            src.code += &src.local_size[2].to_string();
        }
        if src.local_size.len() >= 4 {
            src.code += "local_size_x = ";
            src.code += &src.local_size.iter().product::<u32>().to_string();
        }
        src.code += ") in;\n";

        // (2) structs
        for struct_def in src.structs {
            src.code += &struct_def;
        }

        // (3) buffer for each parameter
        let mut params = ParamsBuilder::new();
        for (i, param) in src.params.iter().enumerate() {
            params = params.param(src.params_mutability[i]);
            src.code += "\nlayout(set = 0, binding = ";
            src.code += &i.to_string();
            src.code += ") buffer Buffer";
            src.code += &i.to_string();
            src.code += " {\n";
            src.code += param;
            src.code += ";\n};\n";
        }

        // (4) consts
        for (left_hand, right_hand) in src.consts {
            src.code += &left_hand;
            src.code += " = ";
            src.code += &right_hand;
            src.code += ";\n";
        }

        // (5) shared
        for shared in src.shared {
            src.code += "shared ";
            src.code += &shared;
            src.code += ";\n";
        }

        // (4) helper code
        src.code += &src.helper_code;

        // (5) kernel code
        src.code += "\nvoid main() {\n";
        src.code += &src.kernel_code;
        src.code += "}\n";

        // (6) compile to SPIR-V
        let mut compiler = unsafe { shaderc::Compiler::new().unwrap() };
        let binary_result = unsafe {
            compiler
                .compile_into_spirv(
                    &src.code,
                    shaderc::ShaderKind::Compute,
                    "a compute shader",
                    "main",
                    None,
                )
                .unwrap()
        };

        // yes, copying the binary over into a vec is expensive
        // but it's necessary so that we can allow users to mutate binary later on
        // and the copying of the binary is dwarfed by many other operations of this library
        // also, we cache anyway
        Ok(Spirv {
            params: params.build(),
            name: kernel_name,
            code: binary_result.as_binary().to_vec(),
        })
    }
}
