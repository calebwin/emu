//! This is not the main documentation for Emu. Go to [docs.rs/em](https://docs.rs/em)
//! for the main documentation of Emu.

// for generating Rust
#[macro_use]
extern crate quote;

// for procedural macros
extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;

// for parsing Rust
extern crate syn;
use syn::fold::Fold;
use syn::*;

// THE TABLE OF CONTENTS
//
// these modules are the main modules Emu uses
mod accelerating; // for looking through code for gpu_do!() and using the GPU appropriately
mod passing; // for passing around a reference to the GPU from function to function
             // these modules are more linke utilities for Emu
mod generator; // for generating OpenCL from Rust
mod identifier; // for identifying a for loop as potentially something we can work with
mod inspector; // for inspecting a function for more info

use accelerating::*;
use inspector::*;
use passing::*;

// TODO document this somewhere
// let's consider the following where x is of type T
// gpu_do!(load(x))
// gpu_do!(read(x))
// here are the restrictions for what T can be
// - T must have .as_slice() for reading from slice to GPU
// - T must have .as_mut_slice() for writing to slice back from GPU
// - T must implement Index, IndexMut for use inside a launched loop
// these requirements are here for 2 reasons
// 1. loading and reading T should be possible for GPU
// 2. using T inside of launched loop should be same on GPU or CPU
// by following these requirements you can use not only Vec but also your
// own types like a Tensor or Matrix or Queue
// of course, you can't use methods and stuff but Emu already enforces that

// error represents an error in compilation that makes it more confusing to user to proceed
// if e is an error, we just stop the proc macro execution and just return what was already there + errors
//
// we append errors because it's a bit of a hack to make them show up as compile-time errors to macro users
macro_rules! unwrap_or_return {
    ( $e:expr, $r:expr ) => {
        match $e {
            Ok(x) => x,
            Err(raw_errors) => return {
                // TODO make sure that r is not something that has been transformed at all by the macro
                // r should be the existing whatever code we found. we just want to append errors to it

                // parse Rust code into AST
                let maybe_ast = syn::parse::<ItemFn>($r.clone());

                if maybe_ast.is_ok() {
                    let code = maybe_ast.unwrap();
                    let errors = raw_errors.iter().map(|raw_error| raw_error.to_compile_error()).collect::<Vec<_>>();

                    (quote! {
                        #code
                        #(#errors)*
                    }).into()
                } else {
                    Error::new(Span::call_site().unwrap().into(), "only functions that are items can be tagged with `#[gpu_use]`").to_compile_error().into()
                }
            },
        }
    }
}

