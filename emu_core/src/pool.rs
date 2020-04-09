//! Tools for managing the device pool either implicitly or explicitly
//!
//! If you plan on not really delving into the WebGPU internals of Emu and just want to work from the other side of the abstraction,
//! the only important thing here is [`assert_device_pool_initialized`](fn.assert_device_pool_initialized.html).

use derive_more::{From, Into};
use std::cell::RefCell;
use std::sync::Mutex;

use crate::device::*;
use crate::error::*;

/// Represents a member of the device pool
///
/// This holds both a mutex to a `Device` and information about the device. You must create instances of `DevicePoolMember` to construct your own custom device pool using
/// [`pool`](fn.pool.html).
#[derive(From, Into)]
pub struct DevicePoolMember {
    pub device: Mutex<Device>, // this is a Mutex because we want to be able to mutate this from different threads
    pub device_info: Option<DeviceInfo>, // we duplicate data here because we don't want to have to lock the Mutex just to see info
}

// global state
// used for device pool stuff
lazy_static! {
    static ref CUSTOM_DEVICE_POOL: Mutex<Option<Vec<DevicePoolMember>>> = Mutex::new(None);
    static ref DEVICE_POOL: Option<Vec<DevicePoolMember>> = {
        if CUSTOM_DEVICE_POOL.lock().unwrap().is_some() {
            Some(CUSTOM_DEVICE_POOL.lock().unwrap().take().unwrap()) // we can unwrap since we know it is Some
        } else {
            panic!("pool of devices has not been initialized with `assert_device_pool_initialized`")
        }
    };
}

// thread local state
// used for selecting device for each thread
thread_local! {
    // this is the index of the device being used by the current thread in the above device pool Vec
    // it defaults to None (and not 0 or anything else) because it isn't known if there even is an available device
    // it shouldn't be used until the device pool is initialized
    static DEVICE_IDX: RefCell<Option<usize>> = RefCell::new(None); // the Option here is None when it isn't initialized or DEVICE_POOL is empty
}

// this should be called every time before you want to use DEVICE_POOL
fn maybe_initialize_device_pool() {
    lazy_static::initialize(&DEVICE_POOL);
}

// this should be called every time before you want to use DEVICE_IDX
fn maybe_initialize_device_idx() {
    if DEVICE_POOL.is_some() && DEVICE_IDX.with(|idx| idx.borrow().is_none()) {
        if DEVICE_POOL.as_ref().unwrap().len() > 0 {
            // we can only set device index if pool is Some and has length
            DEVICE_IDX.with(|idx| *idx.borrow_mut() = Some(0));
        }
    }
}

/// Sets the device pool to the given `Vec` of devices
///
/// You can use `pool` to set up a custom pool of devices. It can only be successfully called just once. Calling `pool` multiple times will result in a panic at runtime.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*, std::sync::Mutex};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut device = futures::executor::block_on(Device::all()).remove(0);
/// pool(vec![DevicePoolMember {
///     device: Mutex::new(device),
///     device_info: None
/// }])?;
/// // technically, we don't need this assertion because we know the pool is initialized
/// futures::executor::block_on(assert_device_pool_initialized());
/// # futures::executor::block_on(assert_device_pool_initialized());
/// let pi: DeviceBox<f32> = DeviceBox::with_size(std::mem::size_of::<f32>())?;
/// # Ok(())
/// # }
/// ```
///
/// This function can be useful if you want to work with the WebGPU internals with [`take`](fn.take.html).
/// You can call `pool` at the start of your application to initialize all the devices you plan on using.
/// You can then do graphics stuff using `take` and all of [`wgpu-rs`](https://crates.io/crates/wgpu) and compute stuff with high-level `get`/`set`/`compile`/`spawn`.
pub fn pool(new_device_pool: Vec<DevicePoolMember>) -> Result<(), PoolAlreadyInitializedError> {
    if CUSTOM_DEVICE_POOL.lock().unwrap().is_some() {
        Err(PoolAlreadyInitializedError)
    } else {
        // we only initialize the custom device pool right now
        // the actual device pool will be initialized automatically when it is used
        *CUSTOM_DEVICE_POOL.lock().unwrap() = Some(new_device_pool);
        Ok(())
    }
}

