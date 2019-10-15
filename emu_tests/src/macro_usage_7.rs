use em::*;

// this will fail because the function cannot be extern
#[gpu_use(do_something)]
extern fn do_something() {
}

#[gpu_use]
fn main() {
}