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
	add(global_res [i32], global_a [i32], global_b [i32]) {
		global_res[get_global_id(0)] = global_a[get_global_id(0)] + global_b[get_global_id(0)];
	}
}

// translate intermediate code to Rust functions that can be called
build! { add [i32] [i32] [i32] }


fn main() {
	// a vector with elements of type f32
	let initial_data = vec![0, 0, 0];
	let a            = vec![1, 9, 8];
	let b            = vec![9, 2, 7];

	// call the multiply function written in Emu on the vector of data
	let final_data = add(initial_data, a, b).unwrap();

	// print the results to the console
	println!("{:?}", final_data);
}