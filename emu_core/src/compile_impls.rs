//! A few implemented source languages that can be compiled to SPIR-V

use crate::compile::*;
use crate::device::*;
use crate::error::*;

use std::borrow::BorrowMut;

use std::hash::Hash;

//
// Spirv made using SpirvBuilder
//

/// A SPIR-V-to-SPIR-V compiler!
///
/// Yeah, this doesn't really do anything. But it _does_ implement `CompileToSpirv` so it _is_ what you should use
/// as the type parameter for [`compile`](../compile/fn.compile.html) if you are compiling down from SPIR-V.
pub struct SpirvCompile;

impl<P: Hash + BorrowMut<[u32]>> CompileToSpirv<Spirv<P>, P> for SpirvCompile {
    fn compile_to_spirv(src: Spirv<P>) -> Result<Spirv<P>, CompileError> {
        Ok(src)
    }
}

//
// Glsl
//

/// A wrapper of GLSL code with methods to help progressively wrap your GLSL
///
/// This wrapper includes some extra information that is important, such as the name of the entry point of the GLSL chunk (e.g. - "main"), the mutability of each parameter buffer,
/// and the GLSL code itself.
///
/// You can construct a GLSL kernel and compile it with `GlslCompile` as follows.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # futures::executor::block_on(assert_device_pool_initialized());
/// # let data = vec![1.0; 2048];
/// # let mut data_on_gpu: DeviceBox<[f32]> = data.as_device_boxed_mut()?;
///
/// let kernel: Glsl = Glsl::new()
///     .set_entry_point_name("main")
///     .add_param_mut::<[f32]>()
///     .add_param::<f32>()
///     .set_code_with_glsl(r#"
/// #version 450
/// layout(local_size_x = 1) in;
///
/// layout(set = 0, binding = 0) buffer Data {
///     float[] data;
/// };
///
/// // if you are passing in multiple arguments of primitive types,
/// // you may want to store each argument in a field of a structure and
/// // then pass that structure into a buffer in a GLSL kernel
/// layout(set = 0, binding = 1) buffer Scalar {
///     float scalar;
/// };
///
/// void main() {
///     uint index = gl_GlobalInvocationID.x;
///     data[index] = data[index] * scalar;
/// }
///     "#);
/// let spirv_or_finished = compile::<Glsl, GlslCompile, _, GlobalCache>(kernel)?;
/// // now at this point you can call `.finish` to turn `spirv_or_finished` into
/// // a finished `DeviceFnMut`
/// # let finished = spirv_or_finished.finish()?;
/// # unsafe { spawn(2048).launch(call!(finished, &mut data_on_gpu, &DeviceBox::new(10.0f32)?))?; }
/// # assert_eq!(futures::executor::block_on(data_on_gpu.get())?, vec![10.0; 2048].into_boxed_slice());
/// # Ok(())
/// # }
/// ```
#[derive(Hash)]
#[cfg(feature = "glsl-compile")]
pub struct Glsl {
    name: String,
    params_builder: ParamsBuilder,
    code: String,
}

#[cfg(feature = "glsl-compile")]
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
    pub fn set_entry_point_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Declares an additional parameter - that is constant - to the compute kernel in this GLSL
    pub fn add_param<T: ?Sized>(mut self) -> Self {
        self.params_builder = self.params_builder.param::<T>(Mutability::Const);
        self
    }

    /// Declares an additional parameter - that is mutable - to the compute kernel in this GLSL
    pub fn add_param_mut<T: ?Sized>(mut self) -> Self {
        self.params_builder = self.params_builder.param::<T>(Mutability::Mut);
        self
    }

    /// Use the given string as the GLSL source code
    pub fn set_code_with_glsl(mut self, code: impl Into<String>) -> Self {
        self.code = code.into();
        self
    }
}

/// A `shaderc`-based compiler for [`Glsl`](struct.Glsl.html) to SPIR-V
#[cfg(feature = "glsl-compile")]
pub struct GlslCompile;

