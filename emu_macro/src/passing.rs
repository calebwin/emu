// for generating Rust
extern crate quote;
use quote::ToTokens;

// for procedural macros
extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::Span;

// for parsing Rust
extern crate syn;
use syn::fold::Fold;
use syn::*;

// for etc.
use std::result::Result;

// this was copied from standard library source code
// it is used for folding arbitrary items or exprs
macro_rules! fold_expr_default {
    ($f:expr, $node:expr) => {
        match $node {
            Expr::Array(_binding_0) => Expr::Array($f.fold_expr_array(_binding_0)),
            Expr::Assign(_binding_0) => Expr::Assign($f.fold_expr_assign(_binding_0)),
            Expr::AssignOp(_binding_0) => Expr::AssignOp($f.fold_expr_assign_op(_binding_0)),
            Expr::Async(_binding_0) => Expr::Async($f.fold_expr_async(_binding_0)),
            Expr::Await(_binding_0) => Expr::Await($f.fold_expr_await(_binding_0)),
            Expr::Binary(_binding_0) => Expr::Binary($f.fold_expr_binary(_binding_0)),
            Expr::Block(_binding_0) => Expr::Block($f.fold_expr_block(_binding_0)),
            Expr::Box(_binding_0) => Expr::Box($f.fold_expr_box(_binding_0)),
            Expr::Break(_binding_0) => Expr::Break($f.fold_expr_break(_binding_0)),
            Expr::Call(_binding_0) => Expr::Call($f.fold_expr_call(_binding_0)),
            Expr::Cast(_binding_0) => Expr::Cast($f.fold_expr_cast(_binding_0)),
            Expr::Closure(_binding_0) => Expr::Closure($f.fold_expr_closure(_binding_0)),
            Expr::Continue(_binding_0) => Expr::Continue($f.fold_expr_continue(_binding_0)),
            Expr::Field(_binding_0) => Expr::Field($f.fold_expr_field(_binding_0)),
            Expr::ForLoop(_binding_0) => Expr::ForLoop($f.fold_expr_for_loop(_binding_0)),
            Expr::Group(_binding_0) => Expr::Group($f.fold_expr_group(_binding_0)),
            Expr::If(_binding_0) => Expr::If($f.fold_expr_if(_binding_0)),
            Expr::Index(_binding_0) => Expr::Index($f.fold_expr_index(_binding_0)),
            Expr::Let(_binding_0) => Expr::Let($f.fold_expr_let(_binding_0)),
            Expr::Lit(_binding_0) => Expr::Lit($f.fold_expr_lit(_binding_0)),
            Expr::Loop(_binding_0) => Expr::Loop($f.fold_expr_loop(_binding_0)),
            Expr::Macro(_binding_0) => Expr::Macro($f.fold_expr_macro(_binding_0)),
            Expr::Match(_binding_0) => Expr::Match($f.fold_expr_match(_binding_0)),
            Expr::MethodCall(_binding_0) => Expr::MethodCall($f.fold_expr_method_call(_binding_0)),
            Expr::Paren(_binding_0) => Expr::Paren($f.fold_expr_paren(_binding_0)),
            Expr::Path(_binding_0) => Expr::Path($f.fold_expr_path(_binding_0)),
            Expr::Range(_binding_0) => Expr::Range($f.fold_expr_range(_binding_0)),
            Expr::Reference(_binding_0) => Expr::Reference($f.fold_expr_reference(_binding_0)),
            Expr::Repeat(_binding_0) => Expr::Repeat($f.fold_expr_repeat(_binding_0)),
            Expr::Return(_binding_0) => Expr::Return($f.fold_expr_return(_binding_0)),
            Expr::Struct(_binding_0) => Expr::Struct($f.fold_expr_struct(_binding_0)),
            Expr::Try(_binding_0) => Expr::Try($f.fold_expr_try(_binding_0)),
            Expr::TryBlock(_binding_0) => Expr::TryBlock($f.fold_expr_try_block(_binding_0)),
            Expr::Tuple(_binding_0) => Expr::Tuple($f.fold_expr_tuple(_binding_0)),
            Expr::Type(_binding_0) => Expr::Type($f.fold_expr_type(_binding_0)),
            Expr::Unary(_binding_0) => Expr::Unary($f.fold_expr_unary(_binding_0)),
            Expr::Unsafe(_binding_0) => Expr::Unsafe($f.fold_expr_unsafe(_binding_0)),
            Expr::Verbatim(_binding_0) => Expr::Verbatim(_binding_0),
            Expr::While(_binding_0) => Expr::While($f.fold_expr_while(_binding_0)),
            Expr::Yield(_binding_0) => Expr::Yield($f.fold_expr_yield(_binding_0)),
            _ => unreachable!(),
        }
    };
}

// what does it mean to be a function that is declared to be a helper function?
// well, it means that you need to accept the GPU as an argument and return it back to whoever called you
// the purpose of this module is to transform functions appropriately so this is exactly what happens
//
// this function plays a small part in this transformation of functions
// specifically, it will change the signature of the function appropriately
pub fn modify_signature_for_helper_function(
    input: TokenStream,
    has_return: bool,
) -> Result<TokenStream, Vec<Error>> {
    // parse into function
    let maybe_ast = syn::parse::<ItemFn>(input.clone());

    // there are 2 steps to this transformation
    // (1) modify the input to the function, in order to accept the GPU
    // (2) modify the output of the function, in order to return the GPU

    if let Ok(mut ast) = maybe_ast {
        // modify based on whether or not the function returns something already
        if has_return {
            // (1) modify input
            let input: proc_macro::TokenStream = quote! {
                mut gpu: Gpu
            }
            .into();
            ast.sig
                .inputs
                .insert(0, syn::parse::<FnArg>(input).unwrap()); // insert as parameter

            // (2) modify output
            if let ReturnType::Type(existing_output_arrow, existing_output_type) = ast.sig.output {
                let output = quote! {
                    #existing_output_arrow (#existing_output_type, Gpu)
                }
                .into_token_stream();
                ast.sig.output = syn::parse::<ReturnType>(output.into_token_stream().into())
                    .expect("could not change return type");
            } else {
                // we are already given at this point that there has to be a return so this case should not happen
                // that is because ReturnType can only either be Default or Type
            }
        } else {
            // (1) modify input
            let input = quote! {
                mut gpu: Gpu
            }
            .into();
            ast.sig
                .inputs
                .insert(0, syn::parse::<FnArg>(input).unwrap());

            // (2) modify output
            // note that the GPU is the second argument
            // this is because when we return we want to first evaluate the existing body of the function
            // which might mutate the GPU. then only do we return the GPU after all the mutations have happened
            let output = quote! {
                -> ((), Gpu)
            }
            .into();
            ast.sig.output = syn::parse::<ReturnType>(output).unwrap();
        }

        // return the modified input
        Ok(ast.to_token_stream().into())
    } else {
        Err(vec![Error::new(
            Span::call_site().unwrap().into(),
            "only functions that are items can be tagged with `#[gpu_use]`",
        )])
    }
}

// TODO handle question mark operator
// we can do one of several things
// - forbid ?
// - handle by expanding syntactic sugar
// - handle by looking for Result in signature and modifying the Ok part of it

