use std::iter::FromIterator;

use crate::device::*;
use crate::error::*;
use crate::pool::*;

use zerocopy::*;

// what follows is a bunch of convenience functions for constructing DeviceBox<T>

impl<T: ?Sized> DeviceBox<T> {
    pub fn new<U: IntoDeviceBoxed<T>>(obj: U) -> Result<Self, NoDeviceError> {
        obj.into_device_boxed()
    }

    pub fn from_ref<U: AsDeviceBoxed<T> + ?Sized>(obj: &U) -> Result<Self, NoDeviceError> {
        obj.as_device_boxed()
    }

    pub fn with_size(size: usize) -> Result<Self, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_with_size(size))
    }
}

pub trait IntoDeviceBoxed<T: ?Sized> {
    fn into_device_boxed(self) -> Result<DeviceBox<T>, NoDeviceError>;
}

impl<T: AsBytes> IntoDeviceBoxed<T> for T {
    fn into_device_boxed(self) -> Result<DeviceBox<T>, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_from_ref(&self))
    }
}

impl<T: AsBytes, U: Iterator<Item = T>> IntoDeviceBoxed<[T]> for U {
    fn into_device_boxed(self) -> Result<DeviceBox<[T]>, NoDeviceError> {
        Ok(take()?
            .lock()
            .unwrap()
            .create_from_ref(&*self.collect::<Box<[T]>>()))
    }
}

impl<T: AsBytes> FromIterator<T> for DeviceBox<[T]> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        iter.into_iter().into_device_boxed().unwrap()
    }
}

pub trait AsDeviceBoxed<T: ?Sized> {
    fn as_device_boxed(&self) -> Result<DeviceBox<T>, NoDeviceError>;
}

impl<T: AsBytes + ?Sized, U: AsRef<T>> AsDeviceBoxed<T> for U {
    fn as_device_boxed(&self) -> Result<DeviceBox<T>, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_from_ref(self.as_ref()))
    }
}

// now that we can easily construct DeviceBox<T>, we provide functions for reading/writing

impl<T: AsBytes + ?Sized> DeviceBox<T> {
    pub fn set<U: AsRef<T>>(&mut self, obj: U) -> Result<(), NoDeviceError> {
        Ok(take()?.lock().unwrap().set_from_ref(self, obj.as_ref()))
    }
}

impl<T: FromBytes + Copy> DeviceBox<[T]> {
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
