use em::*;

#[gpu_use(foo)]
fn foo() {

}

// this will pass because closures are looked at
#[gpu_use(foo)]
fn main() {
	let _ = {
		foo();
		5
	};

	foo();
}