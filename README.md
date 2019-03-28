# Emu
Emu is a programming language designed to make GPU-accelerated numerical computing more accessible in Rust.
```rust
emu! {
	// adds a scalar to elements of a buffer
	// buffer must be array of floats in global address space, scaler must be float
	add (buffer as [f32]:GLOBAL, scalar as f32) {
		buffer[get_global_id(0)] += scalar;
	}

	// multiplies elements of a buffer by a scalar
	// buffer must be array of floats in global address space, scaler must be float
	multiply (buffer as [f32]:GLOBAL, scalar as f32) {
		buffer[get_global_id(0)] *= scalar;
	}
}
```
