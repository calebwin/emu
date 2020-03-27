#[macro_use]
extern crate emu_core;
use emu_core::boxed::*;
use emu_core::device::*;
use emu_core::error::CompletionError;
use emu_core::pool::*;
use emu_core::r#fn::*;
use zerocopy::*;

#[macro_use]
extern crate timeit;

#[repr(C)]
#[derive(AsBytes, FromBytes, Copy, Clone, Default, Debug)]
struct Rectangle {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

async fn do_some_stuff() -> Result<(), Box<dyn std::error::Error>> {
    // let mut d = take()?.lock()?;

    // let mut x = vec![0u32; 1000];
    // let mut x_on_d = d.create_from_ref(x.as_slice());

    // let mut y = vec![Rectangle::default(); 100];
    // let mut y_on_d = d.create_from_ref(y.as_slice());

    // d.set_from_ref(&mut x_on_d, &[1u32; 1000]);
    // d.set_from_ref(&mut x_on_d, &[5u32; 1000]);
    // println!("{:?}", d.get(&x_on_d).await?);
    // d.set_from_ref(&mut y_on_d, &[Rectangle::default(); 100]);
    // println!("{:?}", d.get(&y_on_d).await?);
    // println!("{:?}", futures::try_join!(d.get(&x_on_d), d.get(&y_on_d))?);

    // // let mut iterations = d.create_from_ref((1..=100).collect::<Vec<u32>>().as_slice());
    // let mut iterations = d.create_from_ref(y.as_slice());
    // println!("{:?}", d.get(&iterations).await?);
    // unsafe {
    //     let collatz = d.compile(
    //         ParamBuilder::new().param(Mutability::Mut).build(),
    //         std::fs::File::open("examples/collatz.spv").unwrap(),
    //         "main",
    //     )?;
    //     d.call(
    //         &collatz,
    //         (100, 1, 1),
    //         ArgBuilder::new().arg(&iterations).build(),
    //     );
    //     d.call(
    //         &collatz,
    //         (100, 1, 1),
    //         ArgBuilder::new().arg(&iterations).build(),
    //     );
    //     d.call(
    //         &collatz,
    //         (100, 1, 1),
    //         ArgBuilder::new().arg(&iterations).build(),
    //     );
    // }
    // println!("{:?}", d.get(&iterations).await?);

    // replace(d);

    // take()?.lock().unwrap().queue.submit(&[]);

    // let mut x = vec![0.0f32; 1000].as_device_boxed()?;

    // x.set(vec![0.0f32; 1000]);
    // println!("{:?}", x.get().await?);

    let mut x: DeviceBox<[Rectangle]> = vec![Rectangle::default(); 128].as_device_boxed()?;

    // let c = unsafe {
    //     compile::<Vec<u8>, SpirvCompile, _, GlobalCache>(
    //         std::fs::read("examples/collatz.spv").unwrap(),
    //     )?
    // };
    // let c = unsafe {
    //     compile::<String, GlslCompile, _, GlobalCache>(
    //         std::fs::read_to_string("examples/collatz.comp").unwrap(),
    //     )?
    // };
    let c = unsafe {
        compile::<String, GlslCompile, _, GlobalCache>(String::from(
            r#"
#version 450
layout(local_size_x = 1) in;

struct Rectangle {
    uint x;
    uint y;
    uint w;
    uint h;
};

layout(set = 0, binding = 0) buffer PrimeIndices {
    Rectangle[] indices;
}; // this is used as both input and output for convenience

// The Collatz Conjecture states that for any integer n:
// If n is even, n = n/2
// If n is odd, n = 3n+1
// And repeat this process for each new n, you will always eventually reach 1.
// Though the conjecture has not been proven, no counterexample has ever been found.
// This function returns how many times this recurrence needs to be applied to reach 1.
uint collatz_iterations(uint n) {
    uint i = 0;
    while(n != 1) {
        if (mod(n, 2) == 0) {
            n = n / 2;
        }
        else {
            n = (3 * n) + 1;
        }
        i++;
    }
    return i;
}

void main() {
    uint index = gl_GlobalInvocationID.x;
    // indices[index] = collatz_iterations(indices[index].w);
    indices[index].x = 10 * 10;
}
            "#,
        ))?
    };
    unsafe {
        spawn(128).launch(call!(c, &mut x));
    }

    println!("{:?}", x.get().await?);

    Ok(())
}

fn main() {
    futures::executor::block_on(do_some_stuff()).expect("failed to do stuff on GPU");
}
