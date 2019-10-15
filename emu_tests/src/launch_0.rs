use em::*;

// this will fail because an undefined for loop is used
#[gpu_use]
fn main() {
	let data = vec![0.0; 1000];

	gpu_do!(load(data));
	gpu_do!(launch());
	for x in data {

	}
	gpu_do!(read(data));
}