// for parsing Rust
extern crate syn;
use syn::*;

// this represents a dimension
//
// every kernel corresponds to a multi-dimensional space
// this multi-dimensional space has a specific integer size of each dimension that may be
// a literal known at compile-time (like 1000) or an expression that is
// evaluated at compile-time (like data.len())
//
// but how is this multi-dimensional space used by its corresponding kernel?
// when the kernel is run, the code that corresponds with the kernel is run
// not just once but many many times
// in particular, the code is run once for every position in the multi-dimensional space
// the kernel is usually able to get it's position through a built-in function that can be called
// like get_global_id(x) where x is the dimension you want to know your position in (either 0 or 1 or 2)
#[derive(Debug, Clone)]
pub enum Dim {
    RangeFromZero(String, i32), // TODO add support for iteration over &mut [f32], [f32], etc.
}

// tries to identify dimensions of global work for for loop and nested for loops
// returns empty Vec if the for loops wrapping inside kernel that it finds don't match what it expects
// returns None for the block if no dimensions found
//
// this is recursive so it might be a bit hard to follow
// because we will never recurse more than 3 times, maybe there is a simpler way?
// maybe something sequential?
// that might be cool...
//
// also, note that kernels are only a low-level concept of low-level GPGPU frameworks like OpenCL
// and CUDA; it should not be used in documentation for Emu because it just isn't a concept
// Emu looks at portions of code that have the structure of nested for loops and then (1) looks at the for loops
// to get metedata (like dimensions of global work size aka the multi-dimensional space kernels are associated
// with) and (2) looks at a for loop body for the kernel code
#[allow(unused_assignments)]
pub fn get_global_work_size(
    mut global_work_size: Vec<Dim>,
    i: ExprForLoop,
) -> (Vec<Dim>, Option<Block>) {
    // can't have more than 3 dimensions of the work size space that is associated with a kernel
    // so we just fail early in that case
    if global_work_size.len() == 3 {
        return (global_work_size, None);
    }

    // look at current for loop to see if new dimension can be appended
    let mut new_global_work_size_var = None;
    let mut new_global_work_size = None;

    // we can't have labels on the for loop
    if i.label.is_some() {
        return (global_work_size, None);
    }

    // must be for i in [something here] {}
    // so if i is not an identifier, we also fail early here
    if let Pat::Ident(ident) = i.pat {
        if ident.by_ref.is_none() && ident.mutability.is_none() && ident.subpat.is_none() {
            // use ident to say mapping of variable -> values in series
            new_global_work_size_var = Some(ident.ident.to_string());
        } else {
            return (global_work_size, None);
        }
    } else {
        return (global_work_size, None);
    }

    // now we look at the expr (which currently must be a range)
    // there are many different kinds of ranges you could have
    // so we try to find one specific kind
    //
    // this is a giant nested expression which can be intimidating...
    // but it is really just a bunch of if's to check if this is really the
    // kind of expr we want
    if let Expr::Range(range) = *i.expr {
        if let Some(from) = range.from {
            if let Some(to) = range.to {
                if let Expr::Lit(from_lit) = *from {
                    if let Expr::Lit(to_lit) = *to {
                        if let Lit::Int(from_lit_int) = from_lit.lit {
                            if let Lit::Int(to_lit_int) = to_lit.lit {
                                let from_val_raw = from_lit_int.base10_parse::<i32>();
                                let to_val_raw = to_lit_int.base10_parse::<i32>();

                                if let Ok(from_val) = from_val_raw {
                                    if let Ok(to_val) = to_val_raw {
                                        if from_val == 0 && from_val < to_val {
                                            if let Some(var) = new_global_work_size_var {
                                                // this is a case of a for loop we can work with
                                                // so we go ahead and see if further recursion can be done on the for loop body

                                                // add new global work size
                                                new_global_work_size = Some(to_val - from_val);
                                                global_work_size.push(Dim::RangeFromZero(
                                                    var,
                                                    new_global_work_size.unwrap(),
                                                ));

                                                // look at body for potential new global work sizes for further recursion
                                                if i.body.stmts.len() == 1 {
                                                    match &i.body.stmts[0] {
                                                        // we should handle both cases of Expr(expr) or Semi(expr, _) exactly the same
                                                        // either way we check for a for loop inside the passed in for loop
                                                        // if one exists we return the new global work size and new body
                                                        // otherwise we return the new global work size (which wouldn't have changed) and the body of the passed in for loop
                                                        Stmt::Expr(expr) => {
                                                            if let Expr::ForLoop(for_expr) = expr {
                                                                let (
                                                                    new_global_work_size,
                                                                    block_for_kernel,
                                                                ) = get_global_work_size(
                                                                    global_work_size,
                                                                    for_expr.clone(),
                                                                );
                                                                if block_for_kernel.is_none() {
                                                                    return (
                                                                        new_global_work_size,
                                                                        Some(i.body),
                                                                    );
                                                                } else {
                                                                    return (
                                                                        new_global_work_size,
                                                                        block_for_kernel,
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        Stmt::Semi(expr, _) => {
                                                            if let Expr::ForLoop(for_expr) = expr {
                                                                let (
                                                                    new_global_work_size,
                                                                    block_for_kernel,
                                                                ) = get_global_work_size(
                                                                    global_work_size,
                                                                    for_expr.clone(),
                                                                );
                                                                if block_for_kernel.is_none() {
                                                                    return (
                                                                        new_global_work_size,
                                                                        Some(i.body),
                                                                    );
                                                                } else {
                                                                    return (
                                                                        new_global_work_size,
                                                                        block_for_kernel,
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        _ => {}
                                                    }
                                                }

                                                return (global_work_size, Some(i.body));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // if we didn't find a valid expr (like for x in expr), then we just fail
    // handling more kinds of exprs can be done by adding handling for that case
    // in an if statement (or something similar) above this
    (global_work_size, None)
}
