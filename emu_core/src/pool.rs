use std::cell::{Cell, RefCell, RefMut};
use std::collections::HashSet;
use std::fmt;
use std::ops::DerefMut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use core::future::Future;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;

use derive_more::{From, Into};

use crate::device::*;
use crate::error::*;

// these are some things a pool should let you do
// - provide a custom pool once or just use a default pool
// - mutate the wgpu internals in the pool (by blocking till an &mut Device is available) }
// - use high-level functions like set/get/compile/launch (that block to get &mut Device) } both of these use a thread-local selected index

#[derive(From, Into, Clone)]
pub struct DeviceInfo {
    info: wgpu::AdapterInfo,
}

impl fmt::Debug for DeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ name: {:?}, vendor_id: {:?}, device_id: {:?}, device_type: {:?} }}",
            self.get_name(),
            self.get_vendor_id(),
            self.get_device_id(),
            self.get_device_type()
        )
    }
}

#[derive(Debug)]
pub enum DeviceType {
    Cpu,
    IntegratedGpu,
    DiscreteGpu,
    VirtualGpu,
    Other,
}

impl DeviceInfo {
    fn get_name(&self) -> String {
        self.info.name.clone()
    }

    fn get_vendor_id(&self) -> usize {
        self.info.vendor
    }

    fn get_device_id(&self) -> usize {
        self.info.device
    }

    fn get_device_type(&self) -> DeviceType {
        match &self.info.device_type {
            Cpu => DeviceType::Cpu,
            IntegratedGpu => DeviceType::IntegratedGpu,
            DiscreteGpu => DeviceType::DiscreteGpu,
            VirtualGpu => DeviceType::VirtualGpu,
            _ => DeviceType::Other,
        }
    }
}

#[derive(From, Into)]
pub struct DevicePoolMember {
    device: Mutex<Device>, // this is a Mutex because we want to be able to mutate this from different threads
    device_info: Option<DeviceInfo>, // this is an Option because we might not know info about the device
}

// a convenience helper function for getting a device
// the device returned by this function would then be used to form the default device pool
fn any_device() -> Result<DevicePoolMember, NoDeviceError> {
    let adapter = wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
        },
        wgpu::BackendBit::PRIMARY,
    )
    .ok_or(NoDeviceError)?; // we use a ? because if request fails there is no (or None) device to be used

    // we then get a device and a queue
    // you might think we need to support multiple queues per device
    // but Metal, DX, and WebGPU standard itself move the handling of different queues to underlying implmenetation
    // so we only need one queue
    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits::default(),
    });

    // return the constructed device
    // there is no cost to returning device info so we just do it
    // it might be useful for making an iterator over devices
    Ok(DevicePoolMember {
        device: Mutex::new(Device {
            device: device,
            queue: queue,
        }),
        device_info: Some(DeviceInfo {
            info: adapter.get_info(),
        }),
    })
}

// global state
// used for device pool stuff
lazy_static! {
    static ref CUSTOM_DEVICE_POOL: Mutex<Option<Vec<DevicePoolMember>>> = Mutex::new(None);
    static ref DEVICE_POOL: Option<Vec<DevicePoolMember>> = {
        if CUSTOM_DEVICE_POOL.lock().unwrap().is_some() {
            Some(CUSTOM_DEVICE_POOL.lock().unwrap().take().unwrap()) // we can unwrap since we know it is Some
        } else if let Ok(device) = any_device() {
            Some(vec![device]) // in the future, we will actually add all devices and not just 1
        } else {
            Some(vec![])
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

// this sets device pool
// it can only be successfully called just once
// it should also be followed by a select to select a device
//
// this function is useful if you want to work with the wgpu internals with take
// you can call pool at the start of your application to initalize all the devices you plan on using
// you can then do graphics stuff with take and compute stuff with high-level get/set/compile/launch
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

// this function is the connection between the high-level pool-based interface and the low-level wgpu internals
// with take, you can mutate the wgpu internals "hidden" behind the pool
// as a result, you can have full control over each device in the pool if you want or use high-level get/set/compile/launch
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

#[derive(Clone, Debug)]
pub struct DevicePoolMemberInfo {
    pub index: usize,
    pub info: Option<DeviceInfo>,
}

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
