//! Functions for spawning threads and launching compiled `DeviceFnMut`s

use crate::device::*;
use crate::error::*;
use crate::pool::*;

use std::sync::Arc;

/// Constructs a [`Spawner`](struct.Spawner.html) with the given number of threads spawned
///
/// Each `spawn(n)` will spawn a new dimension of threads of size `n`. In other words, for each thread already spawned, `n` threads are spawned.
/// If more than 3 dimensions are added, all dimensions are collapsed into the "x" dimension where the size
/// of the "x" dimension is now the product of all sizes of dimensions so far. Until 3 dimensions of threads have been spawned,
/// threads will be spawned on dimensions "x", "y", and "z" in that order.
///
/// This can be used in conjunction with `Spawner` as follows. `spawn` returns a `Spawner` which lets you `.spawn` more dimensions of threads.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // don't forget - this should always be the first thing you call
/// // don't assume there is a device available without calling this
/// // if there isn't one, you will recieve a runtime panic
/// futures::executor::block_on(assert_device_pool_initialized());
///
/// // move data to a device
/// let data = vec![1.0; 1 << 20];
/// let mut data_on_gpu: DeviceBox<[f32]> = data.as_device_boxed_mut()?;
///
/// // compile a kernel
/// let kernel: GlslKernel = GlslKernel::new()
///     .param_mut::<[f32], _>("float[] data")
///     .param::<f32, _>("float scalar")
///     .with_kernel_code(r#"
/// uint index = (1 << 10) * gl_GlobalInvocationID.x + gl_GlobalInvocationID.y;
/// data[index] = data[index] * scalar;
///     "#);
/// let c = compile::<GlslKernel, GlslKernelCompile, _, GlobalCache>(kernel)?.finish()?;
///
/// // run the compiled kernel
/// unsafe {
///     spawn(1 << 10)
///         .spawn(1 << 10)
///         .launch(call!(c, &mut data_on_gpu, &DeviceBox::new(10.0f32)?))?;
/// }
///
/// // download data from the GPU and check the result
/// assert_eq!(futures::executor::block_on(data_on_gpu.get())?, vec![10.0; 1 << 20].into_boxed_slice());
/// # Ok(())
/// # }
/// ```
pub fn spawn(num_threads: u32) -> Spawner {
    Spawner {
        work_space_dim: vec![num_threads],
    }
}

/// A "builder" for a space of threads that are to be spawned
///
/// See [`spawn`](fn.spawn.html) for more details.
pub struct Spawner {
    work_space_dim: Vec<u32>,
}

impl Spawner {
    /// Adds a new dimension to the space of threads with size determined by the given number of threads
    pub fn spawn(mut self, num_threads: u32) -> Self {
        self.work_space_dim.push(num_threads);
        self
    }

    fn get_work_space_dim(&self) -> Result<(u32, u32, u32), LaunchError> {
        match self.work_space_dim.len() {
            0 => Ok((0, 0, 0)),
            1 => Ok((self.work_space_dim[0], 1, 1)),
            2 => Ok((self.work_space_dim[0], self.work_space_dim[1], 1)),
            3 => Ok((
                self.work_space_dim[0],
                self.work_space_dim[1],
                self.work_space_dim[2],
            )),
            _ => Ok((self.work_space_dim.iter().product(), 1, 1)),
        }
    }

    /// Launches given `DeviceFnMut` with given arguments on the space of threads built so far
    ///
    /// You can provide the arguments using [`ArgsBuilder`](../device/struct.ArgsBuilder.html) or using the `call` macro.
    pub unsafe fn launch<'a>(
        &self,
        device_fn_mut_with_args: (Arc<DeviceFnMut>, DeviceFnMutArgs<'a>),
    ) -> Result<(), LaunchError> {
        take()
            .map_err(|_| LaunchError::NoDevice)?
            .lock()
            .unwrap()
            .call(
                &device_fn_mut_with_args.0,
                self.get_work_space_dim()?,
                device_fn_mut_with_args.1,
            )
    }
}

/// A macro which evaluates to something that can be passed into [`launch`](spawn/struct.Spawner.html#method.launch)
///
/// For example usage, see [`spawn`](spawn/fn.spawn.html)
#[macro_export]
macro_rules! call {
	($fn_mut:expr $( ,$fn_mut_arg:expr )*) => (
		{
            (
            	$fn_mut,
            	ArgsBuilder::new()$(
                	.arg($fn_mut_arg)
            	)*.build()
            )
        }
	)
}
