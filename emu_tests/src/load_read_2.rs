use em::*;

// this will suceed because the data doesn't need to be mutable to load it to GPU
#[gpu_use]
fn main() {
	let data = vec![0.0; 1000];

	gpu_do!(load(data));
}