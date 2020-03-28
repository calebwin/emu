extern crate proc_macro;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Fields};

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
	// parse
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // create glsl
    let mut glsl = String::from("struct ");
    glsl += &name.to_string();
    glsl += " {";
    if let Data::Struct(struct_data) = input.data {
        if let Fields::Named(named_fields) = struct_data.fields {
            for field in named_fields.named.iter() {
                glsl += &rust_to_glsl(field.ty.to_token_stream().to_string());
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

    // create rust
    let expanded = quote! {
        impl GlslStruct for #name {
            fn as_glsl() -> String {
                String::from(#glsl)
            }
        }
    };

    // return
    TokenStream::from(expanded)
}
