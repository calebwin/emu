// some std stuff...
use std::collections::HashMap;
use std::fmt;
use std::io::{Read, Seek};
use std::marker::PhantomData;

// zerocopy is used for serializing and deserializing data to/from devices
use zerocopy::*;

// futures is for returning anything read from device as a future
use futures::future::FutureExt;

// derive_more allows us to easily derive interop with wgpu stuff
use derive_more::{From, Into};

use crate::error::*;

#[derive(From, Into, Clone)]
pub struct DeviceInfo(pub wgpu::AdapterInfo);

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

impl DeviceInfo {
    pub fn get_name(&self) -> String {
        self.0.name.clone()
    }

    pub fn get_vendor_id(&self) -> usize {
        self.0.vendor
    }

    pub fn get_device_id(&self) -> usize {
        self.0.device
    }

    pub fn get_device_type(&self) -> DeviceType {
        match &self.0.device_type {
            Cpu => DeviceType::Cpu,
            IntegratedGpu => DeviceType::IntegratedGpu,
            DiscreteGpu => DeviceType::DiscreteGpu,
            VirtualGpu => DeviceType::VirtualGpu,
            _ => DeviceType::Other,
        }
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

// this is a handle to a device
// it represents a single device
// and so only one instance of it should exist for each device
//
// by making the fields public, Device is interoperable with wgpu
// you can construct it from wgpu and mutate its wgpu internals
pub struct Device {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue, // in the future. when multiple queues are supported, overlapping compute, mem ops on the same device will be possible
    pub info: Option<DeviceInfo>,
}

impl Device {
    // this shouldn't really ever be called
    // instead select a device from the pool with take/replace (if you are a library)
    // construct a pool with pool or use the default (if you are an application)
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

    pub fn create_with_size<T>(&mut self, size: usize) -> DeviceBox<T>
    where
        T: ?Sized,
    {
        let staging_buffer = {
            let mapped = self.device.create_buffer_mapped(
                size,
                wgpu::BufferUsage::MAP_READ
                    | wgpu::BufferUsage::COPY_DST
                    | wgpu::BufferUsage::COPY_SRC,
            );
            mapped.finish()
        };
        let storage_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            size: size as u64, // casting usize to u64 is safe since usize is subtype of u64
            usage: wgpu::BufferUsage::STORAGE
                | wgpu::BufferUsage::COPY_DST
                | wgpu::BufferUsage::COPY_SRC,
        });
        DeviceBox {
            staging_buffer,
            storage_buffer,
            size: size as u64,
            phantom: PhantomData,
        }
    }

