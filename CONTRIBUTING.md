# contributing

The roadmap for Emu is simple - support a larger subset of Rust. In what order, though do we want to expand the subset, though? Consider the following program-

```rust
#[macro_use]
extern crate em;
use em::*;

#[gpu_use]
fn main() {
    let mut x = vec![0.0; 1000];
    gpu_do!(load(x)); // move data to the GPU
    
    gpu_do!(launch()); // off-load to run on the GPU
    for i in 0..1000 {
        x[i] = x[i] * 10.0;
    }

    gpu_do!(read(x)); // move data back from the GPU
    println!("{:?}", x);
}
```

The order we want to expoand the subset is from outside to inside. From outside to inside, we will-

1. work on `#[gpu_use]`
2. work on `gpu_do!()`
3. work on `gpu_do!(launch())` and support more kinds of for loops
4. work on supporting more kinds of stuff inside the for loops
5. etc.

Some of the things that need to be worked on include but are not limited to-

1. work on `#[gpu_use]`
   - [ ] support methods
   - [ ] support `?` operator
2. work on `gpu_do!()`
   - [ ] support `gpu_do!(load_async(data))`, `gpu_do!(read_async(data))`, `gpu_do!(launch_async())`, `gpu_do!(wait())` for asynchronous usage of GPUs
   - [ ] support `gpu_do!(open("nvidia ti"))`, `#[gpu_use("nvidia ti", "nvidia gt")]` for usage of multiple GPUs
   - [ ] support `gpu_do!(load_mut(data))` to optimize for data that isn't actually mutable
3. work on `gpu_do!(launch())` and support more kinds of for loops
   - [ ] support loops that declare block algorithms/blocked iteration/iteration over chunks
   - [ ] support loops that `for x in data`
   - [ ] support loops that `for x in 0..(i * j)`
4. work on supporting more kinds of stuff inside the for loops
   - [ ] support variables
   - [ ] support all binary/unary operators
   - [ ] support if/else-if/else
   - [ ] support loops
   - [ ] support more primitive types such as `char`, `i32`, `u128`
5. etc.
   - [ ] support structures with `#[gpu_use_struct]`
   - [ ] support functions (that can be called from inside of for loop) `#[gpu_use_fn]`

To be even more specific, here is what we want to do-

(Before you look through this list, please carefully read the [docs.rs/em](https://docs.rs/em/0.3.0/em/) so that you understand the terms `#[gpu_use]`, `gpu_do!()`, "helper functions". You might also want to see this [table of contents](https://github.com/calebwin/emu/blob/master/emu_macro/src/lib.rs#L18) of code to understand how Emu code is structured.)

# 1. support methods

Currently, `#[gpu_use]` modifies 5 things.

- the function signature to accept a `Gpu` (docs [here](https://docs.rs/em/0.3.0/em/struct.Gpu.html)) as input (if this is a helper function) (code [here](https://github.com/calebwin/emu/blob/master/emu_macro/src/passing.rs#L74))
- the function signature to return the modified inputted `Gpu` as output (modifying `-> T` to `-> (T, Gpu)`) (if this is a helper function) (code also [here](https://github.com/calebwin/emu/blob/master/emu_macro/src/passing.rs#L74))
- the body of the function to either use and mutate the `Gpu` as described by all `gpu_do!()`s (if this is a helper function) or otherwise instantiate a new `Gpu` (code [here](https://github.com/calebwin/emu/blob/master/emu_macro/src/accelerating.rs#L208) and [here](https://github.com/calebwin/emu/blob/master/emu_macro/src/passing.rs#L149))
- the body of the function to use and mutate the `Gpu` by passing it into and out of all helper functions that are called in the body (code [here](https://github.com/calebwin/emu/blob/master/emu_macro/src/passing.rs#L149))
- the body of the function to return the mutated `Gpu` (if this is a helper function) (code [here](https://github.com/calebwin/emu/blob/master/emu_macro/src/passing.rs#L149) and [here](https://github.com/calebwin/emu/blob/master/emu_macro/src/passing.rs#L149))

All of these modifications are great but they they don't support methods. Ideally we should support something like this.

```rust
struct MyVec {
    data: Vec<f32>
    // ... other fields
}

impl MyVec {
    #[gpu_use(add)]
    fn add(&mut self, other: Vec<f32>) {
        // ... add the vectors and update 
    }
    
    #[gpu_use(other_add)]
    fn other_add(x: Vec<f32>, y: Vec<f32>) -> Vec<f32> {
        // ... some stuff
    }
}

#[gpu_use(add, other_add)]
fn main() {
    // ... some initialization code
    gpu_do!(load(data));
    my_vec.add(data);
    MyVec::other_add(data, vec![0.0; 1000]);
}
```

To make this code possible, there are 2 things we need to change.

- modifying signatures (`Gpu` parameter needs to be added at end not start and passed as argument at end not start)
- modifying calls (we should support any call/method and when we see something like `MyVec::other_add` we should only look at `other_add`

These 2 changes will need to be made in the following places.

- modify `lib.rs` and some other places to not only parse for `ItemFn` but also methods
- modify `passing.rs` to change how signatures are modified
- modify `passing.rs` to change how calls are modified

# 2. support `?` operator

Remember how I said `#[gpu_use]` modifies things? Here was one thing I said.

- the body of the function to return the mutated `Gpu` (if this is a helper function) (code [here](https://github.com/calebwin/emu/blob/master/emu_macro/src/passing.rs#L149) and [here](https://github.com/calebwin/emu/blob/master/emu_macro/src/passing.rs#L149))

There are 2 ways we do this.

1. wrap the entire function body by transorming `body` to `(body, gpu)`.
2. modify returns by transforming `return x` to `return (x, gpu)`.

This handles 2 ways that stuff can be returned from functions.

1. blocks as expressions with last expression being what gets returned
2. return statements

But there are at least 2 more.

3. macros
4. `?` operator implicitly returning an error in case of error and otherwise unwrapping and evaluating to unwrapped value

We can ignore macros because we simply can't expand them at compile time on stable Rust from a proc macro. But the `?` is important. To support `?` operator, we need to add the following.

- finding `?` operator and modifying it

This entails 2 things.

- modify `passing.rs` to include a new function that uses [`Fold`](https://docs.rs/syn/1.0.7/syn/fold/index.html) to modify `?` to return the `Gpu`
- modify `lib.rs` to call the function from `passing.rs` on code in the function

# 3...

Of course, this isn't a complete list. This list will change. But unlike the list on the README, this list contains actual things that we are almost certain needs to be done. And the order of this list is also something we are almost certain about. If you have more questions about anything, please discuss at [gitter.im/talk-about-emu/thoughts](https://gitter.im/talk-about-emu/thoughts).
