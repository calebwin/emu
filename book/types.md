# Table of contents
- Chapter 1 - [Language](https://github.com/calebwin/emu/tree/master/book/language.md#table-of-contents)
- Chapter 2 - [Types](https://github.com/calebwin/emu/tree/master/book/types.md#table-of-contents)
- Chapter 3 - [Holes](https://github.com/calebwin/emu/tree/master/book/holes.md#table-of-contents)
- Chapter 4 - [Numbers](https://github.com/calebwin/emu/tree/master/book/numbers.md#table-of-contents)

# Types
In Emu, a scalar value can be one of the following types.

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

An array is of one of the above types of scalar values. An array type looks a bit different from a scalar type
```rust
// a scalar type
i32

// an array type of a scalar type
[i32]
```
