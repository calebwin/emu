# The Emu Book
- Chapter 0 - [Introduction](https://github.com/calebwin/emu/blob/master/book/introduction.md)
- Chapter 1 - [The Body](https://github.com/calebwin/emu/blob/master/book/body.md)
- Chapter 2 - [The Parameters](https://github.com/calebwin/emu/blob/master/book/parameters.md)
- Chapter 3 - [The Functions](https://github.com/calebwin/emu/blob/master/book/functions.md)
- Chapter 4 - [The Numbers](https://github.com/calebwin/emu/blob/master/book/numbers.md)
- Chapter 5 - [The Execution](https://github.com/calebwin/emu/blob/master/book/execution.md)

# Parameters
There are 3 parts of every function defined in the Emu language - (1) the name, (2) the parameters, and (3) the body containing the code to be executed when the function is invoked. Here is an example of the parameters of a function-
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

You likely noticed that two of the parameters in the above example have names prefixed by `global_`. What does that mean? Every parameter of a function defined in Emu [belongs to a certain address space](https://www.khronos.org/registry/OpenCL/sdk/1.1/docs/man/xhtml/qualifiers.html). By default, all parameters belong to the `__private__` address space. You can make a parameter belong to the global or local address spaces by prefixing the name of the parameter with `global_` or `local_`. 

This may seem like it flies in the face of the ["Explicit better than implicit" philosophy](https://www.python.org/dev/peps/pep-0020/). However, there are two main reasons for doing this - (1) it's significantly more readable for someone reading the code and (2) it's a documented feature that someone who is actually modifying the code should be aware of.

Note that if you aren't using a binding to OpenCL to run your code and instead using the `build` macro to generate Rust functions to call your Emu functions, you don't need to think to much about what address spaces are. Just use `global_` for all vectors you want to pass in as parameters and no prefix for everything else.

One final thing to note is the brackets around two of the parameters. Brackets around the type mean that the type of the parameter is - a vector (technically a pointer in the generated OpenCL code) of the type inside the brackets. `global_num` or `global_res` can be indexed as vectors inside the body of the function with `global_num[2]` or `global_res[0]`.
