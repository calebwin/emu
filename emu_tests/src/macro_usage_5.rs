use em::*;

// this won't pass because of an invalid syntax for the attribute macro invocation
#[gpu_use(x = "hello")]
fn main() {
}