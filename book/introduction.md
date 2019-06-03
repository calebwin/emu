# The Emu Book
- Chapter 0 - [Introduction](https://github.com/calebwin/emu/blob/master/book/introduction.md#the-emu-book)
- Chapter 1 - [The Body](https://github.com/calebwin/emu/blob/master/book/body.md#the-emu-book)
- Chapter 2 - [The Parameters](https://github.com/calebwin/emu/blob/master/book/parameters.md#the-emu-book)
- Chapter 3 - [The Functions](https://github.com/calebwin/emu/blob/master/book/functions.md#the-emu-book)
- Chapter 4 - [The Numbers](https://github.com/calebwin/emu/blob/master/book/numbers.md#the-emu-book)
- Chapter 5 - [The Execution](https://github.com/calebwin/emu/blob/master/book/execution.md#the-emu-book)

# Introduction

Emu is a language for programming GPUs from Rust. Particularly, it provides 2 procedural macros -  `emu!` for compiling functions in the Emu language to intermediate code (currently that is OpenCL) stored it in a `const` `&'static str` global constant called `EMU` and `build!` for generating Rust functions that interpret the intermediate code found in `EMU`.

You can get started by adding the following to your `Cargo.toml` file.

```toml
[dependencies]
em = "0.1.3"
```

Then add the following to the file where you would like to use the `emu!` macro.

```rust
extern crate em;
use em::emu;
```

The following is also necessary if you would like to use the `build!` macro.

```rust
use em::build;

extern crate ocl;
use ocl::{flags, Platform, Device, Context, Queue, Program, Buffer, Kernel};
```

Now that you have imported the crates you need to compile and run Emu functions, you are ready to start writing Emu functions yourself. In Emu, there are two types of functions, ones that just do stuff and ones that do stuff and then also return some sort of result. If your function just does stuff, you can write it easily with the `emu!` macro.

```rust
emu! {
	move_particles(particles [f32]) {
		// more code here...
	}
    
	rotate_particles(particles [f32]) {
		// some more code here...
	}
}
```

As you can see there are three main parts of the function - (1) the name, (2) the parameters of the function, (3) the body code of the function. You can define multiple functions within a call to the `emu!` macro but you can't call the `emu!` macro as many times as you want; remember - the macro is defining a global constant. Calling the macro multiple times one after the other is re-defining the global constant `EMU` multiple times. 

The second (and only other) kind of function in Emu is the kind the returns a value. The syntax for doing this is pretty straightforward.

```rust
emu! {
	collapse(x f32, y f32, z f32) f32 {
		return x + y + z;
	}
    
	scale(input f32, multiple f32) f32 {
		return input * multiple;
	}
}
```

Now you're probably wondering - how do I actually run these functions? This question is answered in [the chapter on execution](https://github.com/calebwin/emu/blob/master/book/execution.md). The next few chapters will also go into more details on what you can have in the parameters and in the body of a function in the Emu language.