    pub fn create_from_ref<T>(&mut self, host_obj: &T) -> DeviceBox<T>
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
            size: host_obj_bytes.len() as u64, // casting usize to u64 is safe since usize is subtype of u64
            usage: wgpu::BufferUsage::STORAGE
                | wgpu::BufferUsage::COPY_DST
                | wgpu::BufferUsage::COPY_SRC,
        });

        // now copy over the staging buffer to the storage buffer
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
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
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        encoder.copy_buffer_to_buffer(
            &device_obj.staging_buffer,
            0,
            &device_obj.storage_buffer,
            0,
            device_obj.size,
        );
        self.queue.submit(&[encoder.finish()]);
    }

    pub async fn get<T>(&mut self, device_obj: &DeviceBox<[T]>) -> Result<Box<[T]>, CompletionError>
    where
        T: FromBytes + Copy, // implicitly, T is also Sized which is necessary for us to be able to deserialize
    {
        // first, we copy over data from the storage buffer to the staging buffer
        // the staging buffer is host visible so we can then work with it more easily
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
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
        let result = device_obj
            .staging_buffer
            .map_read(0u64, device_obj.size); // this gets a GpuFuture<Result<BufferReadMapping, ()>>

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
            .map_err(|error| CompletionError)
    }

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
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        let mut bind_groups = vec![];
        for (set_num, (bind_group, offsets)) in &args.bind_groups {
            bind_groups.push(
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
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

            for (set_num, (bind_group, offsets)) in args.bind_groups {
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

    pub fn compile<T: Into<String>, U: Read + Seek>(
        &self,
        program_params: DeviceFnMutParams,
        program: U,
        program_entry: T,
    ) -> Result<DeviceFnMut, CompileError> {
        // TODO return a Result with error for compile error
        // TODO use proper error types
        let mut bind_group_layouts: HashMap<u32, wgpu::BindGroupLayout> = HashMap::new();
        for (set_num, set) in program_params.bind_group_layouts {
            bind_group_layouts.insert(
                set_num,
                self.device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    module: &self.device.create_shader_module(
                        wgpu::read_spirv(program).unwrap_or(vec![]).as_slice(),
                    ), // this is where we compile the bytecode program itself
                    entry_point: program_entry.into().as_str(), // this will probably be something like "main" or the name of the main function
                },
            });

        Ok(DeviceFnMut {
            bind_group_layouts,
            compute_pipeline: pipeline,
        })
    }
}

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

// a DeviceFnMut represents a compiled "kernel" that can then be invoked with call_mut
// compiling a DeviceFnMut is expensive as it involves compilation
// but running a DeviceFnMut with arbitrary work space dimensions or arguments incurs no significant extra compilation
#[derive(From, Into)]
pub struct DeviceFnMut {
    // we really just need 2 things to define a function
    // 1. the layout of input buffers to be bound (think of this as declaring the parameters of the function)
    // 2. the shader module and its entry point (this is like the actual body of the function)
    // both of these can be used to produce the following
    pub(crate) bind_group_layouts: HashMap<u32, wgpu::BindGroupLayout>, // u32 = set number
    pub(crate) compute_pipeline: wgpu::ComputePipeline, // inv: has PipelineLayout consistent with above BindGroupLayout's
}

// a DeviceFnMutParams describes the parameters to a DeviceFnMut
// it should be cheap to construct
// also, it is a relatively low-level construct
// there might be higher-level ways of defining parameters (e.g. - implicitly through a language that compiles to program + program_params)
#[derive(From, Into)]
pub struct DeviceFnMutParams {
    bind_group_layouts: HashMap<u32, HashMap<u32, wgpu::BindGroupLayoutEntry>>, // (u32, u32) = (set number, binding number)
}

impl DeviceFnMutParams {
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

#[derive(Eq, PartialEq, Hash, Copy, Clone)]
pub enum Mutability {
    Mut,
    Const,
}

pub struct ParamBuilder {
    binding_layouts: HashMap<u32, wgpu::BindGroupLayoutEntry>,
}

impl ParamBuilder {
    pub fn new() -> Self {
        Self {
            binding_layouts: HashMap::new(),
        }
    }

    // right now, all we need to know is if the parameter is mutable
    // in the future this param method might except more
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

    pub fn build(self) -> DeviceFnMutParams {
        let mut bind_group_layouts = HashMap::new();
        bind_group_layouts.insert(0, self.binding_layouts); // again, we usually don't need more than 1 set, so we default to just 1

        DeviceFnMutParams { bind_group_layouts }
    }
}

// a DeviceFnMutArgs holds the actual arguments to be passed into a DeviceFnMut
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
    bind_groups: HashMap<
        u32,
        (
            HashMap<u32, wgpu::Binding<'a>>,
            Vec</*wgpu::BufferAddress*/ u32>,
        ),
    >, // (u32, u32) = (set number, binding number)
}

pub struct ArgBuilder<'a> {
    bindings: HashMap<u32, wgpu::Binding<'a>>,
}

impl<'a> ArgBuilder<'a> {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

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

    pub fn build(self) -> DeviceFnMutArgs<'a> {
        let mut bind_groups = HashMap::new();
        bind_groups.insert(0, (self.bindings, vec![])); // again, we usually don't need more than 1 set, so we default to just 1

        DeviceFnMutArgs { bind_groups }
    }
}
