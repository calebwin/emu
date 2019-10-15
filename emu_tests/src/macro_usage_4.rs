use em::*;

// this won't pass because of an invalid syntax for the attribute macro invocation
// however, usage of {} instead of () is valid
#[gpu_use = 100]
fn main() {
}