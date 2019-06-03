#![recursion_limit="256"]

// crates for parsing
extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

// used for containing macro input
use proc_macro::{TokenStream, Span};

// used for parsing macro input
use syn::parse::{Parse, ParseStream};
use syn::parse_macro_input;

// used for traversing AST
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::{
    braced, parenthesized, Attribute, BinOp, Block, Expr, Ident, Lit, Pat, Result, Stmt, Token,
    Type, UnOp,
};

// The Emu Identifier Prefix is appended to the start of each identifier found in Emu code except
// for identifiers that reference OpenCL constructs such as get_global_id
static EMU_IDENTIFIER_SUFFIX: &str = "_emu";

static OPENCL_FUNCTIONS: &'static [&'static str] = &[
    "get_work_dim",
    "get_global_size",
    "get_global_id",
    "get_local_size",
    "get_local_id",
    "get_num_groups()",
    "get_group_id()"
];

/// Represents an Emu program
struct EmuProgram {
    kernels: Vec<EmuKernel>,
}

/// Implementation of parser for Emu programs 
impl Parse for EmuProgram {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut kernels: Vec<EmuKernel> = Vec::new();

        while !input.is_empty() && input.lookahead1().peek(Ident) {
            let new_kernel = input.call(EmuKernel::parse)?;
            kernels.push(new_kernel);
        }

        // return new Emu program
        Ok(EmuProgram {
            kernels: kernels,
        })
    }
}

#[derive(Clone)]
struct EmuParameter {
    name: String,
    address_space: String,
    ty: String,
}

impl Parse for EmuParameter {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = String::from("buffer");
        let mut address_space = String::from("__global");
        let mut ty = String::from("float*");

        // get name of parameter
        let name_token: Ident = input.parse()?;
        name = name_token.to_string();

        // get address space of parameter
        if name.starts_with("global_") {
            address_space = String::from("__global");
        } else if name.starts_with("local_") {
            address_space = String::from("__local");
        } else {
            address_space = String::from("__private");
        }

        // get type of parameter
        let ty_token: Type = input.parse()?;
        ty = match ty_token {
            Type::Slice(ty_type) => {
                if let Type::Path(ty_type_path) = *ty_type.elem {
                    String::from(match ty_type_path.path.segments[0].ident.to_string().as_ref() {
                        "bool" => "bool",
                        "f32" => "float",
                        "i8" => "char",
                        "i16" => "short",
                        "i32" => "int",
                        "i64" => "long",
                        "u8" => "uchar",
                        "u16" => "ushort",
                        "u32" => "uint",
                        "u64" => "ulong",
                        _ => "float",
                    }) + "*"
                } else { String::new() }
            }
            Type::Path(ty_type_path) => {
                String::from(match ty_type_path.path.segments[0].ident.to_string().as_ref() {
                    "bool" => "bool",
                    "f32" => "float",
                    "i8" => "char",
                    "i16" => "short",
                    "i32" => "int",
                    "i64" => "long",
                    "u8" => "uchar",
                    "u16" => "ushort",
                    "u32" => "uint",
                    "u64" => "ulong",
                    _ => "float",
                })
            }
            _ => { String::new() }
        };

        Ok(EmuParameter{
            name: name,
            address_space: address_space,
            ty: ty,
        })
    }
}

/// Represents an Emu kerenl within an Emu program
struct EmuKernel {
    name: Ident,
    params: Vec<EmuParameter>,
    stmts: Vec<Stmt>,
    generated_return_type: String,
    generated_code: String,
}

