// This example has been taken from the OCL crate

extern crate emu;
use emu::emu;

extern crate ocl;
use ocl::ProQue;

// const EMU: &'static str = r#"
// 	__kernel void add (__global float buffer*, __private float scalar) {
// 		buffer[get_global_id(0)] += scalar;
// 	}

// 	__kernel void multiply (__global float buffer*, __private float scalar) {
// 		buffer[get_global_id(0)] *= scalar;
// 	}
// "#;

emu! {
	// adds a scalar to elements of a buffer
	add(global_buffer [f32], scalar f32) {
		global_buffer[get_global_id(0)] += scalar;
	}

	// multiplies elements of a buffer by a scalar
	multiply(global_buffer [f32], scalar f32) {
		global_buffer[get_global_id(0)] *= scalar;
	}

	// multiply 2 matrices
	// n is the dimension of the matrices
	// a and b are the matrices to be multiplied, c is the result
	multiply_matrices(n i32, global_a [f32], global_b [f32], global_c [f32]) {
		// indices of cells to multiply
		let i: i32 = get_global_id(0);
		let j: i32 = get_global_id(1);

		// execute step of multiplication
		for k in 0..n {
			global_c[i * n + j] += global_a[i * n + k] * global_b[k * n + j];
		}
	}
}

/// Expanded version with explanations.
///
/// All four functions in this example are functionally identical.
///
/// Continue along to `::trivial_exploded` and `::trivial_cored` to see what's
/// going on under the hood.
///
#[allow(dead_code)]
fn trivial() -> ocl::Result<()> {
	// (1) Create an all-in-one context, program, command queue, and work /
	// buffer dimensions:
	let pro_que = ProQue::builder()
        	.src(EMU)
	        .dims(1 << 20)
        	.build()?;

	// (2) Create a `Buffer`:
	let buffer = pro_que.create_buffer::<f32>()?;

	// (3) Create a kernel with arguments matching those in the source above:
	let kernel = pro_que.kernel_builder("multiply")
		.arg(&buffer)
		.arg(&10.0f32)
		.build()?;

	// (4) Run the kernel:
	unsafe { kernel.enq()?; }

	// (5) Read results from the device into a vector:
	let mut vec = vec![0.0f32; buffer.len()];
	buffer.read(&mut vec).enq()?;

	// Print an element:
	println!("The value at index [{}] is now '{}'!", 200007, vec[200007]);
	Ok(())
}


fn main() {
	trivial().unwrap();
}