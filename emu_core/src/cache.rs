//! Infrastructure for caching kernels that are already JIT compiled

use crate::device::*;

use lazy_static::lazy_static;
use std::collections::hash_map::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

// in the future, we may not need to use a cache because caching is done automatically by wgpu

/// A trait to implement to create your own cache
///
/// The [`compile`](../compile/fn.compile.html) function is generic over a `Cache` implementation.
/// So you could, for example, implement `Cache` for a disk cache or in-memory cache customized for your needs.
pub trait Cache {
    // key is derived from the source language
    // source language what is compiled to SPIR-V and then to machine code (stored in a DeviceFnMut)
    fn contains(key: u64) -> bool;
    fn get(key: u64) -> Arc<DeviceFnMut>;
    fn insert(key: u64, device_fn_mut: Arc<DeviceFnMut>);
}

lazy_static! {
    // RwLock and Arc are expensive, yes, but it's probably worth it since the performance penalty is dwarfed by compile time
    static ref GLOBAL_KERNEL_CACHE: RwLock<HashMap<u64, Arc<DeviceFnMut>>> = RwLock::new(HashMap::new());
    static ref GLOBAL_KERNEL_CACHE_LRU: RwLock<VecDeque<u64>> = RwLock::new(VecDeque::new()); // this "lru list" keeps track of which keys are most recently used
    static ref GLOBAL_KERNEL_CACHE_CAPACITY: RwLock<usize> = RwLock::new(0);
}

fn maybe_initialize_global_kernel_cache() {
    if *GLOBAL_KERNEL_CACHE_CAPACITY.read().unwrap() == 0 {
        *GLOBAL_KERNEL_CACHE_CAPACITY.write().unwrap() = 32;
    }
}

/// A simple in-memory LRU cache for up to 32 JIT-ed kernels
pub struct GlobalCache;

impl GlobalCache {
    /// Reserves space for the given number of additional kernels
    pub fn reserve(additional: usize) {
        *GLOBAL_KERNEL_CACHE_CAPACITY.write().unwrap() += additional;
    }
}

impl Cache for GlobalCache {
    fn contains(key: u64) -> bool {
        maybe_initialize_global_kernel_cache();
        GLOBAL_KERNEL_CACHE.read().unwrap().contains_key(&key)
    }

    fn get(key: u64) -> Arc<DeviceFnMut> {
        maybe_initialize_global_kernel_cache();

        // move key to front of lru list
        let key_location_in_lru = GLOBAL_KERNEL_CACHE_LRU
            .read()
            .unwrap()
            .iter()
            .position(|&x| x == key)
            .unwrap();
        GLOBAL_KERNEL_CACHE_LRU
            .write()
            .unwrap()
            .swap(0, key_location_in_lru);

        // return DeviceFnMut with key from cache
        GLOBAL_KERNEL_CACHE
            .read()
            .unwrap()
            .get(&key)
            .map(|v| Arc::clone(v))
            .unwrap()
    }

    fn insert(key: u64, device_fn_mut: Arc<DeviceFnMut>) {
        maybe_initialize_global_kernel_cache();

        // check if our cache is out of space
        if GLOBAL_KERNEL_CACHE.read().unwrap().len()
            == *GLOBAL_KERNEL_CACHE_CAPACITY.read().unwrap()
        {
            // remove the least recently used
            let lru_location_in_cache = (*GLOBAL_KERNEL_CACHE_LRU.read().unwrap())
                .back()
                .unwrap()
                .clone();
            GLOBAL_KERNEL_CACHE
                .write()
                .unwrap()
                .remove(&lru_location_in_cache);
            // we're out of space so we need to remove the least recently used and insert this as most recently used
            GLOBAL_KERNEL_CACHE_LRU.write().unwrap().pop_back();
            GLOBAL_KERNEL_CACHE_LRU.write().unwrap().push_front(key);
        } else {
            // if not we just add this newly inserted key into the lru list
            GLOBAL_KERNEL_CACHE_LRU.write().unwrap().push_front(key);
        }

        // finally, insert into cache
        GLOBAL_KERNEL_CACHE
            .write()
            .unwrap()
            .insert(key, device_fn_mut);
    }
}
