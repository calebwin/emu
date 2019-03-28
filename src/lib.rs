// crates for parsing
extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

// used for containing macro input
use proc_macro::TokenStream;

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

/// Represents Emu program
struct EmuProgram {
    name: Ident,
    params: Vec<Expr>,
    stmts: Vec<Stmt>,
    generated_code: String,
}

/// Implementation of parser for Emu program
impl Parse for EmuProgram {
    fn parse(input: ParseStream) -> Result<Self> {
        // discard documentation comments
        let _ = input.call(Attribute::parse_outer)?;

        // get name of program
        let name: Ident = input.parse()?;

        // get punctuated parmeters
        let content_kernel;
        let _ = parenthesized!(content_kernel in input);
        let punctuated_parameters: Punctuated<Expr, Token![,]> =
            content_kernel.parse_terminated(Expr::parse)?;
        let punctuated_parameters_iter = punctuated_parameters.pairs();

        // get parameters
        let parameters: Vec<Expr> = punctuated_parameters_iter
            .map(|parameter_pair| parameter_pair.into_value())
            .cloned()
            .collect::<Vec<Expr>>();

        // discard braces and documentation comments
        let content_block;
        let _ = braced!(content_block in input);
        let _ = content_block.call(Attribute::parse_inner)?;

        // get statements
        let statements = content_block.call(Block::parse_within)?;

        // return name and statements
        Ok(EmuProgram {
            name: name,
            params: parameters,
            stmts: statements,
            generated_code: String::new(),
        })
    }
}

/// Implementation of visitor for AST of Emu program
impl<'a> Visit<'a> for EmuProgram {
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
                }
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
                self.generated_code += &e.path.segments[0].ident.to_string();
            }
            Expr::Break(e) => {
                self.generated_code += "break";
            }
            Expr::Continue(e) => {
                self.generated_code += "continue";
            }
            Expr::Paren(e) => {
                self.visit_expr(&e.expr);
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

    // traverse AST of each statement of parsed program
    // then, generate OpenCL code for body of kernel function from statements
    let program_statements = program.stmts.clone();
    for statement in program_statements {
        program.visit_stmt(&statement)
    }

    // get generated body code from program
    let generated_body_code = program.generated_code;
    let mut generated_code =
        String::from(String::from("__kernel void ") + &program.name.to_string());

    // TODO append parameters to generated code
    generated_code.push_str("(");

    // get parameter information from AST
    let program_parameters = program.params.clone();
    for parameter in program_parameters {
        // default values for parameterp properties
        let mut parameter_name = String::from("");
        let mut parameter_address_space = String::from("__private");
        let mut parameter_type = String::from("float");

        if let Expr::Type(parameter_qualifier_expr) = parameter {
            // parse parameter qualifier type
            if let Expr::Cast(parameter_qualifier_type_cast_expr) = *parameter_qualifier_expr.expr {
                // parse parameter name
                if let Expr::Path(parameter_qualifier_name_path_expr) =
                    *parameter_qualifier_type_cast_expr.expr
                {
                    parameter_name = parameter_qualifier_name_path_expr.path.segments[0]
                        .ident
                        .to_string();
                }

                // parse parameter type
                match *parameter_qualifier_type_cast_expr.ty {
                    Type::Path(parameter_qualifier_type_cast_path_expr) => {
                        parameter_type = match parameter_qualifier_type_cast_path_expr.path.segments
                            [0]
                        .ident
                        .to_string()
                        .as_ref()
                        {
                            "bool" => String::from("bool"),
                            "f32" => String::from("float"),
                            "i8" => String::from("char"),
                            "i16" => String::from("short"),
                            "i32" => String::from("int"),
                            "i64" => String::from("long"),
                            "u8" => String::from("uchar"),
                            "u16" => String::from("ushort"),
                            "u32" => String::from("uint"),
                            "u64" => String::from("ulong"),
                            _ => String::from("float"),
                        }
                    }
                    Type::Slice(parameter_qualifier_type_cast_array_expr) => {
                        if let Type::Path(parameter_qualifier_type_cast_array_type_expr) =
                            *parameter_qualifier_type_cast_array_expr.elem
                        {
                            parameter_type =
                                match parameter_qualifier_type_cast_array_type_expr.path.segments[0]
                                    .ident
                                    .to_string()
                                    .as_ref()
                                {
                                    "bool" => String::from("bool"),
                                    "f32" => String::from("*float"),
                                    "i8" => String::from("*char"),
                                    "i16" => String::from("*short"),
                                    "i32" => String::from("*int"),
                                    "i64" => String::from("*long"),
                                    "u8" => String::from("*uchar"),
                                    "u16" => String::from("*ushort"),
                                    "u32" => String::from("*uint"),
                                    "u64" => String::from("*ulong"),
                                    _ => String::from("*float"),
                                }
                        }
                    }
                    _ => {}
                }
            }

            // parse parameter qualifier address space
            if let Type::Path(parameter_qualifier_address_space_path_expr) =
                *parameter_qualifier_expr.ty
            {
                parameter_address_space =
                    match parameter_qualifier_address_space_path_expr.path.segments[0]
                        .ident
                        .to_string()
                        .as_ref()
                    {
                        "GLOBAL" => String::from("__global"),
                        "LOCAL" => String::from("__local"),
                        _ => String::from("__private"),
                    }
            }
        }

        // append parameter details to generated code
        generated_code.push_str(&parameter_address_space);
        generated_code.push_str(" ");
        generated_code.push_str(&parameter_type);
        generated_code.push_str(" ");
        generated_code.push_str(&parameter_name);
        generated_code.push_str(", ");
    }

    // remove last comma if one was appended
    if generated_code.ends_with(", ") {
        generated_code.truncate(generated_code.len() - 2)
    }

    generated_code.push_str(")");

    // append body code
    generated_code.push_str("{");
    generated_code.push_str(&generated_body_code);
    generated_code.push_str("}");

    println!("{:?}", generated_code);

    // generate output Rust code
    let output = quote! {
        static program: &'static str = #generated_code;
    };

    // return output converted to token stream
    output.into()
}
