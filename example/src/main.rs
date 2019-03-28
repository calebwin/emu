extern crate emu;

use emu::emu;

// src! {
// 	r#"
// 	__kernel void multiply_by_scalar(
// 		__private float const coeff,
// 		__global float const* const src,
// 		__global float* const res)
// 	{
// 		uint const idx = get_global_id(0);
// 		res[idx] = src[idx] * coeff;
// 	}
// 	"#
// }

// parameters
// - default: in __private as given type
// - global or private

// static program: &'static str = r#"
// __kernel void multiply_by_scalar(__private float)
// {
	
// }
// "#;

src! {
	/// Multiplies number by coeffecient and puts result in res
	multiply_by_scalar(coeff as f32, num as [f32] : GLOBAL, res as [f32] : GLOBAL) {
		index = get_global_id(0);
		res[index] = num[index] * coeff;
	}
	// fn multiply_by_scalar(coeff, num, res) {
	// 	// index = l_id(0);
	// 	// res[index] = num[index] * coeff
	// 	6 as km + 300 as m
	// }
}

fn main() {
    // let result = answer();
    // println!("{:?}", result);
}