/// Asserts that the device pool has been initialized
///
/// This must be the first thing you call before using Emu for anything. The only thing you might call before this is [`pool`](fn.pool.html) if you are manually setting the pool of devices.
/// You can call this as many times as you like. If no custom pool has be set with `pool`, this will go ahead and initialize all detected devices and add them to the pool.
///
/// This function is asynchronous so you must pass the future it returns to an executor like so.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*, std::sync::Mutex};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// futures::executor::block_on(assert_device_pool_initialized());
/// # futures::executor::block_on(assert_device_pool_initialized());
/// # futures::executor::block_on(assert_device_pool_initialized());
/// # Ok(())
/// # }
/// ```
///
/// So if you are an application, definitely call this before you use Emu do anything on a GPU device.
/// If you are a library, definitely make sure that you call this before every possible first time that you use Emu.
/// You don't have to call it before _every_ API call of course - just before every time when it's possible that this is the first time you are using Emu.
pub async fn assert_device_pool_initialized() {
    if CUSTOM_DEVICE_POOL.lock().unwrap().is_none() {
        let devices = Device::all().await;
        *CUSTOM_DEVICE_POOL.lock().unwrap() = Some(
            devices
                .into_iter()
                .map(|device| {
                    let info = device.info.clone();
                    DevicePoolMember {
                        device: Mutex::new(device),
                        device_info: info,
                    }
                })
                .collect::<Vec<DevicePoolMember>>(),
        );
    }
}

/// Takes the device currently selected out of the device pool and hands you a mutex for mutating the device's sate
///
/// This function is the link between the high-level pool-based interface and the low-level WebGPU internals.
/// With `take`, you can mutate the WebGPU internals "hidden" behind the device pool.
/// Consequently, you can have full control over each device in the pool if you want or use high-level `get`/`set`/`compile`/`spawn`.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*, std::sync::Mutex};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// futures::executor::block_on(assert_device_pool_initialized());
/// # futures::executor::block_on(assert_device_pool_initialized());
/// let mut d = take()?.lock()?;
/// let pi: DeviceBox<f32> = d.create_with_size(std::mem::size_of::<f32>());
/// # Ok(())
/// # }
/// ```
pub fn take<'a>() -> Result<&'a Mutex<Device>, NoDeviceError> {
    maybe_initialize_device_pool();
    maybe_initialize_device_idx();

    DEVICE_IDX.with(|idx| {
        if idx.borrow().is_none() {
            // inv: there are no devices in the device pool, since idx could not be initialized to Some
            Err(NoDeviceError)
        } else {
            Ok(&(DEVICE_POOL
                .as_ref()
                .unwrap()
                .get(idx.borrow().unwrap())
                .unwrap()
                .device))
        }
    })
}

/// Holds information about a member of the device pool
#[derive(Clone, Debug, PartialEq)]
pub struct DevicePoolMemberInfo {
    /// The index of the device in the pool
    pub index: usize,
    /// The actual information wrapped by this structure
    pub info: Option<DeviceInfo>,
}

/// Returns information about all devices in the pool
pub fn info_all() -> Vec<DevicePoolMemberInfo> {
    maybe_initialize_device_pool();
    maybe_initialize_device_idx();

    DEVICE_POOL
        .as_ref()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, device)| DevicePoolMemberInfo {
            index: i,
            info: device.device_info.clone(),
        })
        .collect()
}

/// Returns information about the currently selected device
pub fn info() -> Result<DevicePoolMemberInfo, NoDeviceError> {
    maybe_initialize_device_pool();
    maybe_initialize_device_idx();

    DEVICE_IDX.with(|idx| {
        if idx.borrow().is_none() {
            // inv: there are no devices in the device pool, since idx could not be initialized to Some
            Err(NoDeviceError)
        } else {
            Ok(DevicePoolMemberInfo {
                index: idx.borrow().unwrap(),
                info: DEVICE_POOL
                    .as_ref()
                    .unwrap()
                    .get(idx.borrow().unwrap())
                    .unwrap()
                    .device_info
                    .clone(),
            })
        }
    })
}

/// Selects a device from the pool using the given selector function
///
/// Emu uses thread-local storage to keep track of the selected device for each thread.
/// `select` lets you select a device for the thread it is called from.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*, std::sync::Mutex};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// futures::executor::block_on(assert_device_pool_initialized());
/// # futures::executor::block_on(assert_device_pool_initialized());
/// select(|idx, info| if let Some(info) = info {
///     info.name().to_ascii_lowercase().contains("intel")
/// } else {
///     false
/// })?;
/// let mut d = take()?.lock()?;
/// let pi: DeviceBox<f32> = d.create_with_size(std::mem::size_of::<f32>());
/// # Ok(())
/// # }
/// ```
pub fn select<F: FnMut(usize, Option<DeviceInfo>) -> bool>(
    mut selector: F,
) -> Result<(), NoDeviceError> {
    maybe_initialize_device_pool();
    maybe_initialize_device_idx();

    DEVICE_IDX.with(|idx| {
        if idx.borrow().is_none() {
            // inv: there are no devices in the device pool, since idx could not be initialized to Some
            Err(NoDeviceError)
        } else {
            *idx.borrow_mut() = Some(
                info_all()
                    .iter()
                    .position(|member_info| selector(member_info.index, member_info.info.clone()))
                    .ok_or(NoDeviceError)?,
            );

            Ok(())
        }
    })
}
