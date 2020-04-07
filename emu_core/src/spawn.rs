//! Functions for spawning threads and launching compiled `DeviceFnMut`s

use crate::device::*;
use crate::error::*;
use crate::pool::*;

use std::sync::Arc;

/// Constructs a [`Spawner`](struct.Spawner.html) with the given number of threads spawned
///
/// Each `spawn(n)` will spawn a new dimension of threads of size `n`. In other words, for each thread already spawned, `n` threads are spawned.
/// `spawn` will simply add on a new dimension to the compute shader thread space. If more than 3 dimensions are added, all dimensions are collapsed into the "x" dimension where the size
/// of the "x" dimension is now the product of all sizes of dimensions so far.
pub fn spawn(num_threads: u32) -> Spawner {
    Spawner {
        work_space_dim: vec![num_threads],
    }
}

/// A "builder" for a space of threads that are to be spawned
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

/// A macro which evaluates to something that can be passed into `launch`
#[macro_export]
macro_rules! call {
	($fn_mut:expr $( ,$fn_mut_arg:expr )*) => (
		{
            (
            	$fn_mut,
            	ArgBuilder::new()$(
                	.arg($fn_mut_arg)
            	)*.build()
            )
        }
	)
}
