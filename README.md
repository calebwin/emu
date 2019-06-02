<!--![a picture of a real-world emu](https://i.imgur.com/jraDjSK.jpg)-->
![a picture of a real-world emu](https://i.imgur.com/8CeUiar.jpg)

Emu is a high-level language for programming GPUs. Unlike other languages such as OpenCL or Halide that are designed for embedding in C or C++, Emu is designed for embedding in Rust. It provides a single procedural macro for writing functions. <!--(As of now, these functions get automatically translated to clean, compact OpenCL code at compile time and stored in the `EMU` global constant, which can then be run using any binding to OpenCL such as [`ocl`](https://github.com/cogciprocate/ocl) or [`rust-opencl`](https://github.com/luqmana/rust-opencl).--> The macro translates the functions at compile time into lower-level code so that they can be run on the GPU.

<!--As a high-level language for numerical computing, Emu is focused on providing useful features specifically for doing numerical (and scientific) computation such as built-in mathematical and physical constants, unit annotation and implicit conversion.--->
Emu also provides several features that aim to make programming GPUs more accessible such as built-in mathematical and physical constants, unit annotation and implicit conversion. Here are some example functions...
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
		global_particles_r[get_global_id(0)] += 7.5 * TAU;
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
 To get started programming GPUs with Emu, check out [**the book**](https://github.com/calebwin/emu/tree/master/book), [**the examples**](https://github.com/calebwin/emu/tree/master/examples), and [**the crate**](https://crates.io/crates/em) itself.