#[cfg(feature = "glsl-compile")]
impl CompileToSpirv<Glsl, Vec<u32>> for GlslCompile {
    fn compile_to_spirv(src: Glsl) -> Result<Spirv<Vec<u32>>, CompileError> {
        // (6) compile to SPIR-V
        let mut compiler = shaderc::Compiler::new().unwrap();
        let binary_result = compiler
            .compile_into_spirv(
                &src.code,
                shaderc::ShaderKind::Compute,
                "a compute kernel",
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
///
/// The following is a baseline example.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # futures::executor::block_on(assert_device_pool_initialized());
/// # let data = vec![1.0; 2048];
/// # let mut data_on_gpu: DeviceBox<[f32]> = data.as_device_boxed_mut()?;
///
/// let kernel: GlslKernel = GlslKernel::new()
///     .param_mut::<[f32], _>("float[] data")
///     .param::<f32, _>("float scalar")
///     .with_kernel_code("data[gl_GlobalInvocationID.x] = data[gl_GlobalInvocationID.x] * scalar;");
/// let spirv_or_finished = compile::<GlslKernel, GlslKernelCompile, _, GlobalCache>(kernel)?;
/// // now at this point you can call `.finish` to turn `spirv_or_finished` into
/// // a finished `DeviceFnMut`
/// # let finished = spirv_or_finished.finish()?;
/// # unsafe { spawn(2048).launch(call!(finished, &mut data_on_gpu, &DeviceBox::new(10.0f32)?))?; }
/// # assert_eq!(futures::executor::block_on(data_on_gpu.get())?, vec![10.0; 2048].into_boxed_slice());
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "glsl-compile")]
#[derive(Hash)]
pub struct GlslKernel {
    code: String,
    params: Vec<String>,
    params_mutability: Vec<Mutability>,
    params_builder: ParamsBuilder,
    structs: Vec<String>,
    consts: Vec<(String, String)>,
    shared: Vec<String>,
    local_size: Vec<u32>,
    helper_code: String,
    kernel_code: String,
}

#[cfg(feature = "glsl-compile")]
impl GlslKernel {
    /// Initializes the builder
    pub fn new() -> Self {
        Self {
            code: String::from("#version 450\n"),
            params: vec![],
            params_mutability: vec![],
            params_builder: ParamsBuilder::new(),
            structs: vec![],
            consts: vec![],
            shared: vec![],
            local_size: vec![],
            helper_code: String::new(),
            kernel_code: String::new(),
        }
    }

    /// Spawns threads within each thread block
    ///
    /// This essentially adds on a new dimension with the given size to the space of threads for each thread block.
    /// The dimensions are "x", "y", and "z" in that order. If no threads are spawned, the space of threads is 1-dimensional and of size 1.
    /// If this is called more than 3 times, the dimensions are collapsed to a single dimension with size equal to the product of the sizes of all prior dimensions.
    ///
    /// ```
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # futures::executor::block_on(assert_device_pool_initialized());
    /// # let data = vec![1.0; 1 << 20];
    /// # let mut data_on_gpu: DeviceBox<[f32]> = data.as_device_boxed_mut()?;
    ///
    /// let kernel: GlslKernel = GlslKernel::new()
    ///     .spawn(32)
    ///     .spawn(32)
    ///     .param_mut::<[f32], _>("float[] data")
    ///     .param::<f32, _>("float scalar")
    ///     .with_kernel_code(r#"
    /// uint index = (1 << 10) * gl_GlobalInvocationID.x + gl_GlobalInvocationID.y;
    /// data[index] = data[index] * scalar;
    ///     "#);
    /// let spirv_or_finished = compile::<GlslKernel, GlslKernelCompile, _, GlobalCache>(kernel)?;
    /// // now at this point you can call `.finish` to turn `spirv_or_finished` into
    /// // a finished `DeviceFnMut`
    /// # let finished = spirv_or_finished.finish()?;
    /// # unsafe { spawn((1 << 10) / 32).spawn((1 << 10) / 32).launch(call!(finished, &mut data_on_gpu, &DeviceBox::new(10.0f32)?))?; }
    /// # assert_eq!(futures::executor::block_on(data_on_gpu.get())?, vec![10.0; 1 << 20].into_boxed_slice());
    /// # Ok(())
    /// # }
    /// ```
    pub fn spawn(mut self, num_threads: u32) -> Self {
        self.local_size.push(num_threads);
        self
    }

