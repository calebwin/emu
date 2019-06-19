#![recursion_limit="256"]

// useful output gets printed when debug is true
// debug should be false for production builds
const DEBUG: bool = false;

// for generating Rust
#[macro_use]
extern crate quote;

// for procedural macros
extern crate proc_macro;
use proc_macro::{TokenStream};

// for parsing Rust
extern crate syn;
use syn::parse::{Parse, ParseStream};
use syn::parse_macro_input;
use syn::export::Span;
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::{
    braced, parenthesized, Attribute, BinOp, Block, Expr, Ident, Lit, Pat, Result, Stmt, Token,
    Type, UnOp, ForeignItemFn, FnDecl, Visibility, FnArg, GenericArgument,
    PathArguments
};

#[derive(Clone)]
enum EmuType {
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Null
}

#[derive(Clone)]
struct EmuFunctionParameter {
    name: String,
    is_vector: bool,
    ty: EmuType
}

impl Parse for EmuFunctionParameter {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = String::new();
        let mut is_vector = false;
        let mut ty = EmuType::Null;

        // get name of parameter
        let name_token: Ident = input.parse()?;
        name = name_token.to_string();

        // get type of parameter
        let ty_token: Type = input.parse()?;
        match ty_token {
            Type::Slice(ty_type) => {
                is_vector = true;
                if let Type::Path(ty_type_path) = *ty_type.elem {
                    ty = emu_to_emu_type(&ty_type_path.path.segments[0].ident.to_string());
                    if let EmuType::Null = ty {
                        panic!("Invalid type of vector as parameter in Emu function definition");
                    }
                } else {
                    panic!("Invalid type of vector as parameter in Emu function definition");
                }
            }
            Type::Path(ty_type_path) => {
                is_vector = false;
                ty = emu_to_emu_type(&ty_type_path.path.segments[0].ident.to_string());
                if let EmuType::Null = ty {
                    panic!("Invalid type of scalar as parameter in Emu function definition");
                }
            }
            _ => {
                panic!("Invalid type of parameter in Emu function definition");
            }
        };

        Ok(EmuFunctionParameter{
            name: name,
            is_vector: is_vector,
            ty: ty,
        })
    }
}

struct EmuFunctionBody {
    generated_code: String,
    dimensionality_parameters: Vec<String>
}


impl<'a> Visit<'a> for EmuFunctionBody {
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
                    if i > 0 {
                        panic!("Expected only one identifier on left-hand side of \"let\" statement");
                    }

                    // generate code for type of variable being declared
                    if let Some(declaration_type) = l.ty.clone() {
                        if let Type::Path(declaration_type_path) = *declaration_type.1 {
                            if declaration_type_path.path.segments.len() != 1 {
                                panic!("Invalid type on left-hand side of \"let\" statement");
                            }

                            let type_path = declaration_type_path.path.segments[0].ident.to_string();
                            let opencl_type = emu_to_opencl(&type_path);
                            match opencl_type.clone() {
                                "null" => {
                                    panic!("Invalid type on left-hand side of \"let\" statement");
                                },
                                _ => { self.generated_code += opencl_type; }
                            }
                        } else {
                            panic!("Invalid type on left-hand side of \"let\" statement");
                        }
                    } else {
                        panic!("Invalid type on left-hand side of \"let\" statement");
                    }

                    // generate code for name of variable
                    self.generated_code += " ";
                    if let Pat::Ident(declaration_name_ident) = declaration.into_value() {
                        self.generated_code += &declaration_name_ident.ident.to_string();
                    } else {
                        panic!("Expected identifier on left-hand side of \"let\" statement");
                    }
                    self.generated_code += " = ";

                    // generate code for expression of initial value
                    if let Some(init) = l.init.clone() {
                        self.visit_expr(&init.1);
                    } else {
                        panic!("Expected right-hand side of \"let\" statement");
                    }