/// Implementation of parser for Emu kernels
impl Parse for EmuKernel {
    fn parse(input: ParseStream) -> Result<Self> {
        // discard documentation comments
        let _ = input.call(Attribute::parse_outer)?;

        // get name of program
        let name: Ident = input.parse()?;

        // get punctuated parmeters
        let content_kernel;
        let _ = parenthesized!(content_kernel in input);
        let punctuated_parameters: Punctuated<EmuParameter, Token![,]> =
            content_kernel.parse_terminated(EmuParameter::parse)?;
        let punctuated_parameters_iter = punctuated_parameters.pairs();

        // get parameters
        let parameters: Vec<EmuParameter> = punctuated_parameters_iter
            .map(|parameter_pair| parameter_pair.into_value())
            .cloned()
            .collect::<Vec<EmuParameter>>();            

        // get return type name
        let mut generated_return_type = String::new();
        if input.lookahead1().peek(Ident) {
            let return_type: Ident = input.parse()?;
            generated_return_type = return_type.to_string();
        }

        // discard braces and documentation comments
        let content_block;
        let _ = braced!(content_block in input);
        let _ = content_block.call(Attribute::parse_inner)?;

        // get statements
        let statements = content_block.call(Block::parse_within)?;

        // return name and statements
        Ok(EmuKernel {
            name: name,
            params: parameters,
            stmts: statements,
            generated_return_type: generated_return_type,
            generated_code: String::new(),
        })
    }
}

/// Implementation of visitor for AST of Emu kernels
impl<'a> Visit<'a> for EmuKernel {
    fn visit_block(&mut self, b: &Block) {
        // visit each statement in block
        for statement in &b.stmts {
            self.visit_stmt(&statement);
        }
    }

