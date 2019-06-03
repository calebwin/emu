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
	multiply(global_buffer [f32], scalar f32) {
		global_buffer[get_global_id(0)] *= scalar;
	}
}

// translate intermediate code to Rust functions that can be called
build! { multiply [f32] f32 }


fn main() {
	// a vector with elements of type f32
	let initial_data = vec![3.7, 4.5, 9.0, 1.2, 8.9];

	// call the multiply function written in Emu on the vector of data
	let final_data = multiply(initial_data, 3.0).unwrap();

	// print the results to the console
	println!("{:?}", final_data);
}