                    self.generated_code += ";\n";
                }
            }
            Stmt::Semi(e, _) => {
                // visit all statements with expressions that end in semicolons
                self.visit_expr(e);
                self.generated_code += ";\n";
            }
            Stmt::Expr(e) => {
                // visit expressions that don't end in semicolons such as if statements and loops
                self.visit_expr(e);
            }
            _ => {
                panic!("Invalid statement");
            }
        }
    }

    fn visit_expr(&mut self, e: &Expr) {
        match e {
            Expr::Assign(e) => {
                self.visit_expr(&e.left);
                self.generated_code += " = ";
                self.visit_expr(&e.right);
            }
            Expr::AssignOp(e) => {
                self.visit_expr(&e.left);
                match e.op {
                    BinOp::AddEq(_) => { self.generated_code += " += "; }
                    BinOp::SubEq(_) => { self.generated_code += " -= "; }
                    BinOp::MulEq(_) => { self.generated_code += " *= "; }
                    BinOp::DivEq(_) => { self.generated_code += " /= "; }
                    BinOp::RemEq(_) => { self.generated_code += " %= "; }
                    BinOp::BitXorEq(_) => { self.generated_code += " &= "; }
                    BinOp::BitAndEq(_) => { self.generated_code += " ^= "; }
                    BinOp::ShlEq(_) => { self.generated_code += " <<= "; }
                    BinOp::ShrEq(_) => { self.generated_code += " >>= "; }
                    _ => {
                        panic!("Invalid binary operator");
                    }
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
                        _ => {} // this case should never occur
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
                } else {
                    panic!("Expected identifier for variable for iteration");
                }
                if let Expr::Range(for_range_expr) = *e.expr.clone() {
                    if let Some(for_range_from_expr) = for_range_expr.from {
                        self.visit_expr(&for_range_from_expr);
                        self.generated_code += "; ";
                        self.generated_code += &for_var_name;
                        self.generated_code += " < ";
                    } else {
                        panic!("Expected value for start of range of iteration");
                    }
                    if let Some(for_range_to_expr) = for_range_expr.to {
                        self.visit_expr(&for_range_to_expr);
                        self.generated_code += "; ";
                        self.generated_code += &for_var_name;
                        self.generated_code += "++";
                    } else {
                        panic!("Expected value for end of range of iteration");
                    }
                } else {
                    panic!("Expected range of iteration");
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
                if let Expr::Range(range) = *e.index.clone() {
                    if let Some(_) = range.from {
                        panic!("Invalid syntax for index");
                    } else if let Some(_) = range.to {
                        panic!("Invalid syntax for index");
                    } else {
                        if let Expr::Path(vector_identifier) = *e.expr.clone() {
                            let vector_name = vector_identifier.path.segments[0].ident.to_string();
                            self.generated_code += &vector_name;
                            self.generated_code += "[get_global_id(";

                            let mut is_dimensionality_parameter = false;
                            let mut dimensionality_parameter_index = 0;
                            for dimensionality_parameter in &self.dimensionality_parameters {
                                if dimensionality_parameter == &vector_name {
                                    is_dimensionality_parameter = true;
                                    self.generated_code += &dimensionality_parameter_index.to_string();
                                }
                                dimensionality_parameter_index += 1;
                            }
                            if !is_dimensionality_parameter {
                                self.dimensionality_parameters.push(vector_name);
                                self.generated_code += &(self.dimensionality_parameters.len() - 1).to_string();
                                if self.dimensionality_parameters.len() > 3 {
                                    panic!("Expected number of holes in Emu function to be less than or equal to 3");
                                }
                            }

                            self.generated_code += ")]";
                        } else {
                            panic!("Expected identifier for array to be indexed");
                        }
                    }
                } else {
                    self.visit_expr(&e.expr);
                    self.generated_code += "[";
                    self.visit_expr(&e.index);
                    self.generated_code += "]";
                }
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
                match e.op {
                    UnOp::Not(_) => { self.generated_code += "!"; }
                    UnOp::Neg(_) => { self.generated_code += "-"; }
                    _ => {
                        panic!("Invalid unary operator");
                    }
                };
                self.visit_expr(&e.expr);
            }
            Expr::Binary(e) => {
                self.visit_expr(&e.left);
                 match e.op {
                    BinOp::Add(_) => { self.generated_code += " + " }
                    BinOp::Sub(_) => { self.generated_code += " - " }
                    BinOp::Mul(_) => { self.generated_code += " * " }
                    BinOp::Div(_) => { self.generated_code += " / " }
                    BinOp::Rem(_) => { self.generated_code += " % " }
                    BinOp::And(_) => { self.generated_code += " && " }
                    BinOp::Or(_) => { self.generated_code += " || " }
                    BinOp::BitAnd(_) => { self.generated_code += " & " }
                    BinOp::BitOr(_) => { self.generated_code += " | " }
                    BinOp::BitXor(_) => { self.generated_code += " ^ " }
                    BinOp::Shl(_) => { self.generated_code += " >> " }
                    BinOp::Shr(_) => { self.generated_code += " << " }
                    BinOp::Lt(_) => { self.generated_code += " < " }
                    BinOp::Gt(_) => { self.generated_code += " > " }
                    BinOp::Le(_) => { self.generated_code += " <= " }
                    BinOp::Ge(_) => { self.generated_code += " >= " }
                    BinOp::Eq(_) => { self.generated_code += " == " }
                    BinOp::Ne(_) => { self.generated_code += " != " }
                    _ => {
                        panic!("Invalid binary operator");
                    }
                };
                self.visit_expr(&e.right);
            }
            Expr::Cast(e) => {
                // cast can be either conversion of precision or units
                let mut is_precision_conversion = false;

                // convert precision
                if let Type::Path(ty) = *e.ty.clone() {
                    let mut precision_conversion_prefix = String::from("(");
                    precision_conversion_prefix += emu_to_opencl(&ty.path.segments[0].ident.to_string());
                    precision_conversion_prefix += ")";

                    is_precision_conversion = precision_conversion_prefix != "(null)";
                    
                    self.generated_code += precision_conversion_prefix.as_ref();
                } else {
                    panic!("Invalid type to cast to");
                }

                self.visit_expr(&e.expr);
                
                // convert units
                if !is_precision_conversion {
                    if let Type::Path(ty) = *e.ty.clone() {
                        if let Some(ty_prefix) = ty.path.segments[0].ident.to_string().chars().next() {
                            self.generated_code += String::from(emu_type_prefix_to_opencl(ty_prefix.to_string().as_ref())).as_ref();
                        } else {
                            panic!("Expected prefix for unit annotation");
                        }
                    }
                }
            }
            Expr::Lit(e) => {
                let e_lit = e.lit.clone();
                if let Lit::Int(i) = e_lit {
                    self.generated_code += &i.value().to_string();
                } else if let Lit::Float(f) = e_lit {
                    self.generated_code += &f.value().to_string();
                } else if let Lit::Bool(b) = e_lit {
                    self.generated_code += if b.value { "true" } else { "false" }
                } else {
                    panic!("Invalid literal");
                }
            }
            Expr::Path(e) => {
                // get raw name
                let raw_identifier_name = e.path.segments[0].ident.to_string();

                if e.path.segments.len() != 1 {
                    panic!("Invalid identifier");
                }

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
                    "PI"  => "M_PI_F",
                    "PAU" => "(1.5 * M_PI_F)",
                    "TAU" => "(2 * M_PI_F)",
                    "E"   => "(M_E_F)",
                    "PHI" => "1.618033",
                    "G"   => "6.67408e-11",
                    "SG"  => "9.80665",
                    "C"   => "29979246e1",
                    "H"   => "6.626070e-34",
                    "K"   => "1.380648e-23",
                    "L"   => "6.022140e23",
                    "MU0" => "0.000001",
                    "R"   => "8.314462",
                    _     => &raw_identifier_name,
                };
            }
            Expr::Break(_) => {
                self.generated_code += "break";
            }
            Expr::Continue(_) => {
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
            _ => {
                panic!("Invalid expression");
            }
        }
    }
}

