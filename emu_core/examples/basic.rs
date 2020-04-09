use emu_core::prelude::*;
use emu_glsl::*;
use zerocopy::*;

#[repr(C)]
#[derive(AsBytes, FromBytes, Copy, Clone, Default, Debug, GlslStruct)]
struct Shape {
    x: u32,
    y: u32,
    w: i32,
    h: i32,
    r: [i32; 2],
}

#[cfg(not(feature = "glsl-compile"))]
fn main() {}

#[cfg(feature = "glsl-compile")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ensure that a device pool has been initialized
    // this should be called before every time when you assume you have devices to use
    // that goes for both library users and application users
    futures::executor::block_on(assert_device_pool_initialized());

    println!("{:?}", take()?.lock().unwrap().info.as_ref().unwrap());

    // create some data on GPU
    // even mutate it once loaded to GPU
    let mut shapes: DeviceBox<[Shape]> = vec![Default::default(); 1024].as_device_boxed_mut()?;
    let mut x: DeviceBox<[i32]> = vec![0; 1024].as_device_boxed_mut()?;
    shapes.set(vec![
        Shape {
            x: 0,
            y: 0,
            w: 100,
            h: 100,
            r: [2, 9]
        };
        1024
    ])?;

    // compile GslKernel to SPIR-V
    // then, we can either inspect the SPIR-V or finish the compilation by generating a DeviceFnMut
    // then, run the DeviceFnMut
    let c = compile::<GlslKernel, GlslKernelCompile, Vec<u32>, GlobalCache>(
        GlslKernel::new()
            .spawn(64)
            .share("float stuff[64]")
            .param_mut::<[Shape], _>("Shape[] shapes")
            .param_mut::<[i32], _>("int[] x")
            .param::<i32, _>("int scalar")
            .with_struct::<Shape>()
            .with_const("int c", "7")
            .with_helper_code(
                r#"
Shape flip(Shape s) {
    s.x = s.x + s.w;
    s.y = s.y + s.h;
    s.w *= -1;
    s.h *= -1;
    s.r = ivec2(5, 3);
    return s;
}
"#,
    )
    .with_kernel_code(
        "shapes[gl_GlobalInvocationID.x] = flip(shapes[gl_GlobalInvocationID.x]); x[gl_GlobalInvocationID.x] = scalar + c + int(gl_WorkGroupID.x);",
    ),
)?.finish()?;
    unsafe {
        spawn(1024 / 64).launch(call!(c, &mut shapes, &mut x, &DeviceBox::new(10)?))?;
    }

    // download from GPU and print out
    println!("{:?}", futures::executor::block_on(shapes.get())?);
    println!("{:?}", futures::executor::block_on(x.get())?);
    Ok(())
}
