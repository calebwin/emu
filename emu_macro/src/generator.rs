// for parsing Rust
extern crate syn;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::*;

// for etc.
use crate::identifier::Dim;

// represents a parameter of a kernel
//
// every time someone does gpu_do!(launch()), we try to launch a kernel by
// using the next for loop we find
// the parameters of a kernel are basically everything outside of the for loop that
// is referenced from inside of the for loop
// this would generally be variable defined outside but used inside
// in order to use those variables inside, we need to pass them in
pub struct Parameter {
    pub is_array: bool,
    pub name: String,
}

// this makes it easy to compile a Parameter
// into a chunk of OpenCL code that can be used in the generated
// OpenCL code for the signature of a kernel function
impl ToString for Parameter {
    fn to_string(&self) -> String {
        let mut result = String::new();

        result += if self.is_array {
            "global float*"
        } else {
            "float"
        };
        result += " emumumu_"; // prefix all identifiers with emumumu
        result += &self.name;

        result
    }
}

// this is what is used to generate OpenCL code
//
// it implements Syn's Visit traits so that it can visit
// nodes in Rust AST and generate code
// it holds code (as well as other information about the kernel for whose code
// is being generated) as state
pub struct Generator {
    // metadata for the kernel to be generated
    pub global_work_size_dims: Vec<Dim>,
    // code to be generated
    // code = signature + body
    pub code: String,
    pub signature: String,
    pub body: String,
    // this is built up over the course of visiting different
    // nodes in Rust AST. we look for identifiers that would
    // need to be passed in as parameters and mark them as such
    // by appending to this Vec
    pub params: Vec<Parameter>,
    // used for saying what we allow as possible in the subset of Rust that we work with
    // it can be toggled at different points in visiting
    // more fields like this might be added (like a field_allowed or struct_allowed)
    pub block_allowed: bool,
    // ok so one thing that isn't so cool about macros is that we don't know the types
    // of anything. so we can't know if something is an array or not for example
    //
    // some things to note
    // 1. type checking - this is already done for us by RustC, we can RestInPeace that type usage is correct
    // 2. type restrictions - this is harder, we need to restrict what type things can be, e.g. - we don't support 4D arrays or strings
    // 3. type inference - this is also hard, once we can assume type restriction and type checking, we must look at code and try to figure out types
    //
    // 1st, type restriction...
    //
    // we could deal with type restriction by just assuming everything is correctly typed
    // or, we can try to restrict the types of as many things as possible
    //
    // arguments - arguments passed into the kernel are of known types, we can restrict them (currently only wrapping f32 or Buffer<f32>)
    // literals - we can restrict these too through parsing
    // functions/operators - we must only support operators and functions that will keep types in a restricted subset
    //
    // 2nd, type inference...
    //
    // because Rust itself allows types to be elided, we must infer them in order to generate OpenCL code (since OpenCL doesn't allow elision)
    // now this is a bit tricky. we can assume that types of all things are within a small subset and all usage is type checked
    //
    // but in order to infer types we must do some messy stuff
    // the following field(s) are example of this. we must try to know at different points, what is the type of the next thing
    // for example, when we implement variables we need to look at an expression and see if we can detect what the type must be
    // note that we don't need to do some complex Hindley-Milner stuff, we can assume it is correctly typed and only uses types from a small subset (basically usize, f32, [f32], bool)
    pub is_next_ident_array: bool,
    // used for propogating errors
    pub failed_to_generate: bool,
    pub errors: Vec<Error>,
}

impl Generator {
    pub fn from(global_work_size_dims: Vec<Dim>) -> Self {
        // here we just set everything to defaults
        Self {
            global_work_size_dims: global_work_size_dims,
            code: String::new(),
            signature: String::new(),
            body: String::new(),
            params: vec![],
            failed_to_generate: false,
            block_allowed: true,
            is_next_ident_array: false,
            errors: vec![],
        }
    }
}