struct EmuFunction {
    name: String,
    generated_code: String,
    parameters: Vec<String>,
    dimensionality_parameters: Vec<String>
}

impl Parse for EmuFunction {
    fn parse(input: ParseStream) -> Result<Self> {
        // get "function" qualifier
        let qualifier: Ident = input.parse()?;
        if qualifier.to_string() != "function" {
            panic!("Expected \"function\" at Emu function definition");
        }

        // get name
        let name: Ident = input.parse()?;

        // get parmeters
        let content_parameters;
        let _ = parenthesized!(content_parameters in input);
        let punctuated_parameters: Punctuated<EmuFunctionParameter, Token![,]> = content_parameters.parse_terminated(EmuFunctionParameter::parse)?;
        let parameters: Vec<EmuFunctionParameter> = punctuated_parameters
            .pairs()
            .map(|parameter_pair| parameter_pair.into_value())
            .cloned()
            .collect::<Vec<EmuFunctionParameter>>();

        // get return type
        let mut return_ty = EmuType::Null;
        if input.lookahead1().peek(Ident) {
            let return_literal: Ident = input.parse()?;
            return_ty = emu_to_emu_type(&return_literal.to_string());
            if let EmuType::Null = return_ty {
                panic!("Invalid return type of Emu function");
            }
        }

        // discard braces and documentation comments
        let content_block;
        let _ = braced!(content_block in input);
        let _ = content_block.call(Attribute::parse_inner)?;

        // get statements
        let statements = content_block.call(Block::parse_within)?;

        // get generated code from function
        let mut generated_code = match return_ty {
            EmuType::Null => String::from("__kernel "),
            _ => String::new()
        };

        // generate return type
        generated_code += emu_type_to_opencl(&return_ty);

        // generate name
        generated_code += " ";
        generated_code += &name.to_string();

        // generate parameters
        let mut parameter_names = vec![];
        generated_code += " (";
        for parameter in &parameters {
            parameter_names.push(String::from(parameter.name.clone()));

            generated_code += if parameter.is_vector { "__global " } else { "" };
            generated_code += emu_type_to_opencl(&parameter.ty);
            generated_code += if parameter.is_vector { "* " } else { " " };
            generated_code += &parameter.name;
            generated_code += ",";
        }
        if !parameters.is_empty() {
            generated_code.pop();
        }
        generated_code += ") ";

        // generate body
        generated_code += "{\n";
        let mut body = EmuFunctionBody { generated_code: String::new(), dimensionality_parameters: Vec::new() };
        for statement in statements {
            body.visit_stmt(&statement);
        }
        generated_code += &body.generated_code;
        generated_code += "}\n";

        // look at statements to find which parameters contribute to dimensionality
        let dimensionality_parameters = body.dimensionality_parameters;

        Ok(EmuFunction {
            name: name.to_string(),
            generated_code: generated_code,
            parameters: parameter_names,
            dimensionality_parameters: dimensionality_parameters
        })
    }
}

