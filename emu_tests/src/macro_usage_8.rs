use em::*;

// this will fail because a declared helper function must be an identifier
#[gpu_use(em::do_something)]
fn main() {
}