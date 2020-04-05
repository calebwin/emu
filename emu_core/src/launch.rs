use crate::device::*;
use crate::error::*;
use crate::pool::*;

use std::sync::{Arc, RwLock};

pub fn spawn(num_threads: u32) -> Spawner {
    Spawner {
        work_space_dim: vec![num_threads],
    }
}

pub struct Spawner {
    work_space_dim: Vec<u32>,
}

impl Spawner {
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