struct RustFunctionParameter {
    is_vector: bool,
    name: String,
    ty: EmuType
}

struct RustFunctionDeclaration {
    documentation: Vec<Attribute>,
    is_pub: bool,
    name: String,
    parameters: Vec<RustFunctionParameter>
}

impl Parse for RustFunctionDeclaration {
    fn parse(input: ParseStream) -> Result<Self> {
        // parse for function documentation
        let documentation = input.call(Attribute::parse_outer)?;

        // parse for function declaration
        let function: ForeignItemFn = input.parse()?;
        let function_declaration: FnDecl = *function.decl;

        // check if public
        let mut is_pub = false;
        match function.vis {
            Visibility::Public(_) => { is_pub = true; },
            Visibility::Inherited => { is_pub = false; },
            _ => {
                panic!("Unexpected qualifier for Rust function declaration");
            }
        };

        // get name
        let name = function.ident.to_string();

        // TODO handle error for name that isn't a defined function

        // ger parameters
        let args: Vec<FnArg> = function_declaration.inputs.iter().map(|arg| arg.to_owned()).collect();
        let mut parameters: Vec<RustFunctionParameter> = Vec::new();
        for arg in args {
            if let FnArg::Captured(captured_arg) = arg {
                // get name of parameter
                let mut name = String::new();
                if let Pat::Ident(name_literal) = captured_arg.pat {
                    name = name_literal.ident.to_string();
                } else {
                    panic!("Missing valid name of parameter in Rust function declaration");
                }

                if let Type::Reference(ty_reference) = captured_arg.ty {
                    if let Some(_) = ty_reference.clone().lifetime {
                        panic!("Lifetime not expected in parameter in Rust function declaration");
                    }

                    let is_mut = if let Some(_) = ty_reference.clone().mutability { true } else { false };

                    let mut ty = EmuType::Null;
                    let mut is_vector = false;
                    if let Type::Path(ty_literal) = *ty_reference.elem {

                        let raw_segments: Vec<String> = ty_literal.path.segments.iter().map(|segment| segment.ident.to_string()).collect();
                        let segments: Vec<&str> = raw_segments.iter().map(|segment| segment.as_str()).collect();

                        if segments.len() == 1 {
                            match segments[0] {
                                "Vec" => {
                                    if let PathArguments::AngleBracketed(ty_parameter) = &ty_literal.path.segments[0].arguments {
                                        if !ty_parameter.colon2_token.is_none() || ty_parameter.args.len() != 1 {
                                            panic!("Invalid type of parameter in Rust function declaration");
                                        } else {
                                            if let GenericArgument::Type(ty_parameter_ty) = ty_parameter.args[0].clone() {
                                                if let Type::Path(ty_parameter_literal) = ty_parameter_ty {
                                                    if ty_parameter_literal.path.segments.len() != 1 {
                                                        panic!("Invalid type of parameter in Rust function declaration");
                                                    } else {
                                                        ty = emu_to_emu_type(&ty_parameter_literal.path.segments[0].ident.to_string());
                                                        if let EmuType::Null = ty {
                                                            panic!("Invalid type of parameter in Rust function declaration");
                                                        }
                                                        is_vector = true;
                                                    }
                                                } else {
                                                    panic!("Invalid type of parameter in Rust function declaration");
                                                }
                                            } else {
                                                panic!("Invalid type of parameter in Rust function declaration");
                                            }
                                        }
                                    } else {
                                        panic!("Invalid type of parameter in Rust function declaration");
                                    }
                                }
                                _ => {
                                    is_vector = false;
                                    ty = emu_to_emu_type(segments[0]);

                                    if let EmuType::Null = ty {
                                        panic!("Invalid type of parameter in Rust function declaration");
                                    }
                                }
                            }
                        } else {
                            panic!("Invalid type of parameter in Rust function declaration");
                        }

                        if is_vector != is_mut {
                            if is_vector {
                                panic!("Expected mutable reference to Vec in parameters in Rust function declaration");
                            } else {
                                panic!("Expected immutable reference to scalar in parameters in Rust function declaration");
                            }
                        }
                    } else {
                        panic!("Invalid type of parameter in Rust function declaration");
                    }

                    parameters.push(RustFunctionParameter {
                        is_vector: is_vector,
                        name: name,
                        ty: ty
                    });
                } else {
                    panic!("Expected references in parameters in Rust function declaration");
                }
            } else {
                panic!("Expected explicit parameters in Rust function declaration");
            }
        }

        Ok(RustFunctionDeclaration {
            documentation: documentation,
            is_pub: is_pub,
            name: name,
            parameters: parameters
        })
    }
}

