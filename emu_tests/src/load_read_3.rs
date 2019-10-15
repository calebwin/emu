use em::*;

// this will fail because the data is not declared mutable
#[gpu_use]
fn main() {
	let data = vec![];

	gpu_do!(load(data));
	gpu_do!(read(data));
}