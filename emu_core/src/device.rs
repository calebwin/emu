//! The lowest-level, core functionality for controlling a GPU device
//!
//! You can use this module in isolation from the rest of Emu for the most
//! fine control over the internals of the library. The most important things
//! in this module that are likely of interest to
//! you are [`Device`](struct.Device.html), [`DeviceBox<T>`](stuct.DeviceBox.html)
//! , and [`DeviceFnMut`](struct.DeviceFnMut.html).

use crate::error::*;

// some std stuff...
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt;

use std::hash::{Hash, Hasher};

use std::marker::PhantomData;

// zerocopy is used for serializing and deserializing data to/from devices
use zerocopy::*;

// futures is for returning anything read from device as a future

// derive_more allows us to easily derive interop with wgpu stuff
use derive_more::{From, Into};

/// Contains information about a device
#[derive(From, Into, Clone, PartialEq)]
pub struct DeviceInfo(pub wgpu::AdapterInfo);

impl fmt::Debug for DeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ name: {:?}, vendor_id: {:?}, device_id: {:?}, device_type: {:?} }}",
            self.name(),
            self.vendor_id(),
            self.device_id(),
            self.device_type()
        )
    }
}

impl DeviceInfo {
    /// The name of the device (e.g. - "Intel(R) UHD Graphics 620 (Kabylake GT2)"")
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    /// The vendor ID (e.g. - 32902)
    pub fn vendor_id(&self) -> usize {
        self.0.vendor
    }

    /// The devie ID (e.g. - 22807)
    pub fn device_id(&self) -> usize {
        self.0.device
    }

    /// The device type (e.g. - Cpu)
    pub fn device_type(&self) -> DeviceType {
        match &self.0.device_type {
            wgpu::DeviceType::Cpu => DeviceType::Cpu,
            wgpu::DeviceType::IntegratedGpu => DeviceType::IntegratedGpu,
            wgpu::DeviceType::DiscreteGpu => DeviceType::DiscreteGpu,
            wgpu::DeviceType::VirtualGpu => DeviceType::VirtualGpu,
            _ => DeviceType::Other,
        }
    }
}

/// Represents a type of device
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum DeviceType {
    Cpu,
    IntegratedGpu,
    DiscreteGpu,
    VirtualGpu,
    Other,
}

/// Represents a single device
///
/// Since its fields are public, you can easily construct and mutate a `Device`'s
/// WebGPU internals.
pub struct Device {
    /// The WebGPU device wrapped by this data structure
    pub device: wgpu::Device,
    /// The queue this device exposes to submit work to
    pub queue: wgpu::Queue, // in the future. when multiple queues are supported, overlapping compute, mem ops on the same device will be possible
    /// Information about the device
    ///
    /// This is optional so that you don't _need_ information to construct a `Device` yourself.
    pub info: Option<DeviceInfo>,
}

impl Device {
    // TODO don't use a new staging buffer; instead, pull staging buffers from a pool

    /// Gets all detected devices
    ///
    /// This is asynchronous because it may take a long time for all the devices
    /// that are detected to actually be available. However, you shouldn't
    /// actually use this. Unless you manually construct a pool of devices, a
    /// default device pool is implicitly created. So you should instead do one of the following.
    /// - If you are developing a library, select a device from the pool with [`select`](../pool/fn.select.html)/[`take`](../pool/fn.take.html)
    /// - If you are developing an application, construct a pool with [`pool`](../pool/fn.pool.html) or use the default pool
    ///
    /// If you are using the default pool, don't forget to call [`assert_device_pool_initialized`](../pool/fn.assert_device_pool_initialized.html).
    pub async fn all() -> Vec<Self> {
        let adapters = wgpu::Adapter::enumerate(wgpu::BackendBit::PRIMARY);

        futures::future::join_all(adapters.into_iter().map(|adapter| {
            async move {
                let info = adapter.get_info().clone();
                // we then get a device and a queue
                // you might think we need to support multiple queues per device
                // but Metal, DX, and WebGPU standard itself move the handling of different queues to underlying implmenetation
                // so we only need one queue
                //
                // searching for devices does not need to be async
                // it takes barely any time and should really only be the first thing Emu is used to do
                // also, it's a one-time thing
                let (device, queue) = adapter
                    .request_device(&wgpu::DeviceDescriptor {
                        extensions: wgpu::Extensions {
                            anisotropic_filtering: false,
                        },
                        limits: wgpu::Limits::default(),
                    })
                    .await;

                // return the constructed device
                // there is no cost to returning device info so we just do it
                // it might be useful for making an iterator over devices
                Device {
                    device: device,
                    queue: queue,
                    info: Some(DeviceInfo(info)),
                }
            }
        }))
        .await
    }

