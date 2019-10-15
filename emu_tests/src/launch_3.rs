use em::*;

// this will fail because no for loop is used
#[gpu_use]
fn main() {
	let data = vec![0.0; 1000];

	gpu_do!(load(data));
	gpu_do!(launch());
	data[0] = data[0] + 1.0;
	gpu_do!(read(data));
}