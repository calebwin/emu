use em::*;

// this will pass because we cannot know if a helper function is defined yet
#[gpu_use(add_vector)]
fn main() {

}