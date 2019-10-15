use em::*;

// this will fail because block is used
#[gpu_use]
fn main() {
	let data = vec![0.0; 1000];

	gpu_do!(load(data));
	gpu_do!(launch());
	for i in 0..1000 {
		data[i] = {
			0.0
		};
	}
	gpu_do!(read(data));
}