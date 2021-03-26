//! `emu_glsl` is a crate for GLSL-Rust interop. Currently, it only provides
//! a single derive macro - `glsl_struct`. This macro derives a trait that
//! is defined in the `emu_core` crate - `GlslStruct`. This is what the trait
//! looks like.
//! ```
//! pub trait GlslStruct {
//!     fn as_glsl() -> String; // return the GLSL struct definition of Self
//! }
//! ```
//! `emu_glsl` lets you derive this trait for simple structures where each
//! field is one of the following.
//! - `bool`
//! - `i32`
//! - `u32`
//! - `f32`
//! - `f64`
//! - `[i32 | u32 | f32 | f64 | bool; 2 | 3 | 4]`
//!
//! These get straightforwardly translated to their GLSL equivalents with
//! the arrays being translated to GLSL "vector data types". An example usage
//! is the following. (It doesn't compile as is because it's missing imports for the
//! `GlslStruct` trait and `glsl_struct` derive macro.)
//! ```rust,compile_fail
//! #[derive(GlslStruct)]
//! struct Polygon {
//!     num_edges: u32,
//!     radius: f64,
//!     conv: bool, // make sure polygons in same thread block have same convexity
//! }
//! ```

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

fn rust_to_glsl(rust: String) -> String {
    String::from(match rust.as_ref() {
        "bool" => "bool",
        "i32" => "int",
        "u32" => "uint",
        "f32" => "float",
        "f64" => "double",
        _ => &rust,
    })
}

#[proc_macro_derive(GlslStruct)]
pub fn glsl_struct(input: TokenStream) -> TokenStream {
    // parse and get name of struct
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // generate GLSL code
    let mut glsl = String::from("struct ");
    glsl += &name.to_string();
    glsl += " {";
    if let Data::Struct(struct_data) = input.data {
        if let Fields::Named(named_fields) = struct_data.fields {
            // generate code for each field
            for field in named_fields.named.iter() {
                // generate code for the field's type
                glsl += &(match &field.ty {
                    // TODO add support for more features
                    Type::Path(type_path) => {
                        rust_to_glsl(type_path.path.get_ident().unwrap().to_string())
                    }
                    Type::Array(type_array) => {
                        let mut type_prefix =
                            rust_to_glsl(type_array.elem.to_token_stream().to_string())
                                .chars()
                                .next()
                                .unwrap()
                                .to_string();
                        if type_prefix == String::from("f") {
                            type_prefix.clear();
                        }
                        match type_array.len.to_token_stream().to_string().as_str() {
                            "2" => type_prefix + "vec2",
                            "3" => type_prefix + "vec3",
                            "4" => type_prefix + "vec4",
                            _ => rust_to_glsl(field.ty.to_token_stream().to_string()),
                        }
                    }
                    _ => rust_to_glsl(field.ty.to_token_stream().to_string()),
                });
                glsl += " ";
                glsl += &field
                    .ident
                    .as_ref()
                    .expect("field must have an identifier")
                    .to_string();
                glsl += "; "
            }
        } else {
            panic!("expected a struct with named fields");
        }
    } else {
        panic!("expected a struct");
    }
    glsl += " };";

    // create Rust code for implementation with GLSL code embedded
    let expanded = quote! {
        impl GlslStruct for #name {
            fn as_glsl() -> String {
                String::from(#glsl)
            }
        }
    };

    // return Rust code as TokenStream
    TokenStream::from(expanded)
}