    /// Creates a constant `DeviceBox<T>` with size of given number of bytes
    pub fn create_with_size<T>(&mut self, size: usize) -> DeviceBox<T>
    where
        T: ?Sized,
    {
        self.create_with_size_as::<T>(size, Mutability::Const)
    }

    /// Creates a mutable `DeviceBox<T>` with size of given number of bytes
    pub fn create_with_size_mut<T>(&mut self, size: usize) -> DeviceBox<T>
    where
        T: ?Sized,
    {
        self.create_with_size_as::<T>(size, Mutability::Mut)
    }

    /// Creates a constant `DeviceBox<T>` from a reference to `T`
    pub fn create_from_ref<T>(&mut self, host_obj: &T) -> DeviceBox<T>
    where
        T: AsBytes + ?Sized,
    {
        self.create_from_ref_as::<T>(host_obj, Mutability::Const)
    }

    /// Creates a mutable `DeviceBox<T>` from a reference to `T`
    pub fn create_from_ref_mut<T>(&mut self, host_obj: &T) -> DeviceBox<T>
    where
        T: AsBytes + ?Sized,
    {
        self.create_from_ref_as::<T>(host_obj, Mutability::Mut)
    }

    fn create_with_size_as<T>(&mut self, size: usize, mutability: Mutability) -> DeviceBox<T>
    where
        T: ?Sized,
    {
        let staging_buffer = {
            let mapped = self.device.create_buffer_mapped(&wgpu::BufferDescriptor {
                label: None,
                size: size as u64,
                usage: wgpu::BufferUsage::MAP_READ
                    | wgpu::BufferUsage::COPY_DST
                    | wgpu::BufferUsage::COPY_SRC,
            });
            mapped.finish()
        };
        let storage_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: size as u64, // casting usize to u64 is safe since usize is subtype of u64
            usage: match mutability {
                Mutability::Mut => wgpu::BufferUsage::STORAGE,
                Mutability::Const => wgpu::BufferUsage::STORAGE_READ,
            } | wgpu::BufferUsage::COPY_DST
                | wgpu::BufferUsage::COPY_SRC,
        });
        DeviceBox {
            staging_buffer,
            storage_buffer,
            size: size as u64,
            phantom: PhantomData,
        }
    }

    fn create_from_ref_as<T>(&mut self, host_obj: &T, mutability: Mutability) -> DeviceBox<T>
    where
        T: AsBytes + ?Sized,
    {
        // serialize the data into bytes
        // these bytes can later be deserialized back into T
        let host_obj_bytes = host_obj.as_bytes();

        // create a staging buffer with host_obj copied over
        // then create an empty storage buffer of appropriate size
        let staging_buffer = self.device.create_buffer_with_data(
            host_obj_bytes,
            wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::COPY_SRC,
        );
        let storage_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: host_obj_bytes.len() as u64, // casting usize to u64 is safe since usize is subtype of u64
            usage: match mutability {
                Mutability::Mut => wgpu::BufferUsage::STORAGE,
                Mutability::Const => wgpu::BufferUsage::STORAGE_READ,
            } | wgpu::BufferUsage::COPY_DST
                | wgpu::BufferUsage::COPY_SRC,
        });

        // now copy over the staging buffer to the storage buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(
            &staging_buffer,
            0,
            &storage_buffer,
            0,
            host_obj_bytes.len() as u64,
        );
        self.queue.submit(&[encoder.finish()]);

        // return the final DeviceBox
        // note that we keep both the storage buffer and the staging buffer
        // we will re-use the staging buffer for reads (but not for writes, for writes we just create a new staging buffer)
        DeviceBox {
            staging_buffer,
            storage_buffer,
            size: host_obj_bytes.len() as u64,
            phantom: PhantomData,
        }
    }

    // TODO say what is blocking and what isn't in the comments
    /// Uploads data from the given reference to `T` to the given `DeviceBox<T>` that lives on this (meaning `self`) device
    pub fn set_from_ref<T>(&mut self, device_obj: &mut DeviceBox<T>, host_obj: &T)
    where
        T: AsBytes + ?Sized,
    {
        // serialize the data into bytes
        // these bytes can later be deserialized back into T
        let host_obj_bytes = host_obj.as_bytes();

        // create a staging buffer with host_obj copied over
        // set this staging buffer as the new staging buffer for the device box
        let staging_buffer = self.device.create_buffer_with_data(
            host_obj_bytes,
            wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::COPY_SRC,
        );
        device_obj.staging_buffer = staging_buffer;

        // now copy over the staging buffer to the storage buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(
            &device_obj.staging_buffer,
            0,
            &device_obj.storage_buffer,
            0,
            device_obj.size,
        );
        self.queue.submit(&[encoder.finish()]);
    }

    /// Downloads data from the given `DeviceBox<T>` asyncronously and returns a boxed slice of `T`
    pub async fn get<T>(&mut self, device_obj: &DeviceBox<[T]>) -> Result<Box<[T]>, CompletionError>
    where
        T: FromBytes + Copy, // implicitly, T is also Sized which is necessary for us to be able to deserialize
    {
        // first, we copy over data from the storage buffer to the staging buffer
        // the staging buffer is host visible so we can then work with it more easily
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(
            &device_obj.storage_buffer,
            0,
            &device_obj.staging_buffer,
            0,
            device_obj.size,
        );
        self.queue.submit(&[encoder.finish()]);

        // now we can return a future for data read from staging buffer
        // this does a kind of complicated deserialization procedure
        // basically it does staging_buffer -> [T]
        let result = device_obj.staging_buffer.map_read(0u64, device_obj.size); // this gets a GpuFuture<Result<BufferReadMapping, ()>>

        // poll the device
        // TODO this should not be blocking (since this is async) we need to find some way to poll a
        self.device.poll(wgpu::Maintain::Wait);

        result
            .await
            .map(|buffer_read_mapping| {
                buffer_read_mapping
                    .as_slice() // this gets the &[u8] held by BufferReadMapping
                    .chunks_exact(std::mem::size_of::<T>()) // this creates an iterator over each item of size = size_of(T)
                    .map(|item| {
                        let layout_verified: LayoutVerified<_, T> =
                            LayoutVerified::new(item).unwrap(); // TODO ensure this unwrap makes sense
                        *layout_verified
                    }) // this deserializes each size_of(T) item
                    .collect() // this collects it all into a [T]
            }) // this transforms the inner BufferReadMapping
            .map_err(|_error| CompletionError)
    }

    /// Runs the given `DeviceFnMut` on a 3-dimensional space of threads to launch (with given dimensions) and arguments to pass to the launched kernel
    pub unsafe fn call<'a>(
        &mut self,
        device_fn_mut: &DeviceFnMut,
        work_space_dim: (u32, u32, u32),
        args: DeviceFnMutArgs<'a>,
    ) -> Result<(), LaunchError> {
        // begin the encoder of command to send to device
        // then, generate command to do computation
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut bind_groups = vec![];
        for (set_num, (bind_group, _offsets)) in &args.bind_groups {
            bind_groups.push(
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None, // TODO maybe in all these label fields, we should actually use a label
                    layout: &device_fn_mut.bind_group_layouts[&set_num],
                    bindings: bind_group
                        .values()
                        .map(|binding| binding.clone())
                        .collect::<Vec<wgpu::Binding<'a>>>()
                        .as_slice(),
                    // TODO ensure the above clone is okay, it should be only cloning the underlying reference to a buffer and not cloning the entire buffer
                }),
            );
        }
        {
            // our compute pass will have 2 parts
            // 1. the pipeline, using the device_fn_mut
            // 2. the bind group, using the args
            let mut cpass = encoder.begin_compute_pass();
            // first we set the pipeline
            cpass.set_pipeline(&device_fn_mut.compute_pipeline);
            // then we apply the bind groups, binding all the arguments

            for (set_num, (_bind_group, offsets)) in args.bind_groups {
                // bind_group = collection of bindings
                cpass.set_bind_group(set_num, &bind_groups[set_num as usize], &*offsets);
            }
            // finally we dispatch the compute pass with given work space dims
            // note that these work space dims would essentially be the same things that are between triple brackets in CUDA
            cpass.dispatch(work_space_dim.0, work_space_dim.1, work_space_dim.2);
        }

        // finally, send the command
        self.queue.submit(&[encoder.finish()]);

        Ok(())
    }

    /// Compiles a `DeviceFnMut` using the given parameters, entry point name, and SPIR-V program
    ///
    /// The entry point is where in the SPIR-V program the compiled kernel should be entered upon execution.
    /// The entry point's name is anything implementing `Into<String>` including `&str` and `String` while
    /// the program itself is anything `Borrow<[u32]>` including `Vec<u32>` and `&[u32]`.
    pub fn compile<T: Into<String>, P: Borrow<[u32]>>(
        &self,
        program_params: DeviceFnMutParams,
        program_entry: T,
        program: P,
    ) -> Result<DeviceFnMut, CompileError> {
        // TODO return a Result with error for compile error
        // TODO use proper error types
        let mut bind_group_layouts: HashMap<u32, wgpu::BindGroupLayout> = HashMap::new();
        for (set_num, set) in program_params.bind_group_layouts {
            bind_group_layouts.insert(
                set_num,
                self.device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: None,
                        bindings: set
                            .values()
                            .map(|binding_layout| binding_layout.clone())
                            .collect::<Vec<wgpu::BindGroupLayoutEntry>>()
                            .as_slice(),
                    }),
            );
        }
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: bind_group_layouts
                    .values()
                    .collect::<Vec<&wgpu::BindGroupLayout>>()
                    .as_slice(),
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                layout: &pipeline_layout,
                compute_stage: wgpu::ProgrammableStageDescriptor {
                    // TODO use a Result for this function instead of unwrap_or hack
                    module: &self.device.create_shader_module(program.borrow()), // this is where we compile the bytecode program itself
                    entry_point: program_entry.into().as_str(), // this will probably be something like "main" or the name of the main function
                },
            });

        Ok(DeviceFnMut {
            bind_group_layouts,
            compute_pipeline: pipeline,
        })
    }
}