    fn visit_stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Local(l) => {
                // look at first variable being declared in this let statement
                for (i, declaration) in l.pats.pairs().enumerate() {
                    // TODO allow multiple variables to be declared with a single let statement
                    if i > 0 {
                        break;
                    }

                    // generate code for type of variable being declared
                    if let Some(declaration_type) = l.ty.clone() {
                        if let Type::Path(declaration_type_path) = *declaration_type.1 {
                            self.generated_code += match declaration_type_path.path.segments[0]
                                .ident
                                .to_string()
                                .as_ref()
                            {
                                "bool" => "bool",
                                "f32" => "float",
                                "i8" => "char",
                                "i16" => "short",
                                "i32" => "int",
                                "i64" => "long",
                                "u8" => "uchar",
                                "u16" => "ushort",
                                "u32" => "uint",
                                "u64" => "ulong",
                                _ => "float",
                            }
                        }
                    }

                    // generate code for name of variable
                    self.generated_code += " ";
                    if let Pat::Ident(declaration_name_ident) = declaration.into_value() {
                        self.generated_code += &declaration_name_ident.ident.to_string();
                    }
                    self.generated_code += " = ";

                    // generate code for expression of initial value
                    if let Some(init) = l.init.clone() {
                        self.visit_expr(&init.1);
                    }
                }
                self.generated_code += ";";
            }
            Stmt::Semi(e, _) => {
                // visit all statements with expressions that end in semicolons
                self.visit_expr(e);
                self.generated_code += ";";
            }
            Stmt::Expr(e) => {
                // visit expressions that don't end in semicolons such as if statements and loops
                self.visit_expr(e);
            }
            _ => {}
        }
    }

    fn visit_expr(&mut self, e: &Expr) {
        match e {
            Expr::Assign(e) => {
                self.visit_expr(&e.left);
                self.generated_code += "=";
                self.visit_expr(&e.right);
            }
            Expr::AssignOp(e) => {
                self.visit_expr(&e.left);
                self.generated_code += match e.op {
                    BinOp::AddEq(_) => " += ",
                    BinOp::SubEq(_) => " -= ",
                    BinOp::MulEq(_) => " *= ",
                    BinOp::DivEq(_) => " /= ",
                    BinOp::RemEq(_) => " %= ",
                    BinOp::BitXorEq(_) => " &= ",
                    BinOp::BitAndEq(_) => " ^= ",
                    BinOp::ShlEq(_) => " <<= ",
                    BinOp::ShrEq(_) => " >>= ",
                    _ => "",
                };
                self.visit_expr(&e.right);
            }
            Expr::If(e) => {
                // generate code for if statement
                self.generated_code += "if (";
                self.visit_expr(&e.cond);
                self.generated_code += ") {";
                self.visit_block(&e.then_branch);
                self.generated_code += "}";

                // generate code for else branch
                if let Some(if_else_expr) = e.else_branch.clone() {
                    match *if_else_expr.1.clone() {
                        Expr::If(_) => {
                            self.generated_code += " else ";
                            self.visit_expr(&if_else_expr.1);
                        }
                        Expr::Block(if_else_block) => {
                            self.generated_code += " else {";
                            self.visit_block(&if_else_block.block);
                            self.generated_code += "}";
                        }
                        _ => {}
                    }
                }
            }
            Expr::ForLoop(e) => {
                // default values of for loop properties
                let mut for_var_name = String::new();

                self.generated_code += "for (int ";
                if let Pat::Ident(for_var_ident_pat) = *e.pat.clone() {
                    for_var_name = for_var_ident_pat.ident.to_string().clone();

                    self.generated_code += &for_var_ident_pat.ident.to_string();
                    self.generated_code += " = ";
                }
                if let Expr::Range(for_range_expr) = *e.expr.clone() {
                    if let Some(for_range_from_expr) = for_range_expr.from {
                        self.visit_expr(&for_range_from_expr);
                        self.generated_code += "; ";
                        self.generated_code += &for_var_name;
                        self.generated_code += " < ";
                    }
                    if let Some(for_range_to_expr) = for_range_expr.to {
                        self.visit_expr(&for_range_to_expr);
                        self.generated_code += "; ";
                        self.generated_code += &for_var_name;
                        self.generated_code += "++";
                    }
                }
                self.generated_code += ") {";
                self.visit_block(&e.body);
                self.generated_code += "}";
            }
            Expr::While(e) => {
                self.generated_code += "while (";
                self.visit_expr(&e.cond);
                self.generated_code += ") {";
                self.visit_block(&e.body);
                self.generated_code += "}";
            }
            Expr::Loop(e) => {
                self.generated_code += "while (0) {";
                self.visit_block(&e.body);
                self.generated_code += "}";
            }
            Expr::Index(e) => {
                self.visit_expr(&e.expr);
                self.generated_code += "[";
                self.visit_expr(&e.index);
                self.generated_code += "]";
            }
            Expr::Call(e) => {
                self.visit_expr(&e.func);
                self.generated_code += "(";
                for parameter in e.args.pairs() {
                    self.visit_expr(parameter.into_value());
                    self.generated_code += ",";
                }
                if e.args.pairs().len() > 0 { self.generated_code.truncate(self.generated_code.len() - 1); }
                self.generated_code += ")";
            }
            Expr::Unary(e) => {
                self.generated_code += match e.op {
                    UnOp::Deref(_) => "*",
                    UnOp::Not(_) => "!",
                    UnOp::Neg(_) => "-",
                };
                self.visit_expr(&e.expr);
            }
            Expr::Binary(e) => {
                self.visit_expr(&e.left);
                self.generated_code += match e.op {
                    BinOp::Add(_) => " + ",
                    BinOp::Sub(_) => " - ",
                    BinOp::Mul(_) => " * ",
                    BinOp::Div(_) => " / ",
                    BinOp::Rem(_) => " % ",
                    BinOp::And(_) => " && ",
                    BinOp::Or(_) => " || ",
                    BinOp::BitAnd(_) => " & ",
                    BinOp::BitOr(_) => " | ",
                    BinOp::BitXor(_) => " ^ ",
                    BinOp::Shl(_) => " >> ",
                    BinOp::Shr(_) => " << ",
                    BinOp::Lt(_) => " < ",
                    BinOp::Gt(_) => " > ",
                    BinOp::Le(_) => " <= ",
                    BinOp::Ge(_) => " >= ",
                    BinOp::Eq(_) => " == ",
                    BinOp::Ne(_) => " != ",
                    _ => "",
                };
                self.visit_expr(&e.right);
            }
            Expr::Cast(e) => {
                // TODO implement converting precision
                self.visit_expr(&e.expr);
                if let Type::Path(ty) = *e.ty.clone() {
                    if let Some(ty_prefix) = ty.path.segments[0].ident.to_string().chars().next() {
                        self.generated_code += String::from(match ty_prefix.to_string().as_ref() {
                            "Y" => "*10000000000",
                            "Z" => "*1000000000",
                            "E" => "*100000000",
                            "P" => "*10000000",
                            "T" => "*1000000",
                            "G" => "*100000",
                            "M" => "*10000",
                            "k" => "*1000",
                            "h" => "*100",
                            "D" => "*10",
                            "d" => "*0.1",
                            "c" => "*0.01",
                            "m" => "*0.001",
                            "u" => "*0.0001",
                            "n" => "*0.00001",
                            "p" => "*0.000001",
                            "f" => "*0.0000001",
                            "a" => "*0.00000001",
                            "z" => "*0.000000001",
                            "y" => "*0.0000000001",
                            _ => "*1",
                        }).as_ref();
                    }
                    
                }
            }
            Expr::Lit(e) => {
                let e_lit = e.lit.clone();
                if let Lit::Str(s) = e_lit {
                    self.generated_code += &s.value();
                } else if let Lit::Int(i) = e_lit {
                    self.generated_code += &i.value().to_string();
                } else if let Lit::Float(f) = e_lit {
                    self.generated_code += &f.value().to_string();
                } else if let Lit::Bool(b) = e_lit {
                    self.generated_code += if b.value { "true" } else { "false" }
                }
            }
            Expr::Path(e) => {
                // get raw name
                let raw_identifier_name = e.path.segments[0].ident.to_string();

                // TODO remove the below and OPENCL_FUNCTIONS global constant
                // // determine if this identifier is defined by the user or from OpenCL
                // let mut is_user_defined_identifier = true;

                // for OPENCL_FUNCTION in OPENCL_FUNCTIONS {
                //     if &&raw_identifier_name == OPENCL_FUNCTION {
                //         is_user_defined_identifier = false;
                //     }
                // }

                // // append suffix to end if identifier is uniquely defined by user
                // if is_user_defined_identifier {
                //     self.generated_code += &(raw_identifier_name);
                // } else {
                // }

                self.generated_code += match raw_identifier_name.to_string().as_ref() {
                    "PI"  => "3.141592653589793",
                    "PAU" => "4.71238898038",
                    "TAU" => "6.283185307179586",
                    "E"   => "2.718281828459045",
                    "PHI" => "1.618033988749894",
                    "G"   => "6.67408e-11",
                    "SG"  => "9.80665",
                    "C"   => "299792458.0",
                    "H"   => "6.62607004e-34",
                    "K"   => "1.38064852e-23",
                    "L"   => "6.02214086e23",
                    "MU0" => "0.00000125663",
                    "R"   => "8.314462618",

                    _     => &raw_identifier_name,
                };
            }
            Expr::Break(e) => {
                self.generated_code += "break";
            }
            Expr::Continue(e) => {
                self.generated_code += "continue";
            }
            Expr::Return(e) => {
                self.generated_code += "return ";
                if let Some(return_value) = &e.expr {
                    self.visit_expr(&*return_value);
                }
            }
            Expr::Paren(e) => {
                self.generated_code += "(";
                self.visit_expr(&e.expr);
                self.generated_code += ")";
            }
            _ => {}
        }
    }
}

