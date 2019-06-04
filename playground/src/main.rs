// // A playground for experimenting with potentially better ways of letting a user run intermediate code
// // Currently, the only way of running intermediate code is with a binding to OpenCL or using the build! macro
// // which generates Rust functions that call an Emu function from intermediate code on a bunch of elements in a Vec
// // But are there more expressive ways of running the intermediate code?

// // emu for writing code
// extern crate em;
// use em::emu;
// use em::build;

// // ocl for emu for packaging code in Rust functions that can be called
// extern crate ocl;
// use ocl::{flags, Platform, Device, Context, Queue, Program, Buffer, Kernel};

// emu! {
// 	multiply(global_input [f32], coeff f32) {
// 		global_input[get_global_id(0)] *= coeff;
// 	}
// }

// build! {
// 	multiply [f32] f32
// }

// fn main() {
// 	let my_data = vec![3.0, 9.8, 3.5];

// 	// this will not work right now
// 	for elem in my_data.emu_iter() {
// 		multiply(my_data, 3.8);
// 	}
// }

// use em::{build, emu};

// extern crate ocl;
// use ocl::{flags, Buffer, Context, Device, Kernel, Platform, Program, Queue};

// emu! {
//     multiply(global_vector [f32], scalar f32) {
//         global_vector[get_global_id(0)] *= scalar;
//     }
// }

// build! { multiply [f32] f32 }

// fn main() {
//     let args = std::env::args().collect::<Vec<String>>();
//     if args.len() < 3 {
//         panic!("cargo run -- <SCALAR> <NUMBERS>...");
//     }

//     let scalar = args[1].parse::<f32>().unwrap();

//     let vector = args[2..]
//         .into_iter()
//         .map(|string| string.parse::<f32>().unwrap())
//         .collect();

//     let result = multiply(vector, scalar).unwrap();
//     dbg!(result);
// }

// This example has been taken from the OCL crate

// emu for writing code
extern crate em;
use em::emu;
use em::build;

// ocl for emu for packaging code in Rust functions that can be called
extern crate ocl;
use ocl::{flags, Platform, Device, Context, Queue, Program, Buffer, Kernel};

// const EMU: &'static str = r#"
//  __kernel void multiply(__global float* buffer, __private float scalar) {
//         buffer[get_global_id(0)] *= scalar;
//     }
// "#;

// compile Emu code to intermediate code (OpenCL as of now) and store it in const EMU: &'static str
emu! {
    // multiplies elements of a buffer by a scalar
    multiply_matrices(m i32, n i32, k i32, global_a [f32], global_b [f32], global_c [f32]) {
        let row: i32 = get_global_id(0);
        let col: i32 = get_global_id(1);

        let acc: f32 = 0.0;
 
        for i in 0..k {
            acc += global_a[i*m + row] * global_b[col*k + i];
        }
     
        // Store the result
        global_c[col * m + row] = acc;
    }
}

fn multiply(m: i32, n: i32, k: i32, global_a: Vec<f32>, global_b: Vec<f32>, global_c: Vec<f32>) -> ocl::Result<Vec<f32>> {

    // (1) Define which platform and device(s) to use. Create a context,
    // queue, and program then define some dims (compare to step 1 above).
    let platform = Platform::default();
    let device = Device::first(platform)?;
    let context = Context::builder()
        .platform(platform)
        .devices(device.clone())
        .build()?;
    let program = Program::builder()
        .devices(device)
        .src(EMU)
        .build(&context)?;
    let queue = Queue::new(&context, device, None)?;
    // let dims = global_buffer.len();
    let dims_0 = 3;
    let dims_1 = 3;
    // [NOTE]: At this point we could manually assemble a ProQue by calling:
    // `ProQue::new(context, queue, program, Some(dims))`. One might want to
    // do this when only one program and queue are all that's needed. Wrapping
    // it up into a single struct makes passing it around simpler.

    // (2) Create a `Buffer`:
    let buffer_a = Buffer::<f32>::builder()
        .queue(queue.clone())
        .flags(flags::MEM_READ_WRITE)
        .len(global_a.len())
        .copy_host_slice(&global_a)
        .build()?;

    let buffer_b = Buffer::<f32>::builder()
        .queue(queue.clone())
        .flags(flags::MEM_READ_WRITE)
        .len(global_b.len())
        .copy_host_slice(&global_b)
        .build()?;

    let buffer_c = Buffer::<f32>::builder()
        .queue(queue.clone())
        .flags(flags::MEM_READ_WRITE)
        .len(global_c.len())
        .copy_host_slice(&global_c)
        .build()?;

    // (3) Create a kernel with arguments matching those in the source above:
    let kernel = Kernel::builder()
        .program(&program)
        .name("multiply_matrices")
        .queue(queue.clone())
        .global_work_size([dims_0, dims_1])
        .arg(&m)
        .arg(&n)
        .arg(&k)
        .arg(&buffer_a)
        .arg(&buffer_b)
        .arg(&buffer_c)
        .build()?;

    // (4) Run the kernel (default parameters shown for demonstration purposes):
    unsafe {
        kernel.cmd()
            .queue(&queue)
            .global_work_offset(kernel.default_global_work_offset())
            .global_work_size([dims_0, dims_1])
            .local_work_size(kernel.default_local_work_size())
            .enq()?;
    }

    // (5) Read results from the device into a vector (`::block` not shown):
    let mut vec = vec![0.0f32; dims_0*dims_1];
    buffer_c.cmd()
        .queue(&queue)
        .offset(0)
        .read(&mut vec)
        .enq()?;

    Ok(vec)
}


fn main() {
    let m: i32 = 3;
    let n: i32 = 3;
    let k: i32 = 3;

    let a = vec![3.7, 4.5, 9.0, 3.7, 4.5, 9.0, 3.7, 4.5, 9.0];
    let b = vec![3.7, 4.5, 9.0, 3.7, 4.5, 9.0, 3.7, 4.5, 9.0];
    let mut c = vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

    c = multiply(m, n, k, a, b, c).unwrap();

    println!("{:?}", c);
}