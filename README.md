[![Discord Chat](https://img.shields.io/discord/308323056592486420.svg)](https://discord.gg/WqhrRQ)

<p align="center">
<!-- <img width="250px" src="https://i.imgur.com/kTap42K.png"/> -->
    <img width="250px" src="https://i.imgur.com/CZEkdK1.png"/>
</p>

Emu is a GPGPU library with a focus on portability, modularity, and performance. It's a CUDA-esque compute-specific abstraction over [WebGPU](https://github.com/gfx-rs/wgpu-rs) providing specific functionality to make WebGPU feel more like CUDA. Here's a quick run-down of highlight features...

- **Emu can run anywhere** - Emu uses WebGPU to support DirectX, Metal, Vulkan (and also OpenGL and browser eventually) as compile targets. This allows Emu to run on pretty much any user interface including desktop, mobile, and browser. By moving heavy computations to the user's device, you can reduce system latency and improve privacy.
- **Emu makes compute easier** - Emu makes WebGPU feel like CUDA. It does this by providing...
    - `DeviceBox<T>` as a wrapper for data that lives on the GPU (thereby ensuring type-safe data movement)
    - `DevicePool` as a no-config auto-managed pool of devices (similar to CUDA)
    - `trait Cache` - a no-setup-required LRU cache of JITed compute kernels.
- **Emu is transparent** - Emu is a fully transparent abstraction. This means, at any point, you can decide to remove the abstraction and work directly with WebGPU constructs with zero overhead. For example, if you want to mix Emu with WebGPU-based graphics, you can do that with zero overhead. You can also swap out the JIT compiler artifact cache with your own cache, manage the device pool if you wish, and define your own compile-to-SPIR-V compiler that interops with Emu.

Here's a quick example of Emu. You can find more in `emu_core/examples`.

```rust
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
    w: i32,
    h: i32,
}

async fn do_some_stuff() -> Result<(), Box<dyn std::error::Error>> {
    let mut x: DeviceBox<[Rectangle]> = vec![Default::default(); 128].as_device_boxed()?;
    
    let c = unsafe {
        compile::<String, GlslCompile, _, GlobalCache>(String::from(
            r#"
#version 450
layout(local_size_x = 1) in;

struct Rectangle {
    uint x;
    uint y;
    int w;
    int h;
};

layout(set = 0, binding = 0) buffer Rectangles {
    Rectangle[] rectangles;
}; // this is used as both input and output for convenience

Rectangle flip(Rectangle r) {
    r.x = r.x + r.w;
    r.y = r.y + r.h;
    r.w *= -1;
    r.h *= -1;
    return r;
}

void main() {
    uint index = gl_GlobalInvocationID.x;
    rectangles[index] = flip(rectangles[index]);
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
```

For now, you can get started with using Emu with the following.
```toml
[dependencies]
emu_core = {
    git = "https://github.com/calebwin/emu/tree/master/emu_core.git",
    rev = "265d2a5fb9292e2644ae4431f2982523a8d27a0f"
}
```

If you have any questions, please [ask in the Discord](https://discord.gg/WqhrRQ).