/// A "box" type for storing stuff on a device
///
/// It is generic over a type `T` so that we can safely transmute data from the
/// GPU (`DeviceBox<T>`) to and from data from the CPU (`T`).
pub struct DeviceBox<T>
where
    T: ?Sized,
{
    pub(crate) staging_buffer: wgpu::Buffer,
    pub(crate) storage_buffer: wgpu::Buffer,
    pub(crate) size: u64, // inv: size being constant and equal to sizes of staging, storage buffers respectively
    pub(crate) phantom: PhantomData<T>,
}

impl<T: ?Sized> From<(wgpu::Buffer, wgpu::Buffer, u64)> for DeviceBox<T> {
    fn from(wgpu_stuff: (wgpu::Buffer, wgpu::Buffer, u64)) -> Self {
        Self {
            staging_buffer: wgpu_stuff.0,
            storage_buffer: wgpu_stuff.1,
            size: wgpu_stuff.2,
            phantom: PhantomData,
        }
    }
}

impl<T: ?Sized> Into<(wgpu::Buffer, wgpu::Buffer, u64)> for DeviceBox<T> {
    fn into(self) -> (wgpu::Buffer, wgpu::Buffer, u64) {
        (self.staging_buffer, self.storage_buffer, self.size)
    }
}

/// Represents a compiled kernel that can then be launched across spawned threads with [`Device::call`](struct.Device.html#method.call) or [`spawn`](../spawn/fn.spawn.html)
///
/// While compiling a `DeviceFnMut` is expensive, running a `DeviceFnMut` with varying work space dimensions or arguments incurs no significant extra compilation.
#[derive(From, Into)]
pub struct DeviceFnMut {
    // we really just need 2 things to define a function
    // 1. the layout of input buffers to be bound (think of this as declaring the parameters of the function)
    // 2. the shader module and its entry point (this is like the actual body of the function)
    // both of these can be used to produce the following
    pub(crate) bind_group_layouts: HashMap<u32, wgpu::BindGroupLayout>, // u32 = set number
    pub(crate) compute_pipeline: wgpu::ComputePipeline, // inv: has PipelineLayout consistent with above BindGroupLayout's
}