    /// Appends a GLSL structure definition for the type which this function is generic over
    ///
    /// This can be used for any type that implements [`GlslStruct`](../compile/trait.GlslStruct.html).
    /// ```
    /// use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    ///
    /// #[repr(C)]
    /// #[derive(AsBytes, FromBytes, Copy, Clone, Default, Debug, GlslStruct, PartialEq)]
    /// struct Shape {
    ///     pos: [f32; 2],
    ///     num_edges: u32,
    ///     radius: f32
    /// }
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     futures::executor::block_on(assert_device_pool_initialized());
    ///     // create some shapes on the GPU
    ///     let mut shapes: DeviceBox<[Shape]> = vec![Shape {
    ///         pos: [100.0, 100.0],
    ///         num_edges: 6,
    ///         radius: 100.0
    ///     }; 1024].as_device_boxed_mut()?;
    ///
    ///     // define a kernel to scale and translate
    ///     let kernel: GlslKernel = GlslKernel::new()
    ///         .with_struct::<Shape>()
    ///         .param_mut::<[Shape], _>("Shape[] shapes")
    ///         // in practice, you should probably combine scale and translate in 1 struct
    ///         .param::<f32, _>("float scale")
    ///         .param::<[f32; 2], _>("vec2 translate")
    ///         .with_kernel_code(r#"
    /// shapes[gl_GlobalInvocationID.x].pos += translate;
    /// shapes[gl_GlobalInvocationID.x].radius *= scale;
    ///     "#);
    ///     let spirv_or_finished = compile::<GlslKernel, GlslKernelCompile, _, GlobalCache>(kernel)?;
    ///     let finished = spirv_or_finished.finish()?;
    ///
    ///     // run
    ///     unsafe {
    ///         spawn(1024).launch(call!(
    ///             finished, &mut shapes,
    ///             &DeviceBox::new(2.0f32)?,
    ///             &DeviceBox::new([-100.0f32; 2])?
    ///         ))?;
    ///     }
    ///
    ///     // check result
    ///     assert_eq!(futures::executor::block_on(shapes.get())?, vec![Shape {
    ///         pos: [0.0; 2],
    ///         num_edges: 6,
    ///         radius: 200.0
    ///     }; 1024].into_boxed_slice());
    ///     Ok(())
    /// }
    /// ```
    pub fn with_struct<T: GlslStruct>(mut self) -> Self {
        self.structs.push(T::as_glsl());
        self
    }

    /// Appends a constant definition using the give left hand and right hand sides
    ///
    /// ```
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # futures::executor::block_on(assert_device_pool_initialized());
    /// # let data = vec![1.0; 2048];
    /// # let mut data_on_gpu: DeviceBox<[f32]> = data.as_device_boxed_mut()?;
    ///
    /// let kernel: GlslKernel = GlslKernel::new()
    ///     .param_mut::<[f32], _>("float[] data")
    ///     .param::<f32, _>("float scalar")
    ///     .with_kernel_code("data[gl_GlobalInvocationID.x] = data[gl_GlobalInvocationID.x] * scalar + pi;")
    ///     .with_const("int pi", "3");
    /// let spirv_or_finished = compile::<GlslKernel, GlslKernelCompile, _, GlobalCache>(kernel)?;
    /// // now at this point you can call `.finish` to turn `spirv_or_finished` into
    /// // a finished `DeviceFnMut`
    /// # let finished = spirv_or_finished.finish()?;
    /// # unsafe { spawn(2048).launch(call!(finished, &mut data_on_gpu, &DeviceBox::new(10.0f32)?))?; }
    /// # assert_eq!(futures::executor::block_on(data_on_gpu.get())?, vec![13.0; 2048].into_boxed_slice());
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_const(
        mut self,
        left_hand: impl Into<String>,
        right_hand: impl Into<String>,
    ) -> Self {
        self.consts.push((left_hand.into(), right_hand.into()));
        self
    }

