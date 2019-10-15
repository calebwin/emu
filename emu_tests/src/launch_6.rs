use em::*;

// this will fail because unsupported functionality is used
#[gpu_use]
fn main() {
	let mut data = vec![0.0; 1000];

	gpu_do!(load(data));
	gpu_do!(launch());
	for i in 0..1000 {
		data[i] = data[i] + 0.0;
	}
	gpu_do!(read(data));
}