/// A procedural macro for using the GPU to store data and accelerate parts of code in the tagged function.
///
/// While this is technically what looks through your code for `gpu_do!()`
/// declarations and expands them in ways that potentially accelerate your
/// code, you should think about this more as the "passing" part of Emu rather
/// than the "acceleration" part.
///
/// Well, what do I mean by passing? I've talked in other parts of the docs
/// about the "GPU being in scope" or "passing the GPU around". I literally
/// mean an identifier `gpu` that references an instance of `Gpu` and holds
/// all information about the physical GPU that Emu needs to know. Why do we
/// need to "pass" this `Gpu` instance from function to function? Well, we
/// would like only 1 instance of `Gpu` to exist throughout an entire
/// application. Even if you are using other libaries, those libraries should
/// be using this 1 instance of `Gpu`. The way we accomplish this is by
/// modifying functions (that use the GPU) to accept as input, return as
/// output, pass around the `Gpu` instance. That "modification" of functions
/// is done by this macro.
///
/// So, how do I do passing? If you correctly tag your function with
/// `#[gpu_use]`, it's function signature will be modified and its contents
/// will be modified so as to allow `gpu`, a `Gpu` instance, to be used in
/// the function body. To correctly tag your function with `#[gpu_use]`, you
/// must list the "helper functions" of the tagged function.
/// ```
/// # extern crate em;
/// # use em::*;
/// # #[gpu_use(multiply)]
/// # fn multiply(data: Vec<f32>, scalar: f32) -> Vec<f32> {data}
/// # #[gpu_use(add_one_then_double)]
/// # fn add_one_then_double(data: Vec<f32>) -> Vec<f32> {data}
/// #[gpu_use(multiply, add_one_then_double)]
/// fn main() {
///     let mut data = vec![0.1; 1000];
///
///     gpu_do!(load(data));
///     data = multiply(data, 10.0);
///     data = add_one_then_double(data);
///     gpu_do!(read(data));
///
///     println!("{:?}", data);
/// }
/// ```
/// In this above example, the helper functions of the tagged function are `multiply` and `add_one_then_double`. What would those helper functions look like themselves?
/// ```
/// # extern crate em;
/// # use em::*;
/// #[gpu_use(multiply)]
/// fn multiply(mut data: Vec<f32>, scalar: f32) -> Vec<f32> {
///     gpu_do!(launch());
///     for i in 0..1000 {
///         data[i] = data[i] * scalar;
///     }
///
///     data
/// }
///
/// #[gpu_use(add_one_then_double, multiply)]
/// fn add_one_then_double(mut data: Vec<f32>) -> Vec<f32> {
///     gpu_do!(launch());
///     for i in 0..1000 {
///         data[i] = data[i] + 1.0;
///     }
///     data = multiply(data, 2.0);
///
///     data
/// }
///
/// #[gpu_use(multiply, add_one_then_double)]
/// fn main() {
///     let mut data = vec![0.1; 1000];
///
///     gpu_do!(load(data));
///     data = multiply(data, 10.0);
///     data = add_one_then_double(data);
///     gpu_do!(read(data));
/// }
/// ```
/// There is, admittedly, a lot going on here. But there is just one thing I want you to focus
/// on - the `#[gpu_use()]` invocations/tags. Focus on that. See how different
/// functions have different helper functions. First look at the main function and its helper functions and then look at the other two functions. You will see that the helper functions of a
/// function are each only one of 2 things.
/// 1. A function called from inside the function body that uses the GPU (is tagged with `#[gpu_use]`)
/// 2. The function itself, if it can be a helper function to another function
///
/// Looking at the above example you should be able to justify each helper
/// function listed for each function, using the above 2 cases. Note that the `main` function doesn't list itself as a helper function and that is because
/// it doesn't need the GPU passed to it ever.
#[proc_macro_attribute]
pub fn gpu_use(metadata: TokenStream, mut input: TokenStream) -> TokenStream {
    // there are 3 parts of Emu's procedural code generation
    // these are 3 pieces of code that Emu must generate
    // (1) movement of Gpu from function to function
    // (2) movement of data on Gpu <-> CPU
    // (3) launching of kernels

    // (1) movement of Gpu from function to function

    // find declared helper functions
    let attribute_args = parse_macro_input!(metadata as AttributeArgs);
    let declared_helper_functions =
        unwrap_or_return!(get_declared_helper_functions(attribute_args), input);

    // check if current function is a declared helper function
    let mut is_declared_helper_function = false;
    let function_info = unwrap_or_return!(get_function_info(input.clone()), input);
    for declared_helper_function in &declared_helper_functions.clone() {
        if function_info.name == *declared_helper_function {
            is_declared_helper_function = true;
        }
    }

    // handle all invocations of helper functions
    // GPU must be passed to and back from helper function
    // result of helper function must be used in original way if a result is returned
    input = unwrap_or_return!(
        modify_invocations(input.clone(), declared_helper_functions),
        input
    );

    // handle the current function being a declared helper function
    // basically, we need to transform the function so that it can take a GPU as input and return the modified GPU as output
    if is_declared_helper_function {
        // modify signature and returns
        input = unwrap_or_return!(
            modify_signature_for_helper_function(input.clone(), function_info.has_return),
            input
        );
        input = unwrap_or_return!(
            modify_return_for_helper_function(input.clone(), function_info.has_return),
            input
        );
        input = unwrap_or_return!(modify_returns_for_helper_function(input.clone()), input);
    } else {
        // modify body by adding boilerplate to create GPU to be passed to helper functions
        input = unwrap_or_return!(modify_for_not_a_helper_function(input.clone()), input);
    }

    // (2) movement of data on Gpu <-> CPU by visit_macro
    // (3) launching of kernels by visit_for_loop

    // create new accelerator
    let mut accelerator = Accelerator::new();

    // parse Rust code into AST
    let maybe_ast = syn::parse::<ItemFn>(input.clone());

    if maybe_ast.is_ok() {
        // transform AST
        let new_ast = accelerator.fold_item_fn(maybe_ast.unwrap());

        // // print AST
        // println!("{}", new_ast.to_token_stream().to_string());

        let errors = accelerator
            .errors
            .iter()
            .map(|raw_error| raw_error.to_compile_error())
            .collect::<Vec<_>>();

        (quote! {
            #new_ast
            #(#errors)*
        })
        .into()
    } else {
        Error::new(
            Span::call_site().unwrap().into(),
            "only functions that are items can be tagged with `#[gpu_use]`",
        )
        .to_compile_error()
        .into()
    }
}