/// The `emu!` macro allows you to define a kernel in the Emu language that can later be executed by work-items
#[proc_macro]
pub fn emu(tokens: TokenStream) -> TokenStream {
    // parse program
    let mut program = parse_macro_input!(tokens as EmuProgram);

    // iterate through kernels and generate code
    let mut generated_code = String::new();

    for mut kernel in program.kernels {
        if kernel.generated_return_type == "" {
            generated_code += "__kernel void ";
        } else {
            generated_code += match kernel.generated_return_type.as_str() {
                "bool" => "bool",
                "f32" => "float",
                "i8" => "char",
                "i16" => "short",
                "i32" => "int",
                "i64" => "long",
                "u8" => "uchar",
                "u16" => "ushort",
                "u32" => "uint",
                "u64" => "ulong",
                _ => "float",
            };
            generated_code += " ";
        }
        generated_code += &kernel.name.to_string();
        generated_code += " (";

        for parameter in kernel.params.clone() {
            generated_code += &parameter.address_space;
            generated_code += " ";
            generated_code += &parameter.ty;
            generated_code += " ";
            generated_code += &parameter.name;
            generated_code += ", ";
        }

        // remove last comma if one was appended
        if generated_code.ends_with(", ") {
            generated_code.truncate(generated_code.len() - 2)
        }
        generated_code += ") {";

        // traverse AST of each statement of parsed kernel
        // then, generate OpenCL code for body of kernel function from statements        
        let kernel_statements = kernel.stmts.clone();
        for statement in kernel_statements {
            kernel.visit_stmt(&statement);
        }
        generated_code += &kernel.generated_code;

        generated_code += "}";
    }

    // println!("{:?}", generated_code);

    // generate output Rust code
    let output = quote! {
        const EMU : &'static str = #generated_code;
    };

    // return output converted to token stream
    output.into()
}

