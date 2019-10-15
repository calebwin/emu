use em::*;

// this will fail because the data does not have as_slice method
#[gpu_use]
fn main() {
	let data = String::new();

	gpu_do!(load(data));
}