/// Describes the parameters that can be passed to a `DeviceFnMut`
///
/// This is cheap to construct and something you can safely clone multiple times.
#[derive(From, Into, Clone)]
pub struct DeviceFnMutParams {
    bind_group_layouts: HashMap<u32, HashMap<u32, wgpu::BindGroupLayoutEntry>>, // (u32, u32) = (set number, binding number)
}

impl Hash for DeviceFnMutParams {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for bind_group_layout in self.bind_group_layouts.values() {
            for entry in bind_group_layout.values() {
                entry.hash(state);
            }
        }
    }
}

impl DeviceFnMutParams {
    /// Constructs a set of parameters where each parameter is mutable
    pub fn new(num_params: usize) -> Self {
        let mut bind_group_layouts = HashMap::new();
        let mut binding_layouts = HashMap::new();
        for _ in 0..num_params {
            let new_binding_layout_idx = binding_layouts.len() as u32;
            binding_layouts.insert(
                new_binding_layout_idx,
                wgpu::BindGroupLayoutEntry {
                    binding: new_binding_layout_idx,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::StorageBuffer {
                        dynamic: false, // we usually don't need dynamic for compute so we default to 0, of course if you need it you can provide your own Device...Params
                        readonly: false, // for now, this is just always mutable bc I don't know if readonly is more performant
                    },
                },
            );
        }
        bind_group_layouts.insert(0, binding_layouts);

        Self { bind_group_layouts }
    }
}

