use std::error::Error;
use std::fmt;

use derive_more::Display;

pub struct NoDeviceError;

impl Error for NoDeviceError {}

impl fmt::Debug for NoDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "no device could be found")
    }
}

impl fmt::Display for NoDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "no device could be found")
    }
}

pub struct UnavailableDeviceError;

impl Error for UnavailableDeviceError {}

impl fmt::Debug for UnavailableDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "currently selected device is already taken and unavailable"
        )
    }
}

impl fmt::Display for UnavailableDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "currently selected device is already taken and unavailable"
        )
    }
}

pub struct CompileError;

impl Error for CompileError {}

impl fmt::Debug for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to compile")
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to compile")
    }
}

pub struct CompletionError;

impl Error for CompletionError {}

impl fmt::Debug for CompletionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "failed to successfully complete memory creation/transfer/computation"
        )
    }
}

impl fmt::Display for CompletionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "failed to successfully complete memory creation/transfer/computation"
        )
    }
}

pub struct PoolAlreadyInitializedError;

impl Error for PoolAlreadyInitializedError {}

impl fmt::Debug for PoolAlreadyInitializedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "pool of devices is already intialized")
    }
}

impl fmt::Display for PoolAlreadyInitializedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "pool of devices is already intialized")
    }
}

// TODO figure out if an enum is the best approach for this
#[derive(Debug, Display)]
pub enum GetError {
    Completion,
    NoDevice,
}

impl Error for GetError {}

// TODO figure out if an enum is the best approach for this
#[derive(Debug, Display)]
pub enum CompileOrNoDeviceError {
    Compile,
    NoDevice,
}

impl Error for CompileOrNoDeviceError {}

pub struct TooManyThreadsError;

impl Error for TooManyThreadsError {}

impl fmt::Debug for TooManyThreadsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "threads can be spawned recursively up to 3 levels of recursion"
        )
    }
}

impl fmt::Display for TooManyThreadsError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "threads can be spawned recursively up to 3 levels of recursion"
        )
    }
}

pub struct RuntimeError;

impl Error for RuntimeError {}

impl fmt::Debug for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "runtime error ocurred")
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "runtime error ocurred")
    }
}

// TODO figure out if an enum is the best approach for this
#[derive(Debug, Display)]
pub enum LaunchError {
    TooManyThreads,
    NoDevice,
    Runtime,
}

impl Error for LaunchError {}
