use em::*;

// this will fail, however, because the compiler will attempt to call the function and won't find it
#[gpu_use(do_something)]
fn main() {
	do_something();
}