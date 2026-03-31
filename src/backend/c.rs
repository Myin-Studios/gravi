use crate::{backend::Backend, lexer::Operator, parser::{Expr, Parallelism, Program, Value, VarDecl}};

pub struct CGenerator
{
    out: String
}

impl CGenerator {
    pub fn new() -> Self
    {
        Self
        {
            out: String::new()
        }
    }

    fn gen_var(&mut self, var: &VarDecl)
    {
        let par = var.parallelism();

        let ty = match var.ty() {
            crate::lexer::Type::Numeric(numeric) => {
                match numeric {
                    crate::lexer::Numeric::U8 => "unsigned char",
                    crate::lexer::Numeric::U16 => "unsigned short",
                    crate::lexer::Numeric::U32 => "unsigned int",
                    crate::lexer::Numeric::U64 => "unsigned long",
                    crate::lexer::Numeric::I8 => "char",
                    crate::lexer::Numeric::I16 => "short",
                    crate::lexer::Numeric::I32 => "int",
                    crate::lexer::Numeric::I64 => "long",
                    crate::lexer::Numeric::F16 => "float",
                    crate::lexer::Numeric::F32 => "float",
                    crate::lexer::Numeric::F64 => "double",
                }
            },
            crate::lexer::Type::StringLiteral => {
                "string"
            },
            crate::lexer::Type::Boolean => {
                "bool"
            },
            crate::lexer::Type::Character => {
                "char"
            },
            crate::lexer::Type::Custom(c) => {
                c.as_str()
            },
            crate::lexer::Type::None => {
                "nulltype"
            },
        };
        
        let mutable = if *var.mutable()
        {
            ""
        }
        else {
            "const "
        };

        match var.value().as_ref().unwrap_or(&crate::parser::Value::Null) {
            Value::Expression(e) => {
                match e {
                    crate::parser::Expr::Range(range) => {
                        let start = match range.start().as_ref() {
                            crate::parser::Expr::Literal(val) => val.to_string(),
                            _ => { String::from("0") }
                        };
                        let step = match range.step().as_ref().unwrap_or(&Box::new(Expr::Literal("1".to_string()))).as_ref() {
                            crate::parser::Expr::Literal(val) => val.to_string(),
                            _ => { String::from("1") }
                        };
                        let end = match range.end().as_ref() {
                            crate::parser::Expr::Literal(val) => val.to_string(),
                            _ => { String::from("1") }
                        };
                        let incl = match range.inclusive() {
                            true => " + 1",
                            false => "",
                        };

                        if start == end
                        {
                            self.out.push_str(format!("\t{} {}[{}];\n", ty, var.identifier(), step).as_str());

                            match par {
                                Parallelism::CPU => {
                                    self.out.push_str("\t#pragma omp parallel for\n");
                                },
                                Parallelism::GPU => {},
                                Parallelism::None => {},
                            };

                            self.out.push_str(format!("\tfor (int i = 0; i < {}; i++) {}\n", step, "{").as_str());
                            self.out.push_str(format!("\t\t{}[i] = {};\n", var.identifier(), start).as_str());
                            self.out.push_str(format!("\t\tprintf(\"%f\\n\", {}[i]);\n", var.identifier()).as_str());
                            self.out.push_str("\t}\n\n");
                        }
                        else {
                            self.out.push_str(format!("\tint sz_{} = (int)floor(({} - {}) / {}){};\n", var.identifier(), end, start, step, incl).as_str());
                            self.out.push_str(format!("\t{}* {} = malloc(sz_{} * sizeof({}));\n", ty, var.identifier(), var.identifier(), ty).as_str());
                            
                            match par {
                                Parallelism::CPU => {
                                    self.out.push_str("\t#pragma omp parallel for\n");
                                },
                                Parallelism::GPU => {},
                                Parallelism::None => {},
                            };
                            
                            self.out.push_str(format!("\tfor (int i = 0; i < sz_{}; i++) {}\n", var.identifier(), "{").as_str());
                            self.out.push_str(format!("\t\t{}[i] = {} + i * {};\n", var.identifier(), start, step).as_str());
                            self.out.push_str("\t}\n\n");
                        }
                        
                    },
                    _ => {
                        self.out.push_str(format!("\t{}{} {} = {};\n\n", mutable, ty, var.identifier(), self.gen_expr(e)).as_str());
                    }
                }
            }
            Value::StringLiteral(s) => {
                self.out.push_str(format!("\tconst char* {} = \"{}\";\n\n", var.identifier(), s).as_str());
            },
            Value::Boolean(b) => {
                self.out.push_str(format!("{}bool {} = {}", mutable, var.identifier(), b).as_str());
            }
            Value::Null => {

            },
        }
    }

    fn gen_expr(&self, expr: &Expr) -> String
    {
        let mut res: String = String::new();
        
        match expr {
            Expr::Identifier(id) => res = id.to_string(),
            Expr::Literal(val) => res = val.to_string(),
            Expr::Binary(binary) => {
                let op = match binary.op()
                {
                    Operator::Add => "+",
                    Operator::Sub => "-",
                    Operator::Mul => "*",
                    Operator::Div => "/",
                    _ => "" // unsupported operator
                };
                
                let l = self.gen_expr(binary.left());
                let r = self.gen_expr(binary.right());

                res = format!("{} {} {}", l, op, r);
            },
            Expr::Grouped(e) => {
                res = format!("({})", self.gen_expr(e));
            }
            _ => {}
        };

        res
    }
}

impl Backend for CGenerator {
    fn process(&mut self, prog: &Program) {
        self.out.push_str("#include <stdio.h>\n");
        self.out.push_str("#include <stdlib.h>\n");
        self.out.push_str("#include <math.h>\n");
        self.out.push_str("#include <stdbool.h>\n\n");

        self.out.push_str("int main()\n");
        self.out.push_str("{\n");

        for item in prog.items()
        {
            match item {
                crate::parser::Items::Var(var) => {
                    self.gen_var(var);
                },
            }
        }
        
        self.out.push_str("\treturn 0;\n");
        self.out.push_str("}\n");
    }

    fn output(&self) -> &String
    {
        &self.out
    }
}