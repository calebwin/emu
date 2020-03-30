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

// // thread local state
// // used for selecting device for each thread
// //
// // the big assumption here and elsewhere is that Adapter::enumerate() is invariant
// thread_local! {
//     static ADAPTER_IDX: RefCell<Option<usize>> = RefCell::new(None); // the Option here is None when it isn't initialized
// }

// fn maybe_initialize_adapter_idx() {
//     if ADAPTER_IDX.with(|idx| idx.borrow().is_none()) {
//         ADAPTER_IDX.with(|idx| *idx.borrow_mut() = Some(0));
//     }
// }

// pub async fn take() -> Device {
//     maybe_initialize_adapter_idx();

//     let adapter_idx = ADAPTER_IDX.with(|idx| idx.clone());
//     let adapter =
//         &wgpu::Adapter::enumerate(wgpu::BackendBit::PRIMARY)[adapter_idx.into_inner().unwrap()];
//     let (device, queue) = adapter
//         .request_device(&wgpu::DeviceDescriptor {
//             extensions: wgpu::Extensions {
//                 anisotropic_filtering: false,
//             },
//             limits: wgpu::Limits::default(),
//         })
//         .await;
//     Device {
//         device: device,
//         queue: queue,
//         info: Some(DeviceInfo(adapter.get_info())),
//     }
// }

// // each DeviceInfo in returned Vec has same index of corresponding Device in pool
// pub fn info_all() -> Vec<DeviceInfo> {
//     maybe_initialize_adapter_idx();

//     Adapter::enumerate().into_iter().map(|adapter| DeviceInfo(adapter.get_info())).collect::<Vec<DeviceInfo>>()
// }

// pub fn take() -> DeviceInfo {
//     maybe_initialize_device_idx();

//     let adapter_idx = ADAPTER_IDX.with(|idx| idx.clone());
//     let adapter =
//         &wgpu::Adapter::enumerate(wgpu::BackendBit::PRIMARY)[adapter_idx.into_inner().unwrap()];
//     DeviceInfo(adapter.get_info())
// }

// pub fn select<F: FnMut(usize, DeviceInfo) -> bool>(
//     mut selector: F,
// ) -> Result<(), NoDeviceError> {
//     for (i, device_info) in info_all().iter().enumerate() {
//         if selector()
//     }
// }

// these are some things a pool should let you do
// - provide a custom pool once or just use a default pool
// - mutate the wgpu internals in the pool (by blocking till an &mut Device is available) }
// - use high-level functions like set/get/compile/launch (that block to get &mut Device) } both of these use a thread-local selected index

#[derive(From, Into)]
pub struct DevicePoolMember {
    device: Mutex<Device>, // this is a Mutex because we want to be able to mutate this from different threads
    device_info: Option<DeviceInfo>, // we duplicate data here because we don't want to have to lock the Mutex just to see info
}

// global state
// used for device pool stuff
lazy_static! {
    static ref CUSTOM_DEVICE_POOL: Mutex<Option<Vec<DevicePoolMember>>> = Mutex::new(None);
    static ref DEVICE_POOL: Option<Vec<DevicePoolMember>> = {
        if CUSTOM_DEVICE_POOL.lock().unwrap().is_some() {
            Some(CUSTOM_DEVICE_POOL.lock().unwrap().take().unwrap()) // we can unwrap since we know it is Some
        } else {
            panic!("pool of devices has not been initialized with either pool or pool_init_default")
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

// this should always be the first thing you call
// that is - unless you use pool function - then you should call that first and then pool_init
// 
// so if you are an application, definitely call this before you use Emu do anything on a GPU device
// and if you are a library, definitely make sure that you call this before every possible first time that you use Emu
pub async fn assert_device_pool_initialized() {
    let devices = Device::all().await;
    pool(devices.into_iter().map(|device| {let info = device.info.clone();DevicePoolMember {
                device: Mutex::new(device),
                device_info: info
            }}).collect::<Vec<DevicePoolMember>>());
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
