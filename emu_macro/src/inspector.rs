// for procedural macros
extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;

// for parsing Rust
extern crate syn;
use syn::spanned::Spanned;
use syn::*;

// for etc.
use std::result::Result;

// this is used for storing info about the functions that
// #[gpu_use] is usually tagged with
pub struct FunctionInfo {
    pub name: Ident,
    pub has_return: bool,
}

// looks at AttributeArgs in an invocation of #[gpu_use]
// to see what helper functions are declared
//
// for example, #[gpu_use] should return at empty Vec
// but, #[gpu_use(multiply, add, subtract)] should return a Vec of length 3
// containing multiply, add, subtract
//
// for more information on what a helper function is, look at the passing.rs module
// passing is all about passing the GPU around from function to function
// we need to know what helper functions use the GPU in order to know which ones
// can and should have the GPU passed to them
pub fn get_declared_helper_functions(
    attribute_args: AttributeArgs,
) -> Result<Vec<Ident>, Vec<syn::Error>> {
    let mut declared_helper_functions = vec![];
    let mut errors = vec![];

    // note that this is one place where we try to collect as many errors as we can and
    // don't just return ASAP
    // this is because it would still be helpful to keep looking for errors
    // and also it would not lead to any incorrect compile errors
    for attribute_arg in attribute_args {
        if let NestedMeta::Meta(meta) = attribute_arg {
            if let Meta::Path(path) = meta {
                if let Some(ident) = path.get_ident() {
                    // only a helper function declaration if it is an identifier in a list of them
                    declared_helper_functions.push((*ident).clone());
                } else {
                    errors.push(syn::Error::new(
                        path.span(),
                        "expected identifier/name of helper function",
                    ));
                }
            } else {
                errors.push(syn::Error::new(
                    meta.span(),
                    "expected name of helper function",
                ));
            }
        } else {
            errors.push(syn::Error::new(
                attribute_arg.span(),
                "expected name of helper function",
            ));
        }
    }

    if errors.len() > 0 {
        // must be at least 1 error for this Result to be an Err
        Err(errors)
    } else {
        Ok(declared_helper_functions)
    }
}

// gets information about the function
//
// this is always called only for functions that are tagged with #[gpu_use]
// if there is anything about the signature that makes this an invalid function to be tagged,
// this is where an error should be detected
//
// also, if an error is detected, stop execution as soon as possible
// in general, when errors are detected we try to get back to a point in execution where we
// can make the macro return with no modifications to the function but with
// compile-time errors appended
pub fn get_function_info(input: TokenStream) -> Result<FunctionInfo, Vec<Error>> {
    // parse into function
    let maybe_ast = syn::parse::<ItemFn>(input);
    let mut errors = vec![];

    // check if this parsed into an AST correctly
    if let Ok(ast) = maybe_ast {
        // perform checks
        if ast.sig.abi.is_some() {
            errors.push(syn::Error::new(
                ast.sig.span(),
                "ABI function cannot be tagged with `#[gpu_use]`",
            ));
            return Err(errors);
        }
        if ast.sig.asyncness.is_some() {
            errors.push(syn::Error::new(
                ast.sig.span(),
                "async function cannot be tagged with `#[gpu_use]`",
            ));
            return Err(errors);
        }

        Ok(FunctionInfo {
            name: ast.sig.ident,
            has_return: if let ReturnType::Default = ast.sig.output {
                false
            } else {
                true
            },
        })
    } else {
        Err(vec![Error::new(
            Span::call_site().unwrap().into(),
            "only functions that are items can be tagged with `#[gpu_use]`",
        )])
    }
}