struct EmuItems {
    emu_functions: Vec<EmuFunction>,
    rust_function_declarations: Vec<RustFunctionDeclaration>
}

impl Parse for EmuItems {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut emu_functions = vec![];
        let mut rust_function_declarations = vec![];

        // consume next item
        while !input.is_empty() {
            // determine what kind of item to consume
            let lookahead = input.lookahead1();

            if lookahead.peek(Ident) {
                // handle Emu function
                let new_emu_function = input.call(EmuFunction::parse)?;
                emu_functions.push(new_emu_function);
            } else {
                // handle Rust function declaration
                // throw error if Rust function declaration not found
                let new_rust_function_declaration = input.call(RustFunctionDeclaration::parse)?;
                rust_function_declarations.push(new_rust_function_declaration);
            }
        }

        for rust_function_declaration in &rust_function_declarations {
            let mut is_defined = false;
            for emu_function in &emu_functions {
                if rust_function_declaration.name == emu_function.name {
                    is_defined = true;
                    if emu_function.dimensionality_parameters.len() == 0 || emu_function.dimensionality_parameters.len() > 3 {
                        panic!("Expected function to have exactly 1, 2, or 3 holes");
                    }
                }
            }
            if !is_defined {
                panic!("Function not defined")
            }
        }

        Ok(EmuItems {
            emu_functions: emu_functions,
            rust_function_declarations: rust_function_declarations
        })
    }
}

