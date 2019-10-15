use em::*;

// this will fail because an undefined for loop is used
#[gpu_use]
fn main() {
	let data = vec![0.0; 1000];

	gpu_do!(load(data));
	gpu_do!(launch());
	for (ii, i) in (0..100).enumerate() {

	}
	gpu_do!(read(data));
}