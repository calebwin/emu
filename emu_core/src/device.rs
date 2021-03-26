//! The lowest-level, core functionality for controlling a GPU device
//!
//! You can use this module in isolation from the rest of Emu for the most
//! fine control over the internals of the library. The most important things
//! in this module that are likely of interest to
//! you are [`Device`](struct.Device.html), [`DeviceBox<T>`](stuct.DeviceBox.html)
//! , and [`DeviceFnMut`](struct.DeviceFnMut.html).

use crate::error::*;

// some std stuff...
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek};
use std::marker::PhantomData;
use std::{
    borrow::{Borrow, Cow},
    num::NonZeroU64,
};

use futures::TryFutureExt;
use wgpu::{util::DeviceExt, ComputePassDescriptor};
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
/// WebGPU internals. To get a `Device` from an existing device pool, you will want to use [`take`](../pool/fn.take.html).
///
/// One thing to remember is that each `Device` owns its data. So even though the device pool lets you create `DeviceBox`s on different devices,
/// you cannot use them together in the same kernel.
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
    /// If you are using the default pool, don't forget to call [`assert_device_pool_initialized`](../pool/fn.assert_device_pool_initialized.html) before doing anthing with a device.
    pub async fn all() -> Vec<Self> {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let adapters = instance.enumerate_adapters(wgpu::BackendBit::PRIMARY);

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
                    .request_device(
                        &wgpu::DeviceDescriptor {
                            label: None,
                            features: wgpu::Features::empty(),
                            limits: wgpu::Limits::default(),
                        },
                        None,
                    )
                    .await
                    .unwrap();

                // return the constructed device
                // there is no cost to returning device info so we just do it
                // it might be useful for making an iterator over devices

                println!("{:#?}", device.limits());

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
    ///
    /// ```
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut device = &mut futures::executor::block_on(Device::all())[0];
    /// let pi: DeviceBox<f32> = device.create_with_size(std::mem::size_of::<f32>());
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_with_size<T>(&mut self, size: usize) -> DeviceBox<T>
    where
        T: ?Sized,
    {
        self.create_with_size_as::<T>(size, Mutability::Const)
    }

    /// Creates a mutable `DeviceBox<T>` with size of given number of bytes
    ///
    /// ```
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut device = &mut futures::executor::block_on(Device::all())[0];
    /// let mut data: DeviceBox<[f32]> = device.create_with_size(std::mem::size_of::<f32>() * 2048);
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_with_size_mut<T>(&mut self, size: usize) -> DeviceBox<T>
    where
        T: ?Sized,
    {
        self.create_with_size_as::<T>(size, Mutability::Mut)
    }

    /// Creates a constant `DeviceBox<T>` from a borrow of `T`
    ///
    /// ```
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut device = &mut futures::executor::block_on(Device::all())[0];
    /// let pi: DeviceBox<f32> = device.create_from(&3.1415);
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_from<T, B: Borrow<T>>(&mut self, host_obj: B) -> DeviceBox<T>
    where
        T: AsBytes + ?Sized,
    {
        self.create_from_as::<T, B>(host_obj, Mutability::Const)
    }

    /// Creates a mutable `DeviceBox<T>` from a borrow of `T`
    ///
    /// ```
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut device = &mut futures::executor::block_on(Device::all())[0];
    /// let data = vec![0.0; 2048];
    /// let mut data_on_gpu: DeviceBox<[f32]> = device.create_from(data.as_slice());
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_from_mut<T, B: Borrow<T>>(&mut self, host_obj: B) -> DeviceBox<T>
    where
        T: AsBytes + ?Sized,
    {
        self.create_from_as::<T, B>(host_obj, Mutability::Mut)
    }

    fn create_with_size_as<T>(&mut self, size: usize, mutability: Mutability) -> DeviceBox<T>
    where
        T: ?Sized,
    {
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: size as u64,
            usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        let storage_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: size as u64, // casting usize to u64 is safe since usize is subtype of u64
            usage: match mutability {
                Mutability::Mut => wgpu::BufferUsage::STORAGE,
                Mutability::Const => wgpu::BufferUsage::STORAGE,
            } | wgpu::BufferUsage::COPY_DST
                | wgpu::BufferUsage::COPY_SRC,
            mapped_at_creation: false,
        });
        DeviceBox {
            staging_buffer,
            storage_buffer,
            size: size as u64,
            phantom: PhantomData,
            mutability: Some(mutability),
        }
    }

    fn create_from_as<T, B: Borrow<T>>(
        &mut self,
        host_obj: B,
        mutability: Mutability,
    ) -> DeviceBox<T>
    where
        T: AsBytes + ?Sized,
    {
        // serialize the data into bytes
        // these bytes can later be deserialized back into T
        let host_obj_bytes = host_obj.borrow().as_bytes();

        // create a staging buffer with host_obj copied over
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: host_obj_bytes.len() as u64,
            usage: wgpu::BufferUsage::MAP_READ | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });
        // then create an initialized storage buffer of appropriate size
        let storage_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                usage: match mutability {
                    Mutability::Mut => wgpu::BufferUsage::STORAGE,
                    Mutability::Const => wgpu::BufferUsage::STORAGE,
                } | wgpu::BufferUsage::COPY_SRC
                    | wgpu::BufferUsage::COPY_DST,
                contents: host_obj_bytes,
            });

        // return the final DeviceBox
        // note that we keep both the storage buffer and the staging buffer
        // we will re-use the staging buffer for reads (but not for writes, for writes we just create a new staging buffer)
        DeviceBox {
            staging_buffer,
            storage_buffer,
            size: host_obj_bytes.len() as u64,
            phantom: PhantomData,
            mutability: Some(mutability),
        }
    }

    // TODO say what is blocking and what isn't in the comments
    /// Uploads data from the given borrow to `T` to the given `DeviceBox<T>` that lives on this (meaning `self`) device
    ///
    /// ```
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut device = &mut futures::executor::block_on(Device::all())[0];
    /// let data = vec![0.0; 2048];
    /// let mut data_on_gpu: DeviceBox<[f32]> = device.create_from_mut(data.as_slice());
    /// device.set_from(&mut data_on_gpu, vec![0.5; 2048].as_slice());
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_from<T, B: Borrow<T>>(&mut self, device_obj: &mut DeviceBox<T>, host_obj: B)
    where
        T: AsBytes + ?Sized,
    {
        if device_obj.mutability.is_some() {
            assert_eq!(device_obj.mutability.unwrap(), Mutability::Mut, "expected the `DeviceBox` being set to be mutable (each `DeviceBox` constructor has a \"constant\" version and a \"mut\" version)");
        }

        // serialize the data into bytes
        // these bytes can later be deserialized back into T
        let host_obj_bytes = host_obj.borrow().as_bytes();

        // create a staging buffer with host_obj copied over
        // set this staging buffer as the new staging buffer for the device box
        let staging_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: host_obj_bytes,
                usage: wgpu::BufferUsage::COPY_SRC,
            });
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
        self.queue.submit(vec![encoder.finish()]);
    }

    /// Downloads data from the given `DeviceBox<T>` asynchronously and returns a boxed slice of `T`
    ///
    /// This functions is asynchronous so you can either `.await` it in an asynchronous context (like an `async fn` or `async` block) or you can
    /// simply pass the returned future to an executor.
    /// ```
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // get a device
    /// let mut device = &mut futures::executor::block_on(Device::all())[0];
    ///
    /// // create some data on a GPU and mutate it in place
    /// let data = vec![0.0; 2048];
    /// let mut data_on_gpu: DeviceBox<[f32]> = device.create_from_mut(data.as_slice());
    /// device.set_from(&mut data_on_gpu, vec![0.5; 2048].as_slice());
    ///
    /// // use `get` to download from the GPU
    /// assert_eq!(futures::executor::block_on(device.get(&data_on_gpu))?,
    ///     vec![0.5; 2048].into_boxed_slice());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get<T>(&mut self, device_obj: &DeviceBox<[T]>) -> Result<Box<[T]>, CompletionError>
    where
        T: FromBytes + Copy, // implicitly, T is also Sized which is necessary for us to be able to deserialize
    {
        // assert that the data we're getting is mutable
        // if it's constant, you shouldn't be getting it in the first place
        // there is a possibility it has changed and its only safe to ensure that its marked as mutable
        if device_obj.mutability.is_some() {
            assert_eq!(device_obj.mutability.unwrap(), Mutability::Mut, "the `DeviceBox` from which you are downloading data from a device should be mutable, not constant");
        }

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
        self.queue.submit(vec![encoder.finish()]);

        // now we can return a future for data read from staging buffer
        // this does a kind of complicated deserialization procedure
        // basically it does staging_buffer -> [T]
        let result = device_obj
            .staging_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read);

        //.map_read(0u64, device_obj.size); // this gets a GpuFuture<Result<BufferReadMapping, ()>>

        // poll the device
        // TODO this should not be blocking (since this is async) we need to find some way to poll a
        self.device.poll(wgpu::Maintain::Wait);

        //result.map_err(|_error| CompletionError).await?;

        result.map_err(|_| CompletionError).await?;

        Ok(device_obj
            .staging_buffer
            .slice(..)
            .get_mapped_range()
            .chunks_exact(std::mem::size_of::<T>()) // this creates an iterator over each item of size = size_of(T)
            .map(|item| {
                let layout_verified: LayoutVerified<_, T> = LayoutVerified::new(item).unwrap(); // TODO ensure this unwrap makes sense
                *layout_verified
            }) // this deserializes each size_of(T) item
            .collect()) // this collects it all into a [T]
    }

    /// Runs the given `DeviceFnMut` on a multi-dimensional space of threads to launch and arguments to pass to the launched kernel
    ///
    /// This is unsafe because it runs arbitrary code on a device.
    /// ```no_run
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut device = &mut futures::executor::block_on(Device::all())[0];
    /// let data = vec![0.0; 2048];
    /// let mut data_on_gpu: DeviceBox<[f32]> = device.create_from(data.as_slice());
    ///
    /// // these are bytes so we first convert to 4-byte words
    /// let shader: Vec<u32> = convert_to_spirv(std::io::Cursor::new(vec![
    ///     // Magic number.           Version number: 1.0.
    ///     0x03, 0x02, 0x23, 0x07,    0x00, 0x00, 0x01, 0x00,
    ///     // Generator number: 0.    Bound: 0.
    ///     0x00, 0x00, 0x00, 0x00,    0x00, 0x00, 0x00, 0x00,
    ///     // Reserved word: 0.
    ///     0x00, 0x00, 0x00, 0x00,
    ///     // OpMemoryModel.          Logical.
    ///     0x0e, 0x00, 0x03, 0x00,    0x00, 0x00, 0x00, 0x00,
    ///     // GLSL450.
    ///     0x01, 0x00, 0x00, 0x00]))?;
    ///
    /// // then, we compile to a `DeviceFnMut`
    /// // the compilation here will fail at runtime because the above shader
    /// // doesn't have an entry point called main
    /// let shader_compiled = device.compile(ParamsBuilder::new().build(), "main", shader)?;
    ///
    /// // run
    /// unsafe { device.call(&shader_compiled, (1, 1, 1), ArgsBuilder::new().build())? };
    /// # Ok(())
    /// # }
    /// ```
    pub unsafe fn call<'a>(
        &mut self,
        device_fn_mut: &DeviceFnMut,
        work_space_dim: (u32, u32, u32),
        args: DeviceFnMutArgs<'a>,
    ) -> Result<(), LaunchError> {
        // check that params and args match in type
        for (set_num, set) in &args.bind_groups {
            for (binding_num, binding) in &set.0 {
                let message = "the compiled `DeviceFnMut` does not have parameters that match the arguments being passed to it";
                let arg_type = &binding.1;
                let param_type = device_fn_mut
                    .param_types
                    .get(&set_num)
                    .expect(message)
                    .get(&binding_num)
                    .expect(message);
                if arg_type.type_name.is_some() && param_type.type_name.is_some() {
                    assert_eq!(
                        arg_type.type_name.as_ref().unwrap(),
                        param_type.type_name.as_ref().unwrap(),
                        "argument of type {:?} and parameter of type {:?} do not match in type",
                        arg_type.type_name.as_ref().unwrap(),
                        param_type.type_name.as_ref().unwrap()
                    );
                }
                if arg_type.mutability.is_some() && param_type.mutability.is_some() {
                    if param_type.mutability.unwrap() == Mutability::Mut {
                        assert_eq!(
                            arg_type.mutability.as_ref().unwrap(),
                            &Mutability::Mut,
                            "parameter is mutable so argument must also be mutable, not constant"
                        );
                    }
                }
            }
        }

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
                    entries: bind_group
                        .values()
                        .map(|binding| binding.0.clone())
                        .collect::<Vec<wgpu::BindGroupEntry<'a>>>()
                        .as_slice(),
                    // TODO ensure the above clone is okay, it should be only cloning the underlying borrow of a buffer and not cloning the entire buffer
                }),
            );
        }
        {
            // our compute pass will have 2 parts
            // 1. the pipeline, using the device_fn_mut
            // 2. the bind group, using the args
            let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor { label: None });
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
        self.queue.submit(vec![encoder.finish()]);

        Ok(())
    }

    /// Compiles a `DeviceFnMut` using the given parameters, entry point name, and SPIR-V program
    ///
    /// The entry point is where in the SPIR-V program the compiled kernel should be entered upon execution.
    /// The entry point's name is anything implementing `Into<String>` including `&str` and `String` while
    /// the program itself is anything `Borrow<[u32]>` including `Vec<u32>` and `&[u32]`.
    /// ```no_run
    /// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // get a device to use
    /// let mut device = &mut futures::executor::block_on(Device::all())[0];
    ///
    /// // these are bytes so we first convert to 4-byte words
    /// let shader: Vec<u32> = convert_to_spirv(std::io::Cursor::new(vec![
    ///     // Magic number.           Version number: 1.0.
    ///     0x03, 0x02, 0x23, 0x07,    0x00, 0x00, 0x01, 0x00,
    ///     // Generator number: 0.    Bound: 0.
    ///     0x00, 0x00, 0x00, 0x00,    0x00, 0x00, 0x00, 0x00,
    ///     // Reserved word: 0.
    ///     0x00, 0x00, 0x00, 0x00,
    ///     // OpMemoryModel.          Logical.
    ///     0x0e, 0x00, 0x03, 0x00,    0x00, 0x00, 0x00, 0x00,
    ///     // GLSL450.
    ///     0x01, 0x00, 0x00, 0x00]))?;
    ///
    /// // then, we compile to a `DeviceFnMut`
    /// // the compilation here will fail at runtime because the above shader
    /// // doesn't have an entry point called main
    /// let shader_compiled = device.compile(ParamsBuilder::new().build(), "main", shader)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn compile<T: Into<String>, P: Borrow<[u32]>>(
        &self,
        program_params: DeviceFnMutParams,
        program_entry: T,
        program: P,
    ) -> Result<DeviceFnMut, CompileError> {
        // TODO return a Result with error for compile error
        // TODO use proper error types
        let mut bind_group_layouts: HashMap<u32, wgpu::BindGroupLayout> = HashMap::new();
        let mut param_types = HashMap::new();
        for (set_num, set) in program_params.bind_group_layouts {
            // update param_types
            for (binding_num, binding) in &set {
                if !param_types.contains_key(&set_num) {
                    param_types.insert(set_num, HashMap::new());
                }
                param_types
                    .get_mut(&set_num)
                    .unwrap()
                    .insert(*binding_num, binding.1.clone());
            }
            // update bind_group_layouts
            bind_group_layouts.insert(
                set_num,
                self.device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        label: None,
                        entries: set
                            .values()
                            .map(|binding_layout| binding_layout.0.clone())
                            .collect::<Vec<wgpu::BindGroupLayoutEntry>>()
                            .as_slice(),
                    }),
            );
        }
        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: bind_group_layouts
                    .values()
                    .collect::<Vec<&wgpu::BindGroupLayout>>()
                    .as_slice(),
                push_constant_ranges: &[],
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                // TODO use a Result for this function instead of unwrap_or hack
                module: &self
                    .device
                    .create_shader_module(&wgpu::ShaderModuleDescriptor {
                        label: None,
                        source: wgpu::ShaderSource::SpirV(Cow::Borrowed(program.borrow())),
                        flags: wgpu::ShaderFlags::VALIDATION,
                    }), // this is where we compile the bytecode program itself
                entry_point: program_entry.into().as_str(), // this will probably be something like "main" or the name of the main function
            });
        Ok(DeviceFnMut {
            param_types,
            bind_group_layouts,
            compute_pipeline: pipeline,
        })
    }
}