#[proc_macro]
/// Accepts a chunk of Emu code and generates Rust functions
pub fn emu(tokens: TokenStream) -> TokenStream {
    // parse Emu items
    let EmuItems { emu_functions, rust_function_declarations } = parse_macro_input!(tokens as EmuItems);

    // get program code from concatenate generated code from each Emu function
    let mut program = String::new();
    for emu_function in &emu_functions {
        program += emu_function.generated_code.as_str();
    }

    // generate code for each Rust function
    let mut functions = vec![];
    for function_declaration in rust_function_declarations {
        // get dimensions
        let mut dimensions = vec![];
        for emu_function in &emu_functions {
            if emu_function.name == function_declaration.name {
                for dimensionality_parameter in &emu_function.dimensionality_parameters {
                    let dimension = Ident::new(&dimensionality_parameter, Span::call_site());

                    dimensions.push(quote! {
                        #dimension .len()
                    });
                }

                let mut emu_function_parameter_index = 0;
                for parameter in &emu_function.parameters {
                    if parameter != &function_declaration.parameters[emu_function_parameter_index].name {
                        panic!("Names of parameters in Rust function declaration must match names of parameters in Emu function definition");
                    }

                    emu_function_parameter_index += 1;
                }
            }
        }

        // generate documentation
        let documentation = &function_declaration.documentation;

        // generate signature
        let maybe_pub = if function_declaration.is_pub { quote! { pub } } else { quote! {} };
        let name = Ident::new(&function_declaration.name, Span::call_site());
        let mut parameters = vec![];

        // generate buffer loading
        let mut get_buffers = vec![];

        // generate buffer reading
        let mut read_buffers = vec![];

        // generate creation of kernel
        let kernel_name = function_declaration.name.as_str();
        let mut kernel_arguments = vec![];

        for parameter in function_declaration.parameters {
            // function signature
            let parameter_name = Ident::new(&parameter.name, Span::call_site());
            let ty = emu_parameter_to_rust(&parameter.ty, &parameter.is_vector);

            parameters.push(quote! { #parameter_name : #ty });

            // buffer getting
            if parameter.is_vector {
                let buffer_ty = emu_type_to_rust(&parameter.ty);
                let buffer_name = Ident::new(&(parameter.name.clone() + "_buffer"), Span::call_site());
                let buffer_source = Ident::new(&parameter.name, Span::call_site());

                get_buffers.push(quote! {
                    // TODO look at cache in EMU for applicable instance
                    let #buffer_name: Buffer< #buffer_ty > = Buffer::builder()
                        .queue(queue.clone())
                        .flags(flags::MEM_READ_WRITE)
                        .len(#buffer_source .len())
                        .copy_host_slice(#buffer_source)
                        .build()?;
                });
            }

            // kernel creation
            let maybe_ref = if parameter.is_vector { quote! { & } } else { quote! {  } };
            let argument = match parameter.is_vector {
                true => Ident::new(&(parameter.name.clone() + "_buffer"), Span::call_site()),
                false => Ident::new(&parameter.name, Span::call_site()),
            };
            kernel_arguments.push(quote! {
                .arg(#maybe_ref #argument)
            });

            // buffer reading
            // TODO only read back buffers that have been changed
            if parameter.is_vector {
                let buffer_name = Ident::new(&(parameter.name.clone() + "_buffer"), Span::call_site());
                let buffer_target = Ident::new(&parameter.name, Span::call_site());

                read_buffers.push(quote! {
                    #buffer_name .cmd()
                        .queue(&queue)
                        .offset(0)
                        .read(#buffer_target)
                        .enq()?;
                })
            }
        }

        // generate Rust code for function
        functions.push(quote! {
            #(#documentation)*

            #maybe_pub fn #name ( #(#parameters),* ) -> ocl::Result<()> {

                use ocl::{flags, Platform, Device, Context, Queue, Program, Buffer, Kernel};

                // get platform
                // get device from platform
                // get context from platform, device
                // get queue from context, device
                // TODO look at cache in EMU for applicable instances
                let platform = Platform::default();
                let device = Device::first(platform)?;
                let context = Context::builder()
                    .platform(platform)
                    .devices(device.clone())
                    .build()?;
                let queue = Queue::new(&context, device, None)?;

                // get program
                // TODO look at cache in EMU for applicable instance
                let program = Program::builder()
                    .devices(device)
                    .src( #program )
                    .build(&context)?;

                // get buffers
                #(#get_buffers)*

                // get dimensions
                let dimensions = [ #(#dimensions),* ];

                // create kernel from program, queue, dimensions, arguments/buffers
                let kernel = Kernel::builder()
                    .program(&program)
                    .name(#kernel_name)
                    .queue(queue.clone())
                    .global_work_size(dimensions)
                    #(#kernel_arguments)*
                    .build()?;

                // run kernel
                unsafe {
                    kernel.cmd()
                        .queue(&queue)
                        .global_work_offset(kernel.default_global_work_offset())
                        .global_work_size(dimensions)
                        .local_work_size(kernel.default_local_work_size())
                        .enq()?;
                }

                // TODO read buffers
                // TODO ensure generated code is correct
                #(#read_buffers)*

                Ok(())
            }
        });
    }

    // generate output Rust code
    let output = quote! {
        #(#functions)*
    };

    if DEBUG { println!("{}", output); }

    // return output converted to token stream
    output.into()
}

fn emu_to_emu_type(emu: &str) -> EmuType {
    match emu {
        "bool" => EmuType::Bool,
        "i8" => EmuType::Int8,
        "i16" => EmuType::Int16,
        "i32" => EmuType::Int32,
        "i64" => EmuType::Int64,
        "u8" => EmuType::UInt8,
        "u16" => EmuType::UInt16,
        "u32" => EmuType::UInt32,
        "u64" => EmuType::UInt64,
        "f32" => EmuType::Float32,
        _ => EmuType::Null
    }
}

fn emu_type_to_opencl(emu_type: &EmuType) -> &str {
    match emu_type {
        EmuType::Bool => "bool",
        EmuType::Int8 => "char",
        EmuType::Int16 => "short",
        EmuType::Int32 => "int",
        EmuType::Int64 => "long",
        EmuType::UInt8 => "uchar",
        EmuType::UInt16 => "ushort",
        EmuType::UInt32 => "uint",
        EmuType::UInt64 => "ulong",
        EmuType::Float32 => "float",
        _ => "void"
    }
}

fn emu_to_opencl(emu: &str) -> &str {
    match emu {
        "bool" => "bool",
        "i8" => "char",
        "i16" => "short",
        "i32" => "int",
        "i64" => "long",
        "u8" => "uchar",
        "u16" => "ushort",
        "u32" => "uint",
        "u64" => "ulong",
        "f32" => "float",
        _ => "null"
    }
}

fn emu_parameter_to_rust(ty: &EmuType, is_vector: &bool) -> quote::__rt::TokenStream {
    match (ty, is_vector) {
        (EmuType::Bool, true) => quote! { &mut Vec<bool> },
        (EmuType::Int8, true) => quote! { &mut Vec<i8> },
        (EmuType::Int16, true) => quote! { &mut Vec<i16> },
        (EmuType::Int32, true) => quote! { &mut Vec<i32> },
        (EmuType::Int64, true) => quote! { &mut Vec<i64> },
        (EmuType::UInt8, true) => quote! { &mut Vec<u8> },
        (EmuType::UInt16, true) => quote! { &mut Vec<u16> },
        (EmuType::UInt32, true) => quote! { &mut Vec<u32> },
        (EmuType::UInt64, true) => quote! { &mut Vec<u64> },
        (EmuType::Float32, true) => quote! { &mut Vec<f32> },
        (EmuType::Bool, false) => quote! { & bool },
        (EmuType::Int8, false) => quote! { & i8 },
        (EmuType::Int16, false) => quote! { & i16 },
        (EmuType::Int32, false) => quote! { & i32 },
        (EmuType::Int64, false) => quote! { & i64 },
        (EmuType::UInt8, false) => quote! { & u8 },
        (EmuType::UInt16, false) => quote! { & u16 },
        (EmuType::UInt32, false) => quote! { & u32 },
        (EmuType::UInt64, false) => quote! { & u64 },
        (EmuType::Float32, false) => quote! { & f32 },
        _ => quote! { } // this case should never occur
    }
}

fn emu_type_to_rust(ty: &EmuType) -> quote::__rt::TokenStream {
    match &ty {
        EmuType::Bool => quote! { bool },
        EmuType::Int8 => quote! { i8 },
        EmuType::Int16 => quote! { i16 },
        EmuType::Int32 => quote! { i32 },
        EmuType::Int64 => quote! { i64 },
        EmuType::UInt8 => quote! { u8 },
        EmuType::UInt16 => quote! { u16 },
        EmuType::UInt32 => quote! { u32 },
        EmuType::UInt64 => quote! { u64 },
        EmuType::Float32 => quote! { f32 },
        _ => quote! { } // this case should never occur
    }
}

fn emu_type_prefix_to_opencl(prefix: &str) -> &str {
    match prefix {
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
        "_" => "*1",
        _ => "*1",
    }
}

// TODO don't clone stuff

// TODO report OpenCL runtime errors
// TODO report errors with Span instead of panic! when feature becomes stable
// TODO define reserved keywords and throw error when used for function name, parameter name, variable name
// TODO document all functions, constants that are built in
// TODO define procedural macro for creating caches of data
// TODO make generated functions look at cache before creating new instances of data