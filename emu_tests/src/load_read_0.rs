use em::*;

// this will fail because the data is not defined
#[gpu_use]
fn main() {
	gpu_do!(load(data));
}