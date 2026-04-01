use crate::{backend::Backend, lexer::{Operator, Type}, parser::{Expr, Function, Parallelism, Program, Value, VarDecl}};

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

    fn get_type(&self, ty: &Type) -> String
    {
        match ty {
            crate::lexer::Type::Numeric(numeric) => {
                match numeric {
                    crate::lexer::Numeric::U8 => "unsigned char".to_string(),
                    crate::lexer::Numeric::U16 => "unsigned short".to_string(),
                    crate::lexer::Numeric::U32 => "unsigned int".to_string(),
                    crate::lexer::Numeric::U64 => "unsigned long".to_string(),
                    crate::lexer::Numeric::I8 => "char".to_string(),
                    crate::lexer::Numeric::I16 => "short".to_string(),
                    crate::lexer::Numeric::I32 => "int".to_string(),
                    crate::lexer::Numeric::I64 => "long".to_string(),
                    crate::lexer::Numeric::F16 => "float".to_string(), // C doesn't have "half" of a float
                    crate::lexer::Numeric::F32 => "float".to_string(),
                    crate::lexer::Numeric::F64 => "double".to_string(),
                }
            },
            crate::lexer::Type::StringLiteral => {
                "string".to_string()
            },
            crate::lexer::Type::Boolean => {
                "bool".to_string()
            },
            crate::lexer::Type::Character => {
                "char".to_string()
            },
            crate::lexer::Type::Custom(c) => {
                c.to_string()
            },
            crate::lexer::Type::None => {
                "void".to_string()
            },
        }
    }

    fn gen_var(&mut self, var: &VarDecl) -> String
    {
        let mut res: String = String::new();

        let par = var.parallelism();

        let ty = self.get_type(var.ty());
        
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
                            res.push_str(format!("\t{} {}[{}];\n", ty, var.identifier(), step).as_str());

                            match par {
                                Parallelism::CPU => {
                                    res.push_str("\t#pragma omp parallel for\n");
                                },
                                Parallelism::GPU => {},
                                Parallelism::None => {},
                            };

                            res.push_str(format!("\tfor (int i = 0; i < {}; i++) {}\n", step, "{").as_str());
                            res.push_str(format!("\t\t{}[i] = {};\n", var.identifier(), start).as_str());
                            res.push_str(format!("\t\tprintf(\"%f\\n\", {}[i]);\n", var.identifier()).as_str());
                            res.push_str("\t}\n\n");
                        }
                        else {
                            res.push_str(format!("\tint sz_{} = (int)floor(({} - {}) / {}){};\n", var.identifier(), end, start, step, incl).as_str());
                            res.push_str(format!("\t{}* {} = malloc(sz_{} * sizeof({}));\n", ty, var.identifier(), var.identifier(), ty).as_str());
                            
                            match par {
                                Parallelism::CPU => {
                                    res.push_str("\t#pragma omp parallel for\n");
                                },
                                Parallelism::GPU => {},
                                Parallelism::None => {},
                            };
                            
                            res.push_str(format!("\tfor (int i = 0; i < sz_{}; i++) {}\n", var.identifier(), "{").as_str());
                            res.push_str(format!("\t\t{}[i] = {} + i * {};\n", var.identifier(), start, step).as_str());
                            res.push_str("\t}\n\n");
                        }
                        
                    },
                    _ => {
                        res.push_str(format!("\t{}{} {} = {};\n\n", mutable, ty, var.identifier(), self.gen_expr(e)).as_str());
                    }
                }
            }
            Value::StringLiteral(s) => {
                res.push_str(format!("\tconst char* {} = \"{}\";\n\n", var.identifier(), s).as_str());
            },
            Value::Boolean(b) => {
                res.push_str(format!("\t{}bool {} = {};\n\n", mutable, var.identifier(), b).as_str());
            }
            Value::Null => {

            },
        }

        res
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

    fn gen_fun(&mut self, fun: &Function) -> String
    {
        let mut res: String = String::new();

        let ret = if *fun.main() { "int".to_string() } else { self.get_type(fun.ret()) };
        let id = fun.identifier();
        
        let mut params: String = String::new();

        if !*fun.main()
        {
            for i in 0..fun.params().len()
            {
                let m = if *fun.params()[i].mutable() { "" } else { "const " };
                let t = self.get_type(&fun.params()[i].ty());
                let id = fun.params()[i].identifier();
                let v = fun.params()[i].value();

                if v.is_none() // C doesn't supports default values
                {
                    if i == fun.params().len() - 1
                    {
                        params.push_str(format!(" {}{} {}", m, t, id).as_str());
                    }
                    else {
                        params.push_str(format!("{}{} {},", m, t, id).as_str());
                    }
                }
            }
        } // if main: ignored parameters for now

        let body = if fun.body().is_empty()
        {
            ";".to_string()
        } else {
            let bd = if *fun.main()
            {
                let mut bd = "{\n\treturn 0;\n".to_string();
                bd.push('}');
                bd
            }
            else {
                " {}".to_string()
            };

            bd
        };

        res.push_str(format!("{} {}({}){}\n\n", ret, id, params, body).as_str());

        res
    }
}

impl Backend for CGenerator {
    fn process(&mut self, prog: &Program) {
        self.out.push_str("#include <stdio.h>\n");
        self.out.push_str("#include <stdlib.h>\n");
        self.out.push_str("#include <math.h>\n");
        self.out.push_str("#include <stdbool.h>\n\n");

        let mut temp: String = String::new();
        let mut is_main: bool = false;

        for item in prog.items()
        {
            match item {
                crate::parser::Items::Var(var) => {
                    temp.push_str(self.gen_var(var).as_str());
                },
                crate::parser::Items::Fun(fun) => {
                    is_main = *fun.main();

                    let s = self.gen_fun(fun);
                    self.out.push_str(s.as_str());
                },
                crate::parser::Items::Ret(val) => {
                    
                },
                crate::parser::Items::None => {} // error! invalid statement, function, class or variable declaration!
            }
        }

        if !is_main
        {
            if !temp.is_empty()
            {
                // error! unsupported global variables!
                
                self.out.push_str("int main()\n{\n");
                self.out.push_str("\treturn 0;\n");
                self.out.push_str("}\n");

                return;
            }

            self.out.push_str("int main()\n{\n");
            self.out.push_str(&temp);
            self.out.push_str("\treturn 0;\n");
            self.out.push_str("}\n");
        }
        
    }

    fn output(&self) -> &String
    {
        &self.out
    }
}