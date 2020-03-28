// for generating Rust
extern crate quote;
use quote::ToTokens;

// for procedural macros
extern crate proc_macro;

// for parsing Rust
extern crate syn;
use syn::fold::Fold;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::*;
use proc_macro2::Span;

// for etc.use crate::generator::Generator;
use crate::generator::Generator;
use crate::identifier::get_global_work_size;
use crate::identifier::Dim;

// there is passing
// then there is accelerating
//
// accelerating is what looks through a function tagged with #[gpu_use] and
// finds invocation of gpu_do!() and interprets them by making the appropriate
// code transformations
pub struct Accelerator {
    pub ready_to_launch: bool, // whether or not we are yet ready to launch
    pub errors: Vec<Error>,    // errors that we collect through accelerating
}

impl Accelerator {
    pub fn new() -> Self {
        Self {
            ready_to_launch: false,
            errors: vec![],
        }
    }
}

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

// TODO document that we can't handle macros because we can't expand them at compile-time from here
impl Fold for Accelerator {
    #[allow(irrefutable_let_patterns)]
    fn fold_expr(&mut self, ii: Expr) -> Expr {
        // TODO look at attrs and qself to know if this is a node we can actually work with

        match ii.clone() {
            // transform macros into calls to OpenCL to transfer data
            // don't try to fold on substructure of macro
            // unless macro is something we can work with, just leave it alone
            Expr::Macro(i) => {
                let call_expr = syn::parse::<ExprCall>(i.mac.tokens.into());

                // we only want to look at macros where the contents of the macro is a call
                if call_expr.is_err() {
                    ii
                } else {
                    let call = call_expr.unwrap();

                    // we want to see what the thing being called is
                    if let Expr::Path(path) = *call.func {
                        // what are the arguments of the call
                        let arg = call.args.first();
                        let arg_literal = if let Some(arg_unwrapped) = arg {
                            Some(arg_unwrapped.to_token_stream().to_string())
                        } else {
                            None
                        };

                        // what is being called?
                        // is it load? read? launch?
                        if path
                            .path
                            .is_ident(&Ident::new("load", Span::call_site()))
                        {
                            let new_code = quote! {
                                {
                                    let hash = (#arg).as_slice() as *const [f32];
                                    // if hash is already key, copy_host_slice to existing buffer
                                    // else, create new buffer
                                    if gpu.buffers.contains_key(&hash) {
                                        gpu
                                            .buffers
                                            .get(&hash)
                                            .unwrap()
                                            .cmd()
                                            .queue(&gpu.queue)
                                            .offset(0)
                                            .write((#arg).as_slice())
                                            .enq().expect(&format!("failed to load `{}` to GPU", #arg_literal).as_str());
                                    } else {
                                        let _: &[f32] = (#arg).as_slice();
                                        gpu.buffers.insert(
                                            hash,
                                            ocl::Buffer::<f32>::builder()
                                                .queue(gpu.queue.clone())
                                                .flags(ocl::flags::MEM_READ_WRITE)
                                                .len({
                                                    let length = (#arg).len();
                                                    if length == 0 {
                                                        panic!("`{}` cannot be empty", #arg_literal)
                                                    } else {
                                                        length
                                                    }
                                                })
                                                .copy_host_slice((#arg).as_slice())
                                                .build()
                                                .expect(&format!("failed to load `{}` to GPU", #arg_literal).as_str())
                                        );
                                    }
                                }
                            };

                            let new_ast = syn::parse_str::<Expr>(&new_code.to_string())
                                .expect("could not generate call to OpenCL API to launch kernel");

                            new_ast
                        } else if path
                            .path
                            .is_ident(&Ident::new("read", Span::call_site()))
                        {
                            let new_code = quote! {
                                {
                                    let hash = (#arg).as_slice() as *const [f32];

                                    gpu
                                        .buffers
                                        .get(&hash)
                                        .expect(&format!("`{}` not loaded to GPU", #arg_literal).as_str())
                                        .cmd()
                                        .queue(&gpu.queue)
                                        .offset(0)
                                        .read((#arg).as_mut_slice())
                                        .enq().expect(&format!("failed to read `{}` from GPU", #arg_literal).as_str());
                                }
                            };

                            let new_ast = syn::parse_str::<Expr>(&new_code.to_string())
                                .expect("could not generate call to OpenCL API to launch kernel");

                            new_ast
                        } else if path
                            .path
                            .is_ident(&Ident::new("launch", Span::call_site()))
                        {
                            self.ready_to_launch = true;

                            // just return the macro invocation
                            ii
                        } else {
                            ii
                        }
                    } else {
                        ii
                    }
                }
            }
            // transform for loops into calls to OpenCL to launch kernels
            Expr::ForLoop(i) => {
                if !self.ready_to_launch {
                    // if we find a for loop, we only fold if we are not yet ready to launch
                    // if we are ready to launch, this better be a proper for loop
                    // and if it isn't a proper for loop, we will just leave it as it is and report errors
                    return fold_expr_default!(self, Expr::ForLoop(i.clone()));
                } else {
                    self.ready_to_launch = false;
                }

                // attempt to get global work size of the kernel to be launched
                let (global_work_size_dims, block_for_kernel) =
                    get_global_work_size(vec![], i.clone());
                let global_work_size = global_work_size_dims
                    .iter()
                    .map(|dim| {
                        if let Dim::RangeFromZero(_var, size) = dim {
                            *size
                        } else {
                            0
                        }
                    })
                    .collect::<Vec<_>>();

                // if there is no global work size, fold on substructures
                // if there is no kernel found, fold on substructures
                // otherwise keep going and attempt to generate program, args for kernel
                if global_work_size.len() == 0 || block_for_kernel.is_none() {
                    // if this is not for loop that belongs to well-defined well-documented set of for loops we can work with,
                    // then just pretend we didn't see it and keep moving on
                    self.errors
                        .push(Error::new(i.span(), "unexpected kind of for loop"));
                    return i.into();
                }

                // (a) generate program
                // we use the generator here
                let block = block_for_kernel.unwrap();
                let mut code_generator = Generator::from(global_work_size_dims);
                code_generator.visit_block(&block);
                self.errors.append(&mut code_generator.errors);
                if code_generator.failed_to_generate {
                    // on failing, we just fold on the inside
                    // TODO maybe don't fold because we were supposed to launch but couldn't
                    // maybe we need to just retur nerrors here
                    return fold_expr_default!(self, Expr::ForLoop(i.clone()));
                }
                let program = code_generator.code;

                // (b) generate arguments
                let args = code_generator.params.iter().map(|param| {
                    let ident = Ident::new(&param.name, Span::call_site());
                    let ident_literal = ident.to_string().clone();

                    if param.is_array {
                        quote! {
                            .arg(
                                gpu
                                    .buffers
                                    .get(&((#ident).as_slice() as *const [f32]))
                                    .expect(format!("`{}` not loaded to GPU", #ident_literal).as_str())
                            )
                        }
                    } else {
                        quote! {
                            .arg(&#ident)
                        }
                    }
                }).collect::<Vec<_>>();

                // (c) generate code
                let new_code = quote! {
                    {
                        let __main__ = || {
                            #i
                        };

                        let program_from = String::from(#program);

                        if gpu.programs.contains_key(&program_from) {

                            let kernel = ocl::Kernel::builder()
                                .program(gpu.programs.get(&program_from).unwrap())
                                .name("__main__")
                                .queue(gpu.queue.clone())
                                .global_work_size([#(#global_work_size),*])
                                #(#args)*
                                .build().expect("failed to compile kernel from program to be run on GPU");

                            unsafe {
                                kernel.cmd()
                                    .queue(&gpu.queue)
                                    .global_work_offset(kernel.default_global_work_offset())
                                    .global_work_size([#(#global_work_size),*])
                                    .local_work_size(kernel.default_local_work_size())
                                    .enq().expect("failed to run compiled kernel on GPU");
                            }
                        } else {
                            let program = ocl::Program::builder()
                                    .devices(gpu.device)
                                    .src(&program_from)
                                    .build(&gpu.context).expect("failed to compile program to be run on GPU");

                            let kernel = ocl::Kernel::builder()
                                .program(&program)
                                .name("__main__")
                                .queue(gpu.queue.clone())
                                .global_work_size([#(#global_work_size),*])
                                #(#args)*
                                .build().expect("failed to compile kernel from program to be run on GPU");

                            unsafe {
                                kernel.cmd()
                                    .queue(&gpu.queue)
                                    .global_work_offset(kernel.default_global_work_offset())
                                    .global_work_size([#(#global_work_size),*])
                                    .local_work_size(kernel.default_local_work_size())
                                    .enq().expect("failed to run compiled kernel on GPU");
                            }

                            gpu.programs.insert(program_from, program);
                        }


                    }
                };

                let new_ast = syn::parse_str::<Expr>(&new_code.to_string())
                    .expect("could not generate call to OpenCL API to launch kernel");

                new_ast
            }
            _ => {
                if self.ready_to_launch {
                    self.errors.push(syn::Error::new(
                        ii.span(),
                        "expected `gpu_do!(launch())` to be followed by a for loop",
                    ));
                    self.ready_to_launch = false;
                    ii
                } else {
                    // we fold, of course, because there might be stuff using the GPU inside here
                    fold_expr_default!(self, ii)
                }
            }
        }
    }

    // don't fold on substructures of items
    // items can't use GPU argument to the function the item is in
    fn fold_item(&mut self, i: Item) -> Item {
        if self.ready_to_launch {
            self.errors.push(syn::Error::new(
                i.span(),
                "expected `gpu_do!(launch())` to be followed by a for loop",
            ));
            self.ready_to_launch = false;
        }

        i
    }

    fn fold_local(&mut self, mut l: Local) -> Local {
        if self.ready_to_launch {
            self.errors.push(syn::Error::new(
                l.span(),
                "expected `gpu_do!(launch())` to be followed by a for loop",
            ));
            self.ready_to_launch = false;
            l
        } else {
            // here we DO fold on substructures of this let statement
            // because let statement could be assigning a value where that value
            // is a block expression. in that case, we want to look at the block
            // expression in case that uses the GPU for stuff
            let mut new_expr = None;
            if let Some(mut expr) = l.init.clone() {
                expr.1 = Box::new(fold_expr_default!(self, *(expr.1)));
                new_expr = Some(expr);
            }

            if let Some(_expr) = new_expr.clone() {
                l.init = new_expr;
            }
            l
        }
    }
}
