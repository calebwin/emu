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
// |
// |
// |
// these modules are the main modules Emu uses
mod accelerating;
mod passing; // for passing around a reference to the GPU from function to function // for looking through code for gpu_do!() and using the GPU appropriately
                                                                                    // |
                                                                                    // |
                                                                                    // |
                                                                                    // these modules are more linke utilities for Emu
mod generator;
mod identifier; // for identifying a for loop as potentially something we can work with
mod inspector; // for inspecting a function for more info // for generating OpenCL from Rust

use accelerating::*;
use inspector::*;
use passing::*;

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
/// Errors may occur as the procedural macro looks through code. These are reported in a clean manner using `std::compile_error`.
/// An example of using the macro is shown below.
/// ```compile_fail
/// # extern crate em;
/// # use em::*;
///
/// #[gpu_use]
/// fn main() {
///     let mut data = vec![0.1; 1000];
///
///     gpu_do!(load(data));
///     gpu_do!(launch());
///     for i in 0..1000 {
///         data[i] = data[i] * 10.0;
///     }
///     gpu_do!(read(data));
///
///     println!("{:?}", data);
/// }
/// ```
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