    /// Creates a shared variable using the given code
    ///
    /// ```
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # futures::executor::block_on(assert_device_pool_initialized());
    /// # let data = vec![1.0; 2048];
    /// # let mut data_on_gpu: DeviceBox<[f32]> = data.as_device_boxed_mut()?;
    ///
    /// let kernel: GlslKernel = GlslKernel::new()
    ///     .spawn(64)
    ///     .share("float scratchpad[64]")
    ///     .param_mut::<[f32], _>("float[] data")
    ///     .param::<f32, _>("float scalar")
    ///     .with_kernel_code(r#"
    /// scratchpad[gl_LocalInvocationID.x] = data[gl_GlobalInvocationID.x];
    /// // if we had a more complex access pattern, we might want a barrier() right here to ensure
    /// // all memory has been downloaded to the shraed scratchpad
    /// scratchpad[gl_LocalInvocationID.x] = scratchpad[gl_LocalInvocationID.x] * scratchpad[gl_LocalInvocationID.x];
    /// scratchpad[gl_LocalInvocationID.x] = scratchpad[gl_LocalInvocationID.x] * scalar;
    /// scratchpad[gl_LocalInvocationID.x] = scratchpad[gl_LocalInvocationID.x] * 2;
    /// data[gl_GlobalInvocationID.x] = scratchpad[gl_LocalInvocationID.x];
    ///     "#);
    /// let spirv_or_finished = compile::<GlslKernel, GlslKernelCompile, _, GlobalCache>(kernel)?;
    /// // now at this point you can call `.finish` to turn `spirv_or_finished` into
    /// // a finished `DeviceFnMut`
    /// # let finished = spirv_or_finished.finish()?;
    /// # unsafe { spawn(2048 / 64).launch(call!(finished, &mut data_on_gpu, &DeviceBox::new(10.0f32)?))?; }
    /// # assert_eq!(futures::executor::block_on(data_on_gpu.get())?, vec![20.0; 2048].into_boxed_slice());
    /// # Ok(())
    /// # }
    /// ```
    pub fn share(mut self, shared: impl Into<String>) -> Self {
        self.shared.push(shared.into());
        self
    }

    /// Generates code for a buffer through which constant data can be passed into the kernel
    pub fn param<T: ?Sized, I: Into<String>>(mut self, param: I) -> Self {
        self.params_builder = self.params_builder.param::<T>(Mutability::Const);
        self.params.push(param.into());
        self.params_mutability.push(Mutability::Const);
        self
    }

    /// Generates code for a buffer through which mutable data can be passed into the kernel
    pub fn param_mut<T: ?Sized, I: Into<String>>(mut self, param: I) -> Self {
        self.params_builder = self.params_builder.param::<T>(Mutability::Mut);
        self.params.push(param.into());
        self.params_mutability.push(Mutability::Mut);
        self
    }

    /// Adds the given helper code
    ///
    /// This helper code may include additional type or function definitions.
    /// ```
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # futures::executor::block_on(assert_device_pool_initialized());
    /// # let data = vec![1.0; 2048];
    /// # let mut data_on_gpu: DeviceBox<[f32]> = data.as_device_boxed_mut()?;
    ///
    /// let kernel: GlslKernel = GlslKernel::new()
    ///     .param_mut::<[f32], _>("float[] data")
    ///     .with_helper_code(r#"
    /// float invert(float x) {
    ///     return -x;
    /// }
    ///     "#)
    ///     .with_kernel_code("data[gl_GlobalInvocationID.x] = invert(data[gl_GlobalInvocationID.x]);");
    /// let spirv_or_finished = compile::<GlslKernel, GlslKernelCompile, _, GlobalCache>(kernel)?;
    /// // now at this point you can call `.finish` to turn `spirv_or_finished` into
    /// // a finished `DeviceFnMut`
    /// # let finished = spirv_or_finished.finish()?;
    /// # unsafe { spawn(2048).launch(call!(finished, &mut data_on_gpu))?; }
    /// # assert_eq!(futures::executor::block_on(data_on_gpu.get())?, vec![-1.0; 2048].into_boxed_slice());
    /// # Ok(())
    /// # }
    /// ```
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

/// Another `shaderc`-based compiler for compiling [`GlslKernel`](struct.GlslKernel.html)
#[cfg(feature = "glsl-compile")]
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
        for (i, param) in src.params.iter().enumerate() {
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

        // (6) helper code
        src.code += &src.helper_code;

        // (7) kernel code
        src.code += "\nvoid main() {\n";
        src.code += &src.kernel_code;
        src.code += "}\n";

        // (8) compile to SPIR-V
        let mut compiler = shaderc::Compiler::new().unwrap();
        let binary_result = compiler
            .compile_into_spirv(
                &src.code,
                shaderc::ShaderKind::Compute,
                "a compute kernel",
                "main",
                None,
            )
            .unwrap();

        // yes, copying the binary over into a vec is expensive
        // but it's necessary so that we can allow users to mutate binary later on
        // and the copying of the binary is dwarfed by many other operations of this library
        // also, we cache anyway
        Ok(Spirv {
            params: src.params_builder.build(),
            name: kernel_name,
            code: binary_result.as_binary().to_vec(),
        })
    }
}
