//! Various error types

use std::error::Error;
use std::fmt;

use derive_more::Display;

// TOOD maybe there is a better approach to errors...

/// An error for when there is no device to complete a certain operation
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

/// An error when a device is there but not available for use
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

/// An error for compilation failures
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

/// An error for failure to complete data movement or computation
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

/// An error that occurs when you attempt to initialize an already initialized pool of devices
pub struct PoolAlreadyInitializedError;

impl Error for PoolAlreadyInitializedError {}

impl fmt::Debug for PoolAlreadyInitializedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "pool of devices is already initialized")
    }
}

impl fmt::Display for PoolAlreadyInitializedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "pool of devices is already initialized")
    }
}

/// An error in getting data stored in a `DeviceBox`
#[derive(Debug, Display)]
pub enum GetError {
    Completion,
    NoDevice,
}

impl Error for GetError {}

/// An error for capturing compilation fails or no device present
#[derive(Debug, Display)]
pub enum CompileOrNoDeviceError {
    Compile,
    NoDevice,
}

impl Error for CompileOrNoDeviceError {}

/// A runtime error that occurs on the device
pub struct RuntimeError;

impl Error for RuntimeError {}

impl fmt::Debug for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "runtime error occurred")
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "runtime error occurred")
    }
}

/// An error in launching kernels
#[derive(Debug, Display)]
pub enum LaunchError {
    NoDevice,
    Runtime,
}

impl Error for LaunchError {}
