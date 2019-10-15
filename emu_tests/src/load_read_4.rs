use em::*;

// this will fail because of incorrect data types
#[gpu_use]
fn main() {
	let data = vec![0.0f64; 1000];

	gpu_do!(load(data));
	gpu_do!(launch());
	for i in 0..1000 {
		data[i] = data[i];
	}
	gpu_do!(read(data));
}