/// Represents a parameter with three Strings; the coded name (e.g. - param_0 or param_1), the type, and whether or not it is a vector
struct Parameter(String, String, bool);

#[proc_macro]
pub fn build(tokens: TokenStream) -> TokenStream {
    let raw_input = tokens.to_string();
    let input = raw_input.split_whitespace().collect::<Vec<&str>>();

    // get name
    let kernel_name = input[0];
    let kernel_name_identifier = Ident::new(kernel_name, syn::export::Span::call_site());

    let mut parameters: Vec<Parameter> = vec![];
    let mut token_index = 0;
    let mut token_vector_index = 0;
    for token in &input {
        if token_index > 0 && input[token_index] != "[" && input[token_index] != "]" {
            if input[token_index - 1] == "[" {
                parameters.push(Parameter("param_".to_owned() + &token_vector_index.to_string(), input[token_index].to_string(), true))
            } else {
                parameters.push(Parameter("param_".to_owned() + &token_vector_index.to_string(), input[token_index].to_string(), false))
            }
            token_vector_index += 1;
        }

        token_index += 1;
    }

    let mut params = vec![];

    let mut params_index = 0;
    let mut new_param_index = 0;
    let mut result_param = "";
    let mut result_param_name = String::new();
    for param_type in &input {
        if params_index > 1 && input[params_index] != "[" && input[params_index] != "]" {
            if input[params_index - 1] == "[" {
                result_param = input[params_index];
                result_param_name = String::from("param_");
                result_param_name += &new_param_index.to_string();

                // parameter is vector
                let new_param_name = Ident::new((String::from("param_") + &new_param_index.to_string()).as_str(), syn::export::Span::call_site());
                let new_param_type = Ident::new(input[params_index], syn::export::Span::call_site());
                params.push(quote! { #new_param_name : Vec< #new_param_type > })
            } else {
                // parameter is scalar
                let new_param_name = Ident::new((String::from("param_") + &new_param_index.to_string()).as_str(), syn::export::Span::call_site());
                let new_param_type = Ident::new(input[params_index], syn::export::Span::call_site());
                params.push(quote! { #new_param_name : #new_param_type })
            }
            new_param_index += 1;
        }

        params_index += 1;
    }

    let result_param_type = Ident::new(result_param, syn::export::Span::call_site());
    let result_param_identifier = Ident::new(&result_param_name, syn::export::Span::call_site());

    let mut params_for_kernel_builder = vec![];
    let mut param_for_kernel_builder_index = 0;
    for parameter in &parameters {
        let mut param_for_kernel_builder = String::from(parameter.0.as_str());
        if parameter.2 {
            param_for_kernel_builder += "_buffer";
        }
        let param_for_kernel_builder_index_identifier = Ident::new(&param_for_kernel_builder, syn::export::Span::call_site());
        params_for_kernel_builder.push(quote! { .arg(& #param_for_kernel_builder_index_identifier ) });
        param_for_kernel_builder_index += 1;
    }

    let mut buffers = vec![];
    let mut result_buffer_name = String::new();
    for parameter in &parameters {
        if parameter.2 {
            let mut buffer_name = String::from(parameter.0.as_str());
            buffer_name += "_buffer";
            if result_buffer_name == "" { result_buffer_name = buffer_name.clone() }
            let buffer_type = String::from(parameter.1.as_str());
            let buffer_source = String::from(parameter.0.as_str());
            let buffer_name_identifier = Ident::new(&buffer_name, syn::export::Span::call_site());
            let buffer_type_identifier = Ident::new(&buffer_type, syn::export::Span::call_site());
            let buffer_source_identifier = Ident::new(&buffer_source, syn::export::Span::call_site());
            buffers.push(quote! {
                let #buffer_name_identifier = Buffer::< #buffer_type_identifier >::builder()
                    .queue(queue.clone())
                    .flags(flags::MEM_READ_WRITE)
                    .len(#buffer_source_identifier .len())
                    .copy_host_slice(& #buffer_source_identifier)
                    .build()?;
            })
        }
    }
    let result_buffer = Ident::new(&result_buffer_name, syn::export::Span::call_site());

    // generate output Rust code
    let output = quote! {
        fn #kernel_name_identifier ( #(#params),* ) -> ocl::Result<Vec< #result_param_type >>  {
            // (1) Define which platform and device(s) to use. Create a context,
            // queue, and program then define some dims (compare to step 1 above).
            let platform = Platform::default();
            let device = Device::first(platform)?;
            let context = Context::builder()
                .platform(platform)
                .devices(device.clone())
                .build()?;
            let program = Program::builder()
                .devices(device)
                .src(EMU)
                .build(&context)?;
            let queue = Queue::new(&context, device, None)?;
            let dims = #result_param_identifier .len();
            // [NOTE]: At this point we could manually assemble a ProQue by calling:
            // `ProQue::new(context, queue, program, Some(dims))`. One might want to
            // do this when only one program and queue are all that's needed. Wrapping
            // it up into a single struct makes passing it around simpler.

            // (2) Create a `Buffer`:
            #(#buffers)*

            // (3) Create a kernel with arguments matching those in the source above:
            let kernel = Kernel::builder()
                .program(&program)
                .name(#kernel_name)
                .queue(queue.clone())
                .global_work_size(dims)
                #(#params_for_kernel_builder)*
                .build()?;

            // (4) Run the kernel (default parameters shown for demonstration purposes):
            unsafe {
                kernel.cmd()
                    .queue(&queue)
                    .global_work_offset(kernel.default_global_work_offset())
                    .global_work_size(dims)
                    .local_work_size(kernel.default_local_work_size())
                    .enq()?;
            }

            // (5) Read results from the device into a vector (`::block` not shown):
            let mut vec = vec![0 as #result_param_type; dims as usize];
            #result_buffer .cmd()
                .queue(&queue)
                .offset(0)
                .read(&mut vec)
                .enq()?;

            Ok(vec)
        }
    };

    // println!("{:?}", output.to_string());

    // return output converted to token stream
    output.into()
}

// TODO
// precision conversions => https://www.khronos.org/registry/OpenCL/sdk/1.0/docs/man/xhtml/convert_T.html
// vectors => https://github.com/rsnemmen/OpenCL-examples/blob/master/RayTraced_Quaternion_Julia-Set_Example/qjulia_kernel.cl
// ensure identifier don't get mistaken for unsupported OpenCL keywords