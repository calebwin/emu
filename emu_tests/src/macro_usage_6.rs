use em::*;

// this will fail because the function cannot be async
#[gpu_use(do_something)]
async fn do_something() {

}

#[gpu_use]
fn main() {
}