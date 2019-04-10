# Parameters
There are 3 parts of every kernel function defined in the Emu language - (1) the name, (2) the parameters, and (3) the body containing the code to be executed when the kernel function is invoked on a work item. Here is an example of the parameters of a function-
```rust
(coeff f32, global_num [f32], global_res [f32])
```

The first thing you should notice is that the parameters are seperated by commas. Each parameter has 2 parts - (1) the name and (2) the type. The name is pretty straightforward - it can be any typical identifier you would noramlly see in Rust. The type is just that, the type of the parameter. It can be one of the following-

| Type  | Description           |
| ---- | ---------------------------------------- |
| f32  | A 32-bit floating point number           |
| i8   | A character or an 8-bit integer          |
| i16  | A 16-bit integer                         |
| i32  | A 32-bit integer                         |
| i64  | A 64-bit integer                         |
| u8   | A character or an unsigned 8-bit integer |
| u16  | An unsigned 16-bit integer               |
| u32  | An unsigned 32-bit integer               |
| u64  | An unsigned 64-bit integer               |         

You likely noticed that two of the parameters in the above example have names prefixed by `global_`. What does that mean? Every parameter of a kernel function defined in Emu [belongs to a certain address space](https://www.khronos.org/registry/OpenCL/sdk/1.1/docs/man/xhtml/qualifiers.html). By default, all parameters belong to the `__private__` address space. You can make a parameter belong to the global or local address spaces by prefixing the name of the parameter with `global_` or `local_`. 

This may seem like it flies in the face of the ["Explicit better than implicit" philosophy](https://www.python.org/dev/peps/pep-0020/). However, there are two main reasons for doing this - (1) it's significantly more readable for someone reading the code and (2) it's a documented feature that someone who is actually modifying the code should be aware of.

One final thing to note is the brackets around two of the parameters. Brackets around the type mean that the type of the parameter is - an array (technically a pointer in the generated OpenCL code) of the type inside the brackets. `global_num` or `global_res` can be indexed as arrays inside the body of the kernel function.
