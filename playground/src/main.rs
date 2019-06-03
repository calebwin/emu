// A playground for experimenting with potentially better ways of letting a user run intermediate code
// Currently, the only way of running intermediate code is with a binding to OpenCL or using the build! macro
// which generates Rust functions that call an Emu function from intermediate code on a bunch of elements in a Vec
// But are there more expressive ways of running the intermediate code?

// emu for writing code
extern crate em;
use em::emu;
use em::build;

// ocl for emu for packaging code in Rust functions that can be called
extern crate ocl;
use ocl::{flags, Platform, Device, Context, Queue, Program, Buffer, Kernel};

emu! {
	multiply(global_input [f32], coeff f32) {
		global_input[get_global_id(0)] *= coeff;
	}
}

build! {
	multiply [f32] f32
}

fn main() {
	let my_data = vec![3.0, 9.8, 3.5];

	// this will not work right now
	for elem in my_data.emu_iter() {
		multiply(my_data, 3.8);
	}
}