/// Says whether or not something is mutable
#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug)]
pub enum Mutability {
    Mut,
    Const,
}

/// Helps with building a `DeviceFnMutParams`
#[derive(Clone)]
pub struct ParamsBuilder {
    binding_layouts: HashMap<u32, wgpu::BindGroupLayoutEntry>,
}

impl Hash for ParamsBuilder {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for binding_layout in self.binding_layouts.values() {
            binding_layout.hash(state);
        }
    }
}

impl ParamsBuilder {
    /// Starts the building process with no parameters
    pub fn new() -> Self {
        Self {
            binding_layouts: HashMap::new(),
        }
    }

    /// Adds on a parameter with given mutability
    pub fn param(mut self, mutability: Mutability) -> Self {
        let new_binding_layout_idx = self.binding_layouts.len() as u32;
        self.binding_layouts.insert(
            new_binding_layout_idx,
            wgpu::BindGroupLayoutEntry {
                binding: new_binding_layout_idx,
                visibility: wgpu::ShaderStage::COMPUTE,
                ty: wgpu::BindingType::StorageBuffer {
                    dynamic: false, // we usually don't need dynamic for compute so we default to 0, of course if you need it you can provide your own Device...Params
                    readonly: mutability == Mutability::Const,
                },
            },
        );

        self
    }

    /// Builds a `DeviceFnMutParams`
    pub fn build(self) -> DeviceFnMutParams {
        let mut bind_group_layouts = HashMap::new();
        bind_group_layouts.insert(0, self.binding_layouts); // again, we usually don't need more than 1 set, so we default to just 1

        DeviceFnMutParams { bind_group_layouts }
    }
}

/// Holds the actual arguments to be passed into a [`DeviceFnMut`](struct.DeviceFnMut.html)
//
// it should be cheap to construct and consist mainly of a bunch of wgpu::Binding's where a Binding represents an argument
// it will also contain some extra information needed to construct a BindGroup
#[derive(From, Into)]
pub struct DeviceFnMutArgs<'a> {
    // this contains information for each bind group (marked by a u32 set number)
    // each bind group has a set of bindings (mapped from u32 binding number) and a set of offsets
    // note that set number and binding number are things you might see if you're looking at GLSL code
    //
    // in practice, there will usually just be 1 bind group and offsets = &[]
    // but we accept a full HashMap supporting multiple bind groups to facilitate nice interop with wgpu
    //
    // also, note the lifetime
    // a wgpu::Binding owns a reference to data (like a wgpu::Buffer owned by a DeviceBox)
    // we must ensure that DeviceFnMutArgs doesn't outlive the Buffer (and maybe DeviceBox) that it refers to
    //
    // and technically there can't be more than 4 sets (I think) but we still just use a HashMap for convenience
    bind_groups: HashMap<
        u32,
        (
            HashMap<u32, wgpu::Binding<'a>>,
            Vec</*wgpu::BufferAddress*/ u32>,
        ),
    >, // (u32, u32) = (set number, binding number)
}

/// Helps with building a `DeviceFnMutArgs`
pub struct ArgBuilder<'a> {
    bindings: HashMap<u32, wgpu::Binding<'a>>,
}

impl<'a> ArgBuilder<'a> {
    /// Creates a new builder with no arguments
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Declare a new arguments by passing in a `DeviceBox`
    pub fn arg<T: ?Sized>(mut self, device_obj: &'a DeviceBox<T>) -> Self {
        let new_binding_idx = self.bindings.len() as u32;
        self.bindings.insert(
            new_binding_idx,
            wgpu::Binding {
                binding: new_binding_idx,
                resource: wgpu::BindingResource::Buffer {
                    buffer: &device_obj.storage_buffer,
                    range: 0..device_obj.size,
                },
            },
        );

        self
    }

    /// Builds the final `DeviceFnMutArgs`
    pub fn build(self) -> DeviceFnMutArgs<'a> {
        let mut bind_groups = HashMap::with_capacity(4);
        bind_groups.insert(0, (self.bindings, vec![])); // again, we usually don't need more than 1 set, so we default to just 1

        DeviceFnMutArgs { bind_groups }
    }
}
