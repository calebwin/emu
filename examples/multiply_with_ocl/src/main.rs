// This example has been taken from the OCL crate

// emu for writing code
extern crate em;
use em::emu;
use em::build;

// ocl for emu for packaging code in Rust functions that can be called
extern crate ocl;
use ocl::{flags, Platform, Device, Context, Queue, Program, Buffer, Kernel};

// const EMU: &'static str = r#"
// 	__kernel void multiply(__global float* buffer, __private float scalar) {
//         buffer[get_global_id(0)] *= scalar;
//     }
// "#;

// compile Emu code to intermediate code (OpenCL as of now) and store it in const EMU: &'static str
emu! {
	// multiplies elements of a buffer by a scalar
	multiply(global_buffer [f32], coeff f32) {
		global_buffer[get_global_id(0)] *= coeff;
	}
}

fn multiply(global_buffer: Vec<f32>, coeff: f32) -> ocl::Result<Vec<f32>> {

    // (1) Define which platform and device(s) to use. Create a context,
    // queue, and program then define some dims (compare to step 1 above).
    let platform = Platform::default();
    let device = Device::first(platform)?;
    let context = Context::builder()
        .platform(platform)
        .devices(device.clone())
        .build()?;
    let program = Program::builder()
        .devices(device)
        .src(EMU)
        .build(&context)?;
    let queue = Queue::new(&context, device, None)?;
    let dims = global_buffer.len();
    // [NOTE]: At this point we could manually assemble a ProQue by calling:
    // `ProQue::new(context, queue, program, Some(dims))`. One might want to
    // do this when only one program and queue are all that's needed. Wrapping
    // it up into a single struct makes passing it around simpler.

    // (2) Create a `Buffer`:
    let buffer = Buffer::<f32>::builder()
        .queue(queue.clone())
        .flags(flags::MEM_READ_WRITE)
        .len(dims)
        .copy_host_slice(&global_buffer)
        .build()?;

    // (3) Create a kernel with arguments matching those in the source above:
    let kernel = Kernel::builder()
        .program(&program)
        .name("multiply")
        .queue(queue.clone())
        .global_work_size(dims)
        .arg(&buffer)
        .arg(&coeff)
        .build()?;

    // (4) Run the kernel (default parameters shown for demonstration purposes):
    unsafe {
        kernel.cmd()
            .queue(&queue)
            .global_work_offset(kernel.default_global_work_offset())
            .global_work_size(dims)
            .local_work_size(kernel.default_local_work_size())
            .enq()?;
    }

    // (5) Read results from the device into a vector (`::block` not shown):
    let mut vec = vec![0.0f32; dims];
    buffer.cmd()
        .queue(&queue)
        .offset(0)
        .read(&mut vec)
        .enq()?;

    Ok(vec)
}


fn main() {
	// a vector with elements of type f32
	let initial_data = vec![3.7, 4.5, 9.0, 1.2, 8.9];

	// call the multiply function written in Emu on the vector of data
	let final_data = multiply(initial_data, 3.0).unwrap();

	// print the results to the console
	println!("{:?}", final_data);
}