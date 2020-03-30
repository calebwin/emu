use emu_core::prelude::*;
use emu_glsl::*;
use zerocopy::*;

#[repr(C)]
#[derive(AsBytes, FromBytes, Copy, Clone, Default, Debug, GlslStruct)]
struct Shape {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    r: [i32; 2]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// ensure that a device pool has been initialized
	// this should be called before every time when you assume you have devices to use
	// that goes for both library users and application users
	futures::executor::block_on(assert_device_pool_initialized());

	// create some data on GPU
	// even mutate it once loaded to GPU
	let mut shapes: DeviceBox<[Shape]> = vec![Default::default(); 1024].as_device_boxed()?;
	shapes.set(vec![Default::default(); 1024]);

	// compiel some code
	// then, run it
	let c = compile::<GlslKernel, GlslKernelCompile, _, GlobalCache>(
        GlslKernel::new()
            .spawn(1)
            .param_mut("Shape[] shapes")
            .with_struct::<Shape>()
            .with_helper_code(
                r#"
Shape flip(Shape s) {
    s.x = s.x + s.w;
    s.y = s.y + s.h;
    s.w *= -1;
    s.h *= -1;
    return s;
}
"#,
            )
            .with_kernel_code(
                "shapes[gl_GlobalInvocationID.x] = flip(shapes[gl_GlobalInvocationID.x]);",
            ),
    )?;
    unsafe {
        spawn(128).launch(call!(c, &mut shapes))?;
    }

	// download from GPU and print out
	println!("{:?}", futures::executor::block_on(shapes.get())?);
	Ok(())
}