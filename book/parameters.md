#Parameters
There are 3 parts of every kernel function defined in the Emu language - (1) the name, (2) the parameters, and (3) the body containing the code to be executed when the kernel function is invoked on a work item. Here is an example of the parameters of a function-
```rust
(coeff as f32, num as f32 * GLOBAL, res as f32 * GLOBAL)
```

The first thing you should notice is that the parameters are seperated by commas. Each parameter has 3 parts - (1) the name, (2) the type, and (3) the location any value passed in as the parameter must come from. The name is pretty straightforward - it can be any typical identifier you would noramlly see in Rust. The type is just that, the type of the parameter. It can be one of the following-
| f32 | A 32-bit floating point number           |
|-----|------------------------------------------|
| i8  | A character or an 8-bit integer          |
| i16 | A 16-bit integer                         |
| i32 | A 32-bit integer                         |
| i64 | A 64-bit integer                         |
| u8  | A character or an unsigned 8-bit integer |
| u16 | An unsigned 16-bit integer               |
| u32 | An unsigned 32-bit integer               |
| u64 | An unsigned 64-bit integer               |

The third part of a parameter is a bit more complicated. Every parameter comes from a certain "address space." In the Source language, this can be `GLOBAL` or `LOCAL`. If the address space of a paramter is global, it means the value can be accessed from anywhere. If the space is local, it measn the value can only be accessed within a work group. You can also leave this out entirely meaning the parameter can only be accessed from withing the function.
