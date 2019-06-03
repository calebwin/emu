# Execution
By now, you should have a good idea of what kind of functions you can write in the Emu language. But how do you actually run your functions? Let's say you have a function for multiplying numbers by a scalar.
```rust
extern crate em;
use em::emu;

emu! {
	multiply(global_buffer [f32], scalar f32) {
		global_buffer[get_global_id(0)] *= scalar;
	}
}
```
And let's say you have a vector whose values you want to scale by a certain number.
```rust
fn main() {
        let my_vector = vec![0.0, 9.8, 2.5, 9.2, 4.6];
}
```
Wouldn't it be really great if I could just write the following?
```rust
fn main() {
        let my_vector = vec![0.0, 9.8, 2.5, 9.2, 4.6];
        
        let my_vector_scaled = multiply(my_vector, 3.0);
}
```
What if I told you you could do that with Emu? All you need is a way of auto-generating a `multiply()` function that can take the code stored in the `EMU` global constant by the `emu!` macro and run it on the GPU. And this possible - you just need to make a call to the `build!` macro to generate that code for you, a macro that comes nicely packaged in the `em` crate you are already using.
```rust
use em::build;

extern crate ocl;
use ocl::{flags, Platform, Device, Context, Queue, Program, Buffer, Kernel};

build! { multiply [f32] f32 }

fn main() {
        let my_vector = vec![0.0, 9.8, 2.5, 9.2, 4.6];
        
        let my_vector_scaled = multiply(my_vector, 3.0);
}
```
And if we run this, what happens is the `multiply()` function we wrote in Emu will get called for each element of its first parameter (in this case, `my_vector`). An important thing to note is that if you want to, it is entirely possible to just take the code in `EMU` and use a binding to OpenCL to run it yourself. This is possible and what the `emu!` macro was initially developed for.