// modifies return expression
// note this doesn't fix up all the return statements only the implicit "last expression is returned" stuff
// we'll deal with the return statements later
pub fn modify_return_for_helper_function(
    input: TokenStream,
    has_return: bool,
) -> Result<TokenStream, Vec<Error>> {
    // parse into function
    let maybe_ast = syn::parse::<ItemFn>(input.clone());

    if let Ok(mut ast) = maybe_ast {
        if has_return {
            let existing_body = ast.block;
            // we just change the body so we first evaluate what we normally have there
            // and then we return the GPU
            let body = quote! {
                {
                    (#existing_body, gpu)
                }
            };
            ast.block = Box::new(
                syn::parse::<Block>(body.into_token_stream().into())
                    .expect("could not change returns"),
            );
        } else {
            let existing_body = ast.block;
            // if no return existed, it's a bit different what we do here
            let body = quote! {
                {
                    #existing_body
                    ((), gpu)
                }
            };
            ast.block = Box::new(
                syn::parse::<Block>(body.into_token_stream().into())
                    .expect("could not change returns"),
            );
        }

        // return the modified input
        Ok(ast.to_token_stream().into())
    } else {
        Err(vec![Error::new(
            Span::call_site().unwrap().into(),
            "only functions that are items can be tagged with `#[gpu_use]`",
        )])
    }
}

// this is what we use to modify the return statements
// we want to modify the return statements so that they return the GPU
pub struct HelperFunctionReturnModifier;

impl Fold for HelperFunctionReturnModifier {
    fn fold_expr_return(&mut self, i: ExprReturn) -> ExprReturn {
        let attrs = i.attrs;
        let return_token = i.return_token;
        let expr = i.expr;

        let new_code = if expr.is_none() {
            quote! {
                #(#attrs)*
                #return_token ((), gpu)
            }
        } else {
            quote! {
                #(#attrs)*
                #return_token (#expr, gpu)
            }
        };

        let new_ast = syn::parse_str::<ExprReturn>(&new_code.to_string())
            .expect("could not modify return statements");

        new_ast
    }

    // don't fold on substructures of items
    // closures can't contain return statements that return from this function
    fn fold_expr_closure(&mut self, i: ExprClosure) -> ExprClosure {
        i
    }

    // don't fold on substructures of items
    // items can't contain return statements that will return from this function
    fn fold_item(&mut self, i: Item) -> Item {
        i
    }
}

// modifies return statements
// this mainly just creates an instance of the above "folder" that we defined
// we then just invoke it's "fold_item_fn" method to fold on the function
pub fn modify_returns_for_helper_function(input: TokenStream) -> Result<TokenStream, Vec<Error>> {
    // parse into function
    let maybe_ast = syn::parse::<ItemFn>(input.clone());

    if let Ok(ast) = maybe_ast {
        // make helper function return modifier
        let mut helper_function_return_modifier = HelperFunctionReturnModifier {};

        // transform AST with changes to return statements
        let new_ast = helper_function_return_modifier.fold_item_fn(ast);

        // return the modified input
        Ok(new_ast.to_token_stream().into())
    } else {
        Err(vec![Error::new(
            Span::call_site().unwrap().into(),
            "only functions that are items can be tagged with `#[gpu_use]`",
        )])
    }
}

// modifies body of a not-a-helper function
//
// when is something a not-a-helper?
// it's not a helper when it's #[gpu_use(...)] doesn't list itself as a helper function
// so functions tagged #[gpu_use] would also be considered not helper functions
//
// when a function is not a helper function, it is the place where we CREATE the GPU
// so we must modify it to do that
// note that while we don't need to modify it's input and output we must still modify how it
// invokes all the helper functions it invokes. those invocations must be modified to pass the GPU out
// and bring it back in
pub fn modify_for_not_a_helper_function(input: TokenStream) -> Result<TokenStream, Vec<Error>> {
    // parse into function
    let maybe_ast = syn::parse::<ItemFn>(input.clone());

    if let Ok(mut ast) = maybe_ast {
        let existing_body = ast.block;
        let body = quote! {
            {
                use ocl::*;

                let mut gpu = {
                    let new_platform = ocl::Platform::default();
                    let new_device = ocl::Device::first(new_platform).expect("no GPU found");
                    let new_context = ocl::Context::builder()
                        .platform(new_platform)
                        .devices(new_device.clone())
                        .build()
                        .expect("failed to build context for executing on GPU with OpenCL");
                    let new_queue = ocl::Queue::new(&new_context, new_device, None)
                        .expect("failed to create queue of commands to be sent to GPU");

                    Gpu {
                        device: new_device,
                        context: new_context,
                        queue: new_queue,
                        buffers: std::collections::HashMap::new(),
                        programs: std::collections::HashMap::new()
                    }
                };

                #existing_body
            }
        };
        ast.block = Box::new(
            syn::parse::<Block>(body.into_token_stream().into())
                .expect("could not add boilerplate code for initialization of GPU"),
        );

        // return the modified input
        Ok(ast.to_token_stream().into())
    } else {
        Err(vec![Error::new(
            Span::call_site().unwrap().into(),
            "only functions that are items can be tagged with `#[gpu_use]`",
        )])
    }
}

// looks through a function for all invocations of given helper functions
// it will then make sure that those functions have the GPU passed to them
// and the GPU they return is recieved
pub struct HelperFunctionInvocationModifier {
    pub helper_functions: Vec<Ident>,
}

impl Fold for HelperFunctionInvocationModifier {
    fn fold_expr(&mut self, ii: Expr) -> Expr {
        // TODO look at attrs and qself to know if this is a node we can actually work with

        if let Expr::Call(mut i) = ii {
            if let Expr::Path(path) = *i.func.clone() {
                let mut is_helper_function_invocation = false;

                for helper_function in &self.helper_functions {
                    if path.path.is_ident(helper_function) {
                        is_helper_function_invocation = true;
                    }
                }

                if is_helper_function_invocation {
                    let gpu_ident = quote! {gpu}.to_token_stream();
                    i.args.insert(0, syn::Expr::Verbatim(gpu_ident));

                    let new_code = quote! {
                        {
                            // get result
                            let result = #i;

                            // update GPU to new state
                            gpu = result.1;

                            // return result
                            result.0
                        }
                    };

                    let new_ast = syn::parse_str::<Expr>(&new_code.to_string())
                        .expect("could not modify invocations of helper functions");

                    new_ast
                } else {
                    fold_expr_default!(self, i.into())
                }
            } else {
                fold_expr_default!(self, i.into())
            }
        } else {
            fold_expr_default!(self, ii)
        }
    }

    // TODO handle functions items defined inside that have names that shadow a helper function
    // invocations of function of same name should not be transformed because they are now referencing a function
    // that isn't a helper function

    // don't fold on substructures of items
    // items can't use GPU argument to the function the item is in
    fn fold_item(&mut self, i: Item) -> Item {
        i
    }
}

// this just uses the HelperFunctionInvocationModifier defined above
pub fn modify_invocations(
    input: TokenStream,
    helper_functions: Vec<Ident>,
) -> Result<TokenStream, Vec<Error>> {
    // parse into function
    let maybe_ast = syn::parse::<ItemFn>(input.clone());

    if let Ok(ast) = maybe_ast {
        // make helper function invocation modifier
        let mut helper_function_invocation_modifier = HelperFunctionInvocationModifier {
            helper_functions: helper_functions,
        };

        // transform AST with changes to return statements
        let new_ast = helper_function_invocation_modifier.fold_item_fn(ast);

        // return the modified input
        Ok(new_ast.to_token_stream().into())
    } else {
        Err(vec![Error::new(
            Span::call_site().unwrap().into(),
            "only functions that are items can be tagged with `#[gpu_use]`",
        )])
    }
}
