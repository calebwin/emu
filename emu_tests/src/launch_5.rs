use em::*;

// this will fail because unsupported functionality is used
#[gpu_use]
fn main() {
	let data = vec![0.0; 1000];

	gpu_do!(load(data));
	gpu_do!(launch());
	for i in 0..1000 {
		data[i] = data[i + 0];
		data[i] = data[i] + 0.0f64;
		data[i] = data[i] as f32;
		if true {
			data[i] = data[i];
		}
		data[i] = true;
		let x = data[i];
		fn foo () {

		}
	}
	gpu_do!(read(data));
}