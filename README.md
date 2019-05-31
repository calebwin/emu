![a picture of a real-world emu](https://i.imgur.com/jraDjSK.jpg)

Emu is a high-level language for numerical, GPGPU or CPU-based computation embedded in Rust. Emu provides a single procedural macro for writing functions that do numerical computation. <!--(As of now, these functions get automatically translated to clean, compact OpenCL code at compile time and stored in the `EMU` global constant, which can then be run using any binding to OpenCL such as [`ocl`](https://github.com/cogciprocate/ocl) or [`rust-opencl`](https://github.com/luqmana/rust-opencl).--> Your Emu code is compiled by the procedural macro into a lower-level code, stored in `EMU`, and then run by different back-ends ultimately executing on GPU or CPU.

As a high-level language for numerical computing, Emu is focused more on that first part, providing useful features specifically for doing numerical (and scientific) computation such as built-in mathematical and physical constants, unit annotation and implicit conversion.
```rust
emu! {
	// more particles
	more_particles(num_particles u32, num_moles u32) u32 {
		return num_particles + num_moles * L;
	}

	// moves particles
	move_particles(global_particles_x [f32], global_particles_y f32, global_particles_z f32) {
		global_particles_z[get_global_id(0)] += 7.3e1 as nm;
		global_particles_x[get_global_id(0)] += 2 as cm;
		global_particles_y[get_global_id(0)] += 6 as cm;
	}
	
	// moves particles in circle
	rotate_particles(global_particles_r [f32]) {
		global_particles_r [f32] += 7.5 * TAU;
	}

	// multiplies 2 matrices
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
```
 More details can be found in [**the book**](https://github.com/calebwin/emu/tree/master/book) and [**the examples**](https://github.com/calebwin/emu/tree/master/examples) and [**the crate**](https://crates.io/crates/em).
