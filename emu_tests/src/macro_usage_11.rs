use em::*;

#[gpu_use(foo, bar)]
fn foo(x: i32) {
	let _ = bar(x);
}

#[gpu_use(bar, baz)]
fn bar(z: i32) -> i32 {
	baz(0, z)
}

#[gpu_use(baz)]
fn baz(x: i32, y: i32) -> i32 {
	if x > 0 {
		return 5;
	} else if y > 0 {
		0
	} else {
		return 3;
	}
}

// this will pass because GPU is passed on correctly
#[gpu_use(foo)]
fn main() {
	foo(5);
}