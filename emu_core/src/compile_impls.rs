//! A few implemented source languages that can be compiled to SPIR-V

use crate::compile::*;
use crate::device::*;
use crate::error::*;

use std::borrow::BorrowMut;

use std::hash::Hash;

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

/// A shaderc-based compiler for GLSL to SPIR-V
pub struct GlslCompile;

/// A "builder" for GLSL code
#[derive(Hash)]
pub struct Glsl {
    name: String,
    params_builder: ParamsBuilder,
    code: String,
}

impl Glsl {
    /// Creates a new GLSL builder
    pub fn new() -> Self {
        Glsl {
            name: String::from("main"),
            params_builder: ParamsBuilder::new(),
            code: String::from("#version 450\nvoid main() {}"),
        }
    }

    /// Sets the name of the point in this chunk of GLSL where it should be entered
    ///
    /// For example, your code's entry point name might be "main" if you have a "void main" function.
    pub fn set_entry_point_name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    /// Declares an additional parameter - that is constant - to the compute kernel in this GLSL
    pub fn add_param(mut self) -> Self {
        self.params_builder = self.params_builder.param(Mutability::Const);
        self
    }

    /// Declares an additional parameter - that is mutable - to the compute kernel in this GLSL
    pub fn add_param_mut(mut self) -> Self {
        self.params_builder = self.params_builder.param(Mutability::Mut);
        self
    }

    /// Use the given string as the GLSL source code
    pub fn set_code_with_glsl(mut self, code: impl Into<String>) -> Self {
        self.code = code.into();
        self
    }
}

#[cfg(feature = "glsl-compile")]
impl CompileToSpirv<Glsl, Vec<u32>> for GlslCompile {
    fn compile_to_spirv(src: Glsl) -> Result<Spirv<Vec<u32>>, CompileError> {
        // (6) compile to SPIR-V
        let mut compiler = shaderc::Compiler::new().unwrap();
        let binary_result = compiler
            .compile_into_spirv(
                &src.code,
                shaderc::ShaderKind::Compute,
                "a compute shader",
                &src.name,
                None,
            )
            .unwrap();

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

/// A convenience builder for GLSL compute kernels
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
    /// Initializes the builder
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

    /// Spawns "local threads"
    ///
    /// This essentially adds on a new dimension of threads to each thread block with the given size.
    /// The dimensions are "x", "y", and "z" in that order.
    pub fn spawn(mut self, num_threads: u32) -> Self {
        self.local_size.push(num_threads);
        self
    }

    /// Appends a GLSL struct definition for the type which this function is generic over
    pub fn with_struct<T: GlslStruct>(mut self) -> Self {
        self.structs.push(T::as_glsl());
        self
    }

    /// Appends a constant definition using the give left hand and right hand sides
    pub fn with_const(
        mut self,
        left_hand: impl Into<String>,
        right_hand: impl Into<String>,
    ) -> Self {
        self.consts.push((left_hand.into(), right_hand.into()));
        self
    }

    /// Creates a shared variable using the given code
    pub fn share(mut self, shared: impl Into<String>) -> Self {
        self.shared.push(shared.into());
        self
    }

    /// Generates code for a buffer through which constant data can be passed into the kernel
    pub fn param(mut self, param: impl Into<String>) -> Self {
        self.params.push(param.into());
        self.params_mutability.push(Mutability::Const);
        self
    }

    /// Generates code for a buffer through which mutable data can be passed into the kernel
    pub fn param_mut(mut self, param: impl Into<String>) -> Self {
        self.params.push(param.into());
        self.params_mutability.push(Mutability::Mut);
        self
    }

    /// Adds the given helper code
    ///
    /// This helper code may include additional type or function definitions.
    pub fn with_helper_code(mut self, code: impl Into<String>) -> Self {
        self.helper_code = code.into();
        self
    }

    /// Adds the body code for the kernel
    ///
    /// This body code is simply wrapped in a `void main` function.
    pub fn with_kernel_code(mut self, code: impl Into<String>) -> Self {
        self.kernel_code = code.into();
        self
    }
}

pub struct GlslKernelCompile;

#[cfg(feature = "glsl-compile")]
impl CompileToSpirv<GlslKernel, Vec<u32>> for GlslKernelCompile {
    fn compile_to_spirv(mut src: GlslKernel) -> Result<Spirv<Vec<u32>>, CompileError> {
        let kernel_name = String::from("main");

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
        let mut compiler = shaderc::Compiler::new().unwrap();
        let binary_result = compiler
            .compile_into_spirv(
                &src.code,
                shaderc::ShaderKind::Compute,
                "a compute shader",
                "main",
                None,
            )
            .unwrap();

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