/// Converts a slice of bytes to a slice of 4-byte words
///
/// Just as a quick example...
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let shader: Vec<u32> = convert_to_spirv(std::io::Cursor::new(vec![
///     // Magic number.           Version number: 1.0.
///     0x03, 0x02, 0x23, 0x07,    0x00, 0x00, 0x01, 0x00,
///     // Generator number: 0.    Bound: 0.
///     0x00, 0x00, 0x00, 0x00,    0x00, 0x00, 0x00, 0x00,
///     // Reserved word: 0.
///     0x00, 0x00, 0x00, 0x00,
///     // OpMemoryModel.          Logical.
///     0x0e, 0x00, 0x03, 0x00,    0x00, 0x00, 0x00, 0x00,
///     // GLSL450.
///     0x01, 0x00, 0x00, 0x00]))?;
/// # Ok(())
/// # }
/// ```
pub fn convert_to_spirv<T: Read + Seek>(src: T) -> Result<Vec<u32>, std::io::Error> {
    gfx_auxil::read_spirv(src)
}

/// A type for [boxing](https://en.wikipedia.org/wiki/Object_type_(object-oriented_programming)#Boxing) stuff stored on a device
///
/// It is generic over a type `T` so that we can safely transmute data from the
/// GPU (`DeviceBox<T>`) to and from data from the CPU (`T`). There are many ways a `DeviceBox<T>` can be constructed.
/// ```
/// # use emu_core::prelude::*;
/// # use emu_glsl::*;
/// # use zerocopy::*;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # futures::executor::block_on(assert_device_pool_initialized());
/// // this is useful for passing arguments to kernels that are of primitive types
/// let pi = DeviceBox::new(3.1415)?;
/// let data: DeviceBox<[f32]> = DeviceBox::from_ref(&vec![1.0; 2048])?;
/// let data: DeviceBox<[f32]> = DeviceBox::with_size(2048 * std::mem::size_of::<f32>())?;
/// let pi = (3.1415).into_device_boxed()?;
/// let numbers = (0..2048).into_device_boxed()?;
/// let data = vec![0; 2048].into_iter().into_device_boxed()?;
/// # Ok(())
/// # }
/// ```
/// You can also construct a `DeviceBox<T>` from existing data.
/// ```
/// # use emu_core::prelude::*;
/// # use emu_glsl::*;
/// # use zerocopy::*;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # futures::executor::block_on(assert_device_pool_initialized());
/// let one_on_cpu = vec![1.0; 1024];
/// let zero_on_cpu = vec![0.0; 1024];
/// let data_on_cpu = vec![0.5; 1024];
/// let data_on_gpu: DeviceBox<[f32]> = data_on_cpu.as_device_boxed()?;
/// let zero_on_gpu: DeviceBox<[f32]> = zero_on_cpu.into_iter().into_device_boxed()?;
/// // prefer as_device_boxed to avoid the unnecessary copy
/// // that is unless, you really need to construct from an iterator
/// let one_on_gpu: DeviceBox<[f32]> = one_on_cpu.into_iter().take(512).into_device_boxed()?;
/// # Ok(())
/// # }
/// ```
/// And you can also load custom structures onto the GPU.
/// ```
/// use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
///
/// #[repr(C)]
/// #[derive(AsBytes, FromBytes, Copy, Clone, Default, Debug)]
/// struct Shape {
///     pos: [f32; 2],
///     num_edges: u32,
///     radius: f32
/// }
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // `assert_device_pool_initialized` should be called before using any Emu API function
///     // well, except for `pool` which you would only use to manually populate the device pool
///     futures::executor::block_on(assert_device_pool_initialized());
///
///     // create data and move to the GPU
///     let shapes = vec![Shape::default(); 512];
///     let shapes_on_gpu: DeviceBox<[Shape]> = shapes.as_device_boxed()?;
///
///     Ok(())
/// }
/// ```
/// If you want to make your own collections move-able to the GPU, you can implement either [`AsDeviceBoxed`](../boxed/trait.AsDeviceBoxed.html)
/// or [`IntoDeviceBoxed`](../boxed/trait.IntoDeviceBoxed.html). Lastly, keep in mind that all of the above examples create _constant_ data.
/// To allow GPU data to be mutated, for most of the above functions, their mutable equivalent has the same name but with a `mut` appended.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # futures::executor::block_on(assert_device_pool_initialized());
/// let one_on_cpu = vec![1.0; 1024];
/// let zero_on_cpu = vec![0.0; 1024];
/// let data_on_cpu = vec![0.5; 1024];
/// let data_on_gpu: DeviceBox<[f32]> = data_on_cpu.as_device_boxed_mut()?;
/// let mut zero_on_gpu: DeviceBox<[f32]> = zero_on_cpu.into_iter().into_device_boxed_mut()?;
/// let one_on_gpu: DeviceBox<[f32]> = one_on_cpu.into_iter().take(512).into_device_boxed_mut()?;
/// # Ok(())
/// # }
/// ```
/// Emu keeps tracks of whether or not data is mutable as well as their type to ensure that data is safely passed back and forth to and from
/// kernels running on a GPU.
///
/// Also, `DeviceBox` implements `From` and `Into` to help you switch between `DeviceBox` and its WebGPU internals if you want to.
/// The WebGPU internals are encapsulated in a 4-tuple corresponding simply to the staging buffer, storage buffer, and size in bytes respectively (there is also an optional mutability marker).
/// You should ignore the staging buffer for now since we are working towards replacing 1 staging buffer per `DeviceBox` with a global pool of staging buffers
/// that is shared by all `DeviceBox`s..
pub struct DeviceBox<T>
where
    T: ?Sized,
{
    pub(crate) staging_buffer: wgpu::Buffer,
    pub(crate) storage_buffer: wgpu::Buffer,
    pub(crate) size: u64, // inv: size being constant and equal to sizes of staging, storage buffers respectively
    pub(crate) phantom: PhantomData<T>,
    pub(crate) mutability: Option<Mutability>, // TODO for now constant scalars are passed in as storage buffers
                                               // this is fine for now but in the future we should allow a DeviceBox to potentially use a uniform for small sizes of constant data
                                               // this optimization would make memory transfer faster (maybe)
}

impl<T: ?Sized> From<(wgpu::Buffer, wgpu::Buffer, u64, Option<Mutability>)> for DeviceBox<T> {
    fn from(wgpu_stuff: (wgpu::Buffer, wgpu::Buffer, u64, Option<Mutability>)) -> Self {
        Self {
            staging_buffer: wgpu_stuff.0,
            storage_buffer: wgpu_stuff.1,
            size: wgpu_stuff.2,
            phantom: PhantomData,
            mutability: wgpu_stuff.3,
        }
    }
}

impl<T: ?Sized> Into<(wgpu::Buffer, wgpu::Buffer, u64, Option<Mutability>)> for DeviceBox<T> {
    fn into(self) -> (wgpu::Buffer, wgpu::Buffer, u64, Option<Mutability>) {
        (
            self.staging_buffer,
            self.storage_buffer,
            self.size,
            self.mutability,
        )
    }
}

/// Represents a compiled kernel that can then be launched across spawned threads with [`Device::call`](struct.Device.html#method.call) or [`spawn`](../spawn/fn.spawn.html)
///
/// While compiling a `DeviceFnMut` is expensive, running a `DeviceFnMut` with varying work space dimensions or arguments incurs no significant extra compilation.
/// There isn't really much you will need to do with this. Just know that this is basically the final compiled kernel. It's the end of the compilation pipeline (it's generated
/// from SPIR-V) and is the input to the execution of your kernel.
#[derive(From, Into)]
pub struct DeviceFnMut {
    // we really just need 2 things to define a function
    // 1. the layout of input buffers to be bound (think of this as declaring the parameters of the function)
    // 2. the shader module and its entry point (this is like the actual body of the function)
    // both of these can be used to produce the following
    pub(crate) param_types: HashMap<u32, HashMap<u32, ArgAndParamInfo>>, // you can just set all types to None if you don't care about type checking
    pub(crate) bind_group_layouts: HashMap<u32, wgpu::BindGroupLayout>,  // u32 = set number
    pub(crate) compute_pipeline: wgpu::ComputePipeline, // inv: has PipelineLayout consistent with above BindGroupLayout's
}

/// Describes the parameters that can be passed to a `DeviceFnMut`
///
/// This is cheap to construct and something you can safely clone multiple times.
/// See [`ParamsBuilder`](struct.ParamsBuilder.html) for a convenience builder of `DeviceFnMutParams`.
/// `DeviceFnMutParams` encapsulates a map from each set number to a map from each binding in the set to
/// a binding layout. The binding layout contains both the `wgpu::BindGroupLayoutEntry` and an `ArgAndParamInfo` storing information
/// for each parameter.
///
/// Looking into WebGPU docs and Emu source code is probably the best way to figure out how to work with the WebGPU
/// data structures encapsulated by `DeviceFnMutParams`.
#[derive(From, Into, Clone)]
pub struct DeviceFnMutParams {
    bind_group_layouts: HashMap<u32, HashMap<u32, (wgpu::BindGroupLayoutEntry, ArgAndParamInfo)>>, // (u32, u32) = (set number, binding number)
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
                (
                    wgpu::BindGroupLayoutEntry {
                        binding: new_binding_layout_idx,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None, // for now, this is just always mutable bc I don't know if readonly is more performant
                        },
                        count: None,
                    },
                    ArgAndParamInfo::default(),
                ),
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
///
/// `ParamsBuilder` helps you build a `DeviceFnMutParams` by specifying whether or not each parameter is mutable.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # futures::executor::block_on(assert_device_pool_initialized());
/// let data: DeviceBox<[f32]> = vec![0.0; 4096].as_device_boxed_mut()?;
/// let tau = DeviceBox::new(6.2832)?;
/// let args = ParamsBuilder::new()
///     .param::<[f32]>(Mutability::Mut)
///     .param::<f32>(Mutability::Const)
///     .build();
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct ParamsBuilder {
    binding_layouts: HashMap<u32, (wgpu::BindGroupLayoutEntry, ArgAndParamInfo)>,
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
    pub fn param<T: ?Sized>(mut self, mutability: Mutability) -> Self {
        let new_binding_layout_idx = self.binding_layouts.len() as u32;
        self.binding_layouts.insert(
            new_binding_layout_idx,
            (
                wgpu::BindGroupLayoutEntry {
                    binding: new_binding_layout_idx,
                    visibility: wgpu::ShaderStage::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        has_dynamic_offset: false, // we usually don't need dynamic for compute so we default to 0, of course if you need it you can provide your own Device...Params
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: mutability == Mutability::Const,
                        },
                        //min_binding_size: std::mem::size_of::<T>(),
                        min_binding_size: None,
                    },
                    count: None,
                },
                ArgAndParamInfo {
                    type_name: Some(String::from(core::any::type_name::<T>())),
                    mutability: Some(mutability),
                },
            ), // for now we use type name, in the future we will use something more unique like core::any::TypeID
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

/// Information that is held by both arguments and by parameters
///
/// If its fields are `Some`, `ArgAndParamInfo` can be used to check whether or not
/// arguments and parameters are compatible
#[derive(Default, PartialEq, Hash, Clone)]
pub struct ArgAndParamInfo {
    type_name: Option<String>, // in the future, we should use core::any::TypeId
    mutability: Option<Mutability>,
}

/// Holds the actual arguments to be passed into a [`DeviceFnMut`](struct.DeviceFnMut.html)
///
/// See [`ArgsBuilder`](struct.ArgsBuilder.html) for a convenience builder of `DeviceFnMutArgs`.
/// `DeviceFnMutArgs` encapsulates a map from set numbers to maps from binding numbers to bindings.
/// Each binding stores both a `wgpu::Binding` and an `ArgAndParamInfo` for the argument being bound.
/// Each set stores a `Vec<u32>` which can be empty as a reasonable default.
///
/// Looking into WebGPU docs and Emu source code is probably the best way to figure out how to work with the WebGPU
/// data structures encapsulated by `DeviceFnMutArgs`.
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
    // a wgpu::Binding owns a borrow of data (like a wgpu::Buffer owned by a DeviceBox)
    // we must ensure that DeviceFnMutArgs doesn't outlive the Buffer (and maybe DeviceBox) that it refers to
    //
    // and technically there can't be more than 4 sets (I think) but we still just use a HashMap for convenience
    bind_groups: HashMap<
        u32,
        (
            HashMap<u32, (wgpu::BindGroupEntry<'a>, ArgAndParamInfo)>,
            Vec</*wgpu::BufferAddress*/ u32>,
        ),
    >, // (u32, u32) = (set number, binding number)
}

/// Helps with building a `DeviceFnMutArgs`
///
/// `ArgsBuilder` helps you build a `DeviceFnMutArgs` by providing references to each `DeviceBox` argument. It's perfectly safe to
/// pass a reference to a mutable `DeviceBox`. If the kernel these arguments are being passed to only accepts mutable arguments, Emu
/// will assert that they are at runtime.
/// ```
/// # use {emu_core::prelude::*, emu_glsl::*, zerocopy::*};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # futures::executor::block_on(assert_device_pool_initialized());
/// let data: DeviceBox<[f32]> = vec![0.0; 4096].as_device_boxed()?;
/// let tau = DeviceBox::new(6.2832)?;
/// let args = ArgsBuilder::new()
///     .arg(&data)
///     .arg(&tau)
///     .build();
/// # Ok(())
/// # }
/// ```
pub struct ArgsBuilder<'a> {
    bindings: HashMap<u32, (wgpu::BindGroupEntry<'a>, ArgAndParamInfo)>,
}

impl<'a> ArgsBuilder<'a> {
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
            (
                wgpu::BindGroupEntry {
                    binding: new_binding_idx,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &device_obj.storage_buffer,
                        offset: 0,
                        size: Some(NonZeroU64::new(device_obj.size).unwrap()),
                    },
                },
                ArgAndParamInfo {
                    type_name: Some(String::from(core::any::type_name::<T>())),
                    mutability: device_obj.mutability,
                },
            ), // for now we use type name, in the future we will use something more unique like core::any::TypeID
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