impl<'ast> Visit<'ast> for Generator {
    // this is currently only invoked once for each kernel to be launched
    // so it basically just generates much of the kernel function signature and then
    // goes through the statements to compile them
    fn visit_block(&mut self, node: &'ast Block) {
        if self.block_allowed {
            self.block_allowed = false; // no more blocks
            self.signature += "__kernel void __main__(";
            // write in calls to OpenCL API for each dimension
            self.body += "{\n";
            for (i, global_work_size_dim) in self.global_work_size_dims.iter().enumerate() {
                match global_work_size_dim {
                    Dim::RangeFromZero(name, _) => {
                        self.body += "\t";
                        self.body += "int emumumu_";
                        self.body += &name;
                        self.body += " = get_global_id(";
                        self.body += &i.to_string();
                        self.body += ");\n"
                    }
                }
            }
            // compile all statements
            for stmt in &node.stmts {
                match stmt {
                    // for now, only a series of semicolon-ed statements are expected
                    Stmt::Semi(expr, _) => {
                        match expr {
                            // for now, only statement allowed is assign index
                            Expr::Assign(assign) => {
                                if let Expr::Index(index) = *assign.left.clone() {
                                    // we don't allow 2D arrays so the expr must be an ident
                                    if let Expr::Path(_path) = *index.expr.clone() {
                                        self.body += "\t";
                                        self.is_next_ident_array = true;
                                        self.visit_expr(&index.expr); // we now know that the expr must be a path
                                        self.is_next_ident_array = false;
                                        self.body += "[";
                                        self.visit_expr(&index.index);
                                        self.body += "] = ";
                                        self.visit_expr(&assign.right);
                                        self.body += ";\n";
                                    } else {
                                        self.failed_to_generate = true;
                                        self.errors.push(Error::new(
                                            (*index.expr.clone()).span(),
                                            "can only get index of a 1D array",
                                        ));
                                    }
                                } else {
                                    self.failed_to_generate = true;
                                    self.errors.push(Error::new(
                                        (*assign.left.clone()).span(),
                                        "only assignment of an array element is supported",
                                    ));
                                }
                            }
                            _ => {
                                self.failed_to_generate = true;
                                self.errors.push(Error::new(
                                    (expr.clone()).span(),
                                    "only an assignment is a supported statement",
                                ));
                            }
                        }
                    }
                    _ => {
                        self.failed_to_generate = true;
                        self.errors
                            .push(Error::new((stmt.clone()).span(), "unsupported item"));
                    }
                }
            }
            self.signature += &self
                .params
                .iter()
                .map(|param| param.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            self.signature += ") ";
            self.body += "}";

            self.code += &self.signature;
            self.code += &self.body;
        } else {
            self.failed_to_generate = true;
            self.errors
                .push(Error::new((node.clone()).span(), "block was not exected"));
        }
    }
    // this is invoked for all expressions
    fn visit_expr(&mut self, node: &'ast Expr) {
        match node {
            Expr::Path(path) => {
                // we only work with paths that are identifiers
                if let Some(ident) = path.path.get_ident() {
                    self.body += "emumumu_"; // append prefix to start of all identifiers
                    self.body += &ident.to_string();

                    // we need to see if we need to add this as a parameter
                    // added paramters will be used to figure out the Rust code
                    // for passing arguments into the kernel as well as generating kernel
                    // function signature with all the paramters
                    //
                    // but we only add as a parameter if the parameter
                    // is not yet added as a paramter and if it is not a declared variable
                    let mut is_already_declared = false;
                    let mut is_alread_added = false;
                    // for now, we figure out if this is already declared as a variable by
                    // looking at the dimensions
                    //
                    // why the dimensions?
                    //
                    // for each dimension, we create a variable, e.g. - int emumumu_i = get_global_id(0)
                    // but of course, this will not be the only way we create variables
                    // in the future, we will need a better way of keeping track of variables that are
                    // declared, shadowed, mutated so that we can know right here if an identifier has either
                    // already been declared or if it needs to be passed in as a paramter
                    for global_work_size_dim in self.global_work_size_dims.clone() {
                        match global_work_size_dim {
                            Dim::RangeFromZero(name, _) => {
                                if ident.to_string() == name {
                                    is_already_declared = true;
                                }
                            }
                        }
                    }
                    // check if already added as parameter
                    for param in &self.params {
                        if ident.to_string() == param.name {
                            is_alread_added = true;
                        }
                    }
                    // if not yet added and not already declared, add this as a parameter
                    if !is_already_declared && !is_alread_added {
                        self.params.push(Parameter {
                            is_array: self.is_next_ident_array,
                            name: ident.to_string(),
                        })
                    }
                } else {
                    self.failed_to_generate = true;
                    self.errors
                        .push(Error::new((path.clone()).span(), "expected identifier"));
                }
            }
            Expr::Index(index) => {
                // we can infer that the thing being indexed is an identifier representing a 1D array
                // that is because, as reasoned above, we can assume type restriction to already be done so there
                // are no 2D, 3D, or 4D arrays
                if let Expr::Path(_path) = *index.expr.clone() {
                    self.is_next_ident_array = true;
                    self.visit_expr(&index.expr); // we now know that the expr must be a path
                    self.is_next_ident_array = false;
                    self.body += "[";
                    self.visit_expr(&index.index);
                    self.body += "]";
                } else {
                    self.failed_to_generate = true;
                    self.errors.push(Error::new(
                        (index.expr.clone()).span(),
                        "expected name of a 1D array",
                    ));
                }
            }
            Expr::Lit(lit) => {
                if let Lit::Float(float) = &lit.lit {
                    let float_val = float.base10_parse::<f32>();

                    if float_val.is_ok() {
                        // currently, we only support f32
                        self.body += &float_val.unwrap().to_string();
                    } else {
                        self.failed_to_generate = true;
                        self.errors.push(Error::new(
                            (float.clone()).span(),
                            "expected 32-bit floating point number",
                        ));
                    }
                } else {
                    self.failed_to_generate = true;
                    self.errors.push(Error::new(
                        (lit.clone()).span(),
                        "expected 32-bit floating point number",
                    ));
                }
            }
            Expr::Binary(binary) => {
                // only handle a couple of binops
                // but adding more is super easy! right?
                // this should be an easy contribution, I hope
                match binary.op {
                    BinOp::Mul(_) => {
                        self.visit_expr(&binary.left);
                        self.body += " * ";
                        self.visit_expr(&binary.right);
                    }
                    BinOp::Add(_) => {
                        self.visit_expr(&binary.left);
                        self.body += " + ";
                        self.visit_expr(&binary.right);
                    }
                    _ => {
                        self.failed_to_generate = true;
                        self.errors.push(Error::new(
                            (binary.op.clone()).span(),
                            "unsupported binary expression",
                        ));
                    }
                }
            }
            Expr::Paren(paren) => {
                // pretty straightforward...
                self.body += "(";
                self.visit_expr(&paren.expr);
                self.body += ")";
            }
            _ => {
                // any other expression is simply unsupported
                self.failed_to_generate = true;
                self.errors
                    .push(Error::new((node.clone()).span(), "unsupported expression"));
            }
        }
    }
}
