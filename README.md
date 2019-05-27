# Emu
Emu is a language for numerical computation embedded in Rust. Emu provides a single procedural macro for writing numerical computation scripts which get automatically translated to clean, compact OpenCL code at compile time and stored in the `EMU` global constant. To run the compile code, you can use any binding to OpenCL such as [`ocl`](https://github.com/cogciprocate/ocl) or [`rust-opencl`](https://github.com/luqmana/rust-opencl). More details can be found in [the book](https://github.com/calebwin/emu/tree/master/book), [the examples](https://github.com/calebwin/emu/tree/master/examples).
```rust
emu! {
	// adds a scalar to elements of a buffer
	add(global_buffer [f32], scalar f32) {
		global_buffer[get_global_id(0)] += scalar;
	}

	// multiplies elements of a buffer by a scalar
	multiply(global_buffer [f32], scalar f32) {
		global_buffer[get_global_id(0)] *= scalar;
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

Emu is designed for numerical computation, for working with numbers. Therefore, the following are supported...
- Numbers from vectors - Numbers can be moved from a Rust `Vec` into an OpenCL kernel
- Numbers operated on - Numbers can be added, subtracted, multiplied, divided, exponentiated, and more
- Numbers converted to - Numbers can be implictly converted to consistent units of measurement
