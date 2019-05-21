// This example has been taken from the OCL crate

extern crate emu;
use emu::emu;

extern crate ocl;
use ocl::ProQue;

emu! {
	// computes MSE of 2 arrays
	mse(global_a [f32], global_b[f32], res) {
		// initial total square error
		let total_squared_error: f32 = 0;

		// return result
		res = total_squared_error;
	}
}

fn mse() -> ocl::Result<()> {
	Ok(())
}

fn main() {
	unimplemented!();
}