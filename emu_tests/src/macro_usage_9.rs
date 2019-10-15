use em::*;

#[gpu_use(foo)]
fn foo() {

}

// this will pass because functions are not looked at
#[gpu_use(foo)]
fn main() {
	fn bar(gpu: Gpu) {
		foo(gpu);
	}

	foo();
}