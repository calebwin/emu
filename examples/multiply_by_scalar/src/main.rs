// This example has been taken from the OCL crate

// emu for writing OpenCL code
extern crate em;
use em::emu;

// ocl for executing OpenCL code
extern crate ocl;
use ocl::{ProQue, Buffer, MemFlags};

// const EMU: &'static str = r#"
// 	__kernel void multiply(__global float* buffer, __private float scalar) {
//         buffer[get_global_id(0)] *= scalar;
//         buffer[get_global_id(0)] = pi();
//     }
// "#;

emu! {
	// multiplies elements of a buffer by a scalar
	multiply(global_buffer [f32], scalar f32) {
		global_buffer[get_global_id(0)] *= scalar;
	}
}

/// Expanded version with explanations.
///
/// All four functions in this example are functionally identical.
///
/// Continue along to `::trivial_exploded` and `::trivial_cored` to see what's
/// going on under the hood.
///
fn multiply(data: Vec<f32>, coeff: f32) -> ocl::Result<Vec<f32>> {
	// (1) Create an all-in-one context, program, command queue, and work /
	// buffer dimensions:
	let ctx = ProQue::builder()
        	.src(EMU)
	        .dims(data.len())
        	.build()?;

	// (2) Create a `Buffer`:
    let buffer = Buffer::builder()
        .queue(ctx.queue().clone())
        .len(data.len())
        .copy_host_slice(&data).build()?;

	// (3) Create a kernel with arguments matching those in the source above:
	let kernel = ctx.kernel_builder("multiply")
		.arg(&buffer)
		.arg(&coeff)
		.build()?;

	// (4) Run the kernel:
	unsafe { kernel.enq()?; }

	// (5) Read results from the device into a vector:
	let mut vec = vec![0.0f32; buffer.len()];
	buffer.read(&mut vec).enq()?;

	Ok(vec)
}


fn main() {
	println!("{:?}", multiply(vec![3.7, 4.5, 9.0, 1.2, 8.9], 3.0).unwrap());
}