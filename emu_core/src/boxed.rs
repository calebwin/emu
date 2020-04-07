//! Functions for working with `DeviceBox<T>` and the device pool

use std::borrow::Borrow;
use std::iter::FromIterator;

use crate::device::*;
use crate::error::*;
use crate::pool::*;

use zerocopy::*;

// what follows is a bunch of convenience functions for constructing DeviceBox<T>

impl<T: ?Sized> DeviceBox<T> {
    //
    // FUNCTIONS TO CREATE CONST BOXES
    //

    /// Create a constant `DeviceBox<T>` while consuming the given `T`
    pub fn new<U: IntoDeviceBoxed<T>>(obj: U) -> Result<Self, NoDeviceError> {
        obj.into_device_boxed()
    }

    /// Create a constant `DeviceBox<T>` from a reference to `T`
    pub fn from_ref<U: AsDeviceBoxed<T> + ?Sized>(obj: &U) -> Result<Self, NoDeviceError> {
        obj.as_device_boxed()
    }

    /// Create a constant `DeviceBox<T>` where `T` has the given number of bytes
    pub fn with_size(size: usize) -> Result<Self, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_with_size(size))
    }

    //
    // FUNCTIONS TO CREATE MUTABLE BOXES
    //

    /// Create a mutable `DeviceBox<T>` while consuming the given `T`
    pub fn new_mut<U: IntoDeviceBoxed<T>>(obj: U) -> Result<Self, NoDeviceError> {
        obj.into_device_boxed_mut()
    }

    /// Create a mutable `DeviceBox<T>` from a reference to `T`
    pub fn from_ref_mut<U: AsDeviceBoxed<T> + ?Sized>(obj: &U) -> Result<Self, NoDeviceError> {
        obj.as_device_boxed_mut()
    }

    /// Create a mutable `DeviceBox<T>` where `T` has the given number of bytes
    pub fn with_size_mut(size: usize) -> Result<Self, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_with_size_mut(size))
    }
}

/// A trait for creating a `DeviceBox<T>` by consuming an object `T`
///
/// It is implemented for all `T` that is sized as well as iterators over `T` (for iterators, we just collect everything before uploading to a `DeviceBox<T>`) where `T` can be safely serialized.
/// To ensure you can safely serialize your data, you should use `#[derive(AsBytes)]`
/// from [`zerocopy`](https://docs.rs/zerocopy/).
pub trait IntoDeviceBoxed<T: ?Sized> {
    fn into_device_boxed(self) -> Result<DeviceBox<T>, NoDeviceError>;
    fn into_device_boxed_mut(self) -> Result<DeviceBox<T>, NoDeviceError>;
}

impl<T: AsBytes> IntoDeviceBoxed<T> for T {
    fn into_device_boxed(self) -> Result<DeviceBox<T>, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_from_ref(&self))
    }

    fn into_device_boxed_mut(self) -> Result<DeviceBox<T>, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_from_ref_mut(&self))
    }
}

impl<T: AsBytes, U: Iterator<Item = T>> IntoDeviceBoxed<[T]> for U {
    fn into_device_boxed(self) -> Result<DeviceBox<[T]>, NoDeviceError> {
        Ok(take()?
            .lock()
            .unwrap()
            .create_from_ref(&*self.collect::<Box<[T]>>()))
    }

    fn into_device_boxed_mut(self) -> Result<DeviceBox<[T]>, NoDeviceError> {
        Ok(take()?
            .lock()
            .unwrap()
            .create_from_ref_mut(&*self.collect::<Box<[T]>>()))
    }
}

impl<T: AsBytes> FromIterator<T> for DeviceBox<[T]> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().into_device_boxed().unwrap() // TODO maybe in the future we should make this return a mutable - not const - DeviceBox
    }
}

/// A trait for creating a `DeviceBox<T>` from a reference to an object `T`
///
/// It is implemented for all `T` (even unsized) where `T` can be safely serialized.
/// To ensure you can safely serialize your data, you should use `#[derive(AsBytes)]`
/// from [`zerocopy`](https://docs.rs/zerocopy/).
pub trait AsDeviceBoxed<T: ?Sized> {
    fn as_device_boxed(&self) -> Result<DeviceBox<T>, NoDeviceError>;
    fn as_device_boxed_mut(&self) -> Result<DeviceBox<T>, NoDeviceError>;
}

impl<T: AsBytes + ?Sized, U: Borrow<T>> AsDeviceBoxed<T> for U {
    fn as_device_boxed(&self) -> Result<DeviceBox<T>, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_from_ref(self.borrow()))
    }

    fn as_device_boxed_mut(&self) -> Result<DeviceBox<T>, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_from_ref_mut(self.borrow()))
    }
}

// now that we can easily construct DeviceBox<T>, we provide functions for reading/writing

impl<T: AsBytes + ?Sized> DeviceBox<T> {
    /// Uploads the given data `T` to self (a `DeviceBox<T>`)
    pub fn set<U: AsRef<T>>(&mut self, obj: U) -> Result<(), NoDeviceError> {
        Ok(take()?.lock().unwrap().set_from_ref(self, obj.as_ref()))
    }
}

impl<T: FromBytes + Copy> DeviceBox<[T]> {
    /// Downloads from self (a `DeviceBox<[T]>`) to a `Box<[T]>`
    ///
    /// For now, we only support getting simple slices but in the future we may support more complex nested slices.
    /// Also, to use this `T` must be safe to deserialize. You can ensure this by using `#[derive(FromBytes)]`
    /// from [`zerocopy`](https://https://docs.rs/zerocopy/). Lastly, this is asynchronous - note that _all_ other methods of `DeviceBox` are
    /// non-blocking and can be used in asynchronous code.
    pub async fn get(&self) -> Result<Box<[T]>, GetError> {
        take()
            .map_err(|_| GetError::NoDevice)?
            .lock()
            .unwrap()
            .get(self)
            .await
            .map_err(|_| GetError::Completion)
    }
}
