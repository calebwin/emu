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
/// from [`zerocopy`](https://docs.rs/zerocopy/). If you just want to see some examples of how to create a `DeviceBox` from types for which `IntoDeviceBoxed` is already implemented,
/// then just go to the [docs for `DeviceBox`](../device/struct.DeviceBox.html).
///
/// Now, you can implement this for your own collection if you would like a way
/// for your collection data structure to exist on the GPU.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
/// #[repr(C)]
/// #[derive(AsBytes, FromBytes, Copy, Clone, Default, Debug, PartialEq)]
/// struct Molecule {
///     position: f64,
///     velocities: f64,
///     forces: f64,
/// }
///
/// // aside: you're more likely to be implementing these traits for _general-purpose_ collections
/// // than something domain-specific like this
/// #[derive(Default)]
/// struct Molecules {
///     num_molecules: usize,
///     positions: Vec<f64>,
///     velocities: Vec<f64>,
///     forces: Vec<f64>,
/// }
///
/// impl Molecules {
///     fn zero(num_molecules: usize) -> Self {
///         Self {
///             num_molecules,
///             positions: vec![0.0; num_molecules],
///             velocities: vec![0.0; num_molecules],
///             forces: vec![0.0; num_molecules],
///         }
///     }
/// }
///
/// impl IntoDeviceBoxed<[Molecule]> for Molecules {
///     fn into_device_boxed(self) -> Result<DeviceBox<[Molecule]>, NoDeviceError> {
///         Ok((0..self.num_molecules).map(|idx| Molecule {
///             position: self.positions[idx],
///             velocities: self.velocities[idx],
///             forces: self.forces[idx],
///         }).into_device_boxed()?)
///     }
///
///     fn into_device_boxed_mut(self) -> Result<DeviceBox<[Molecule]>, NoDeviceError> {
///         Ok((0..self.num_molecules).map(|idx| Molecule {
///             position: self.positions[idx],
///             velocities: self.velocities[idx],
///             forces: self.forces[idx],
///         }).into_device_boxed_mut()?)
///     }
/// }
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     futures::executor::block_on(assert_device_pool_initialized());
///     let molecules = Molecules::zero(4096);
///     let molecule_list_on_gpu = molecules.into_device_boxed_mut()?;
///     assert_eq!(futures::executor::block_on(molecule_list_on_gpu.get())?,
///         vec![Molecule::default(); 4096].into_boxed_slice());
///     Ok(())
/// }
/// ```
pub trait IntoDeviceBoxed<T: ?Sized> {
    fn into_device_boxed(self) -> Result<DeviceBox<T>, NoDeviceError>;
    fn into_device_boxed_mut(self) -> Result<DeviceBox<T>, NoDeviceError>;
}

impl<T: AsBytes> IntoDeviceBoxed<T> for T {
    fn into_device_boxed(self) -> Result<DeviceBox<T>, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_from(&self))
    }

    fn into_device_boxed_mut(self) -> Result<DeviceBox<T>, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_from_mut(&self))
    }
}

impl<T: AsBytes, U: Iterator<Item = T>> IntoDeviceBoxed<[T]> for U {
    fn into_device_boxed(self) -> Result<DeviceBox<[T]>, NoDeviceError> {
        Ok(take()?
            .lock()
            .unwrap()
            .create_from(&*self.collect::<Box<[T]>>()))
    }

