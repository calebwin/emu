# Emu
Emu is a procedural macro for writing numerical computation scripts. Emu translates your code to clean, compact OpenCL code at compile time and stores the translated code in the `EMU` global constant. You can then use OpenCL bindings such as [`ocl`](https://github.com/cogciprocate/ocl) to run code from your `EMU` on pretty much any GPU or CPU.
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