    fn into_device_boxed_mut(self) -> Result<DeviceBox<[T]>, NoDeviceError> {
        Ok(take()?
            .lock()
            .unwrap()
            .create_from_mut(&*self.collect::<Box<[T]>>()))
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
/// from [`zerocopy`](https://docs.rs/zerocopy/). If you just want to see some examples of how to create a `DeviceBox` from types for which `AsDeviceBoxed` is already implemented,
/// then just go to the [docs for `DeviceBox`](../device/struct.DeviceBox.html).
///
/// You can implement this trait for your own collection if you would like to have
/// your collection somehow sends its encapsulated data over to a `DeviceBox` on the GPU.
/// ```
/// use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
///
/// // for some reason, we want to store Molecules as an array-of-structures on the GPU
/// // so we define this type for each element of the array
/// #[repr(C)]
/// #[derive(AsBytes, FromBytes, Copy, Clone, Default, Debug, PartialEq)]
/// struct Molecule {
///     position: f64,
///     velocities: f64,
///     forces: f64,
/// }
///
/// // this is the collection we would like to be able to move to the GPU easily
/// #[derive(Default)]
/// struct Molecules {
///     num_molecules: usize,
///     positions: Vec<f64>,
///     velocities: Vec<f64>,
///     forces: Vec<f64>,
/// }
///
/// impl Molecules {
///     fn zero(num_molecules: usize) -> Self {
///         Self {
///             num_molecules,
///             positions: vec![0.0; num_molecules],
///             velocities: vec![0.0; num_molecules],
///             forces: vec![0.0; num_molecules],
///         }
///     }
/// }
///
/// impl AsDeviceBoxed<[Molecule]> for Molecules {
///     fn as_device_boxed(&self) -> Result<DeviceBox<[Molecule]>, NoDeviceError> {
///         Ok((0..self.num_molecules).map(|idx| Molecule {
///             position: self.positions[idx],
///             velocities: self.velocities[idx],
///             forces: self.forces[idx],
///         }).collect::<Vec<Molecule>>().as_device_boxed()?)
///     }
///
///     fn as_device_boxed_mut(&self) -> Result<DeviceBox<[Molecule]>, NoDeviceError> {
///         Ok((0..self.num_molecules).map(|idx| Molecule {
///             position: self.positions[idx],
///             velocities: self.velocities[idx],
///             forces: self.forces[idx],
///         }).collect::<Vec<Molecule>>().as_device_boxed_mut()?)
///     }
/// }
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     futures::executor::block_on(assert_device_pool_initialized());
///     let molecules = Molecules::zero(4096);
///     let molecule_list_on_gpu: DeviceBox<[Molecule]> = molecules.as_device_boxed_mut()?;
///     assert_eq!(futures::executor::block_on(molecule_list_on_gpu.get())?,
///         vec![Molecule::default(); 4096].into_boxed_slice());
///     Ok(())
/// }
/// ```
pub trait AsDeviceBoxed<T: ?Sized> {
    fn as_device_boxed(&self) -> Result<DeviceBox<T>, NoDeviceError>;
    fn as_device_boxed_mut(&self) -> Result<DeviceBox<T>, NoDeviceError>;
}

impl<T: AsBytes + ?Sized, U: Borrow<T>> AsDeviceBoxed<T> for U {
    fn as_device_boxed(&self) -> Result<DeviceBox<T>, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_from(self.borrow()))
    }

    fn as_device_boxed_mut(&self) -> Result<DeviceBox<T>, NoDeviceError> {
        Ok(take()?.lock().unwrap().create_from_mut(self.borrow()))
    }
}

// now that we can easily construct DeviceBox<T>, we provide functions for reading/writing

impl<T: AsBytes + ?Sized> DeviceBox<T> {
    /// Uploads the given data `T` to self (a `DeviceBox<T>`)
    ///
    /// This function - as are most other functions in the Emu API - doesn't block.
    /// So the data transfer only occurs when the future returned by `get` is completed.
    /// `set` is pretty easy to use. You just pass in either an owned object or a reference and
    /// the object is uploaded to the GPU.
    ///
    /// Here's a quick example.
    /// ```
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # futures::executor::block_on(assert_device_pool_initialized());
    /// let mut data: DeviceBox<[f32]> = vec![0.5; 1024].as_device_boxed_mut()?;
    /// data.set(vec![1.0; 1024])?;
    /// # Ok(())
    /// # }
    /// ```
    /// It is expected that the object you pass in is of the same size (in bytes) as what was
    /// already stored in the `DeviceBox`. For example, you should not upload a vector of different
    /// length than that of the slice already stored on the device.
    pub fn set<U: Borrow<T>>(&mut self, obj: U) -> Result<(), NoDeviceError> {
        Ok(take()?.lock().unwrap().set_from(self, obj.borrow()))
    }
}

impl<T: FromBytes + Copy> DeviceBox<[T]> {
    /// Downloads from self (a `DeviceBox<[T]>`) to a `Box<[T]>`
    ///
    /// This function is asynchronous. So you can either `.await` it in an asynchronous context or you
    /// can use an executor to immediately evaluate it.
    /// ```
    /// use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    ///
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     // first, we ensure that the global pool of devices has been initialized
    ///     futures::executor::block_on(assert_device_pool_initialized());
    ///     // then we create some data, move it to the GPU, and mutate it
    ///     let mut data: DeviceBox<[f32]> = vec![0.5; 1024].as_device_boxed_mut()?;
    ///     data.set(vec![1.0; 1024])?;
    ///     // finally, we download the data from the GPU
    ///     assert_eq!(futures::executor::block_on(data.get())?, vec![1.0; 1024].into_boxed_slice());
    ///     Ok(())
    /// }
    /// ```
    ///
    /// For now, we only support getting simple slices but in the future we may support more complex nested slices.
    /// Also, to use this `T` must be safe to deserialize. You can ensure this by using `#[derive(FromBytes)]`
    /// from [`zerocopy`](https://https://docs.rs/zerocopy/).
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
