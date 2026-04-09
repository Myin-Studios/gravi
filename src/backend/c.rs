use colored::Colorize;

use crate::{backend::Backend, error::{NyonError, Reporter}, lexer::{Operator, Type}, ast::*};

pub struct CGenerator
{
    out: String,
    rep: Reporter,
    block_counter: usize,
    name_map: Vec<(String, String)>,
}

impl CGenerator {
    pub fn new() -> Self
    {
        Self
        {
            out: String::new(),
            rep: Reporter::new(),
            block_counter: 0,
            name_map: Vec::new()
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
                "char*".to_string()
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

    fn get_set_mangled(&mut self, name: &String) -> String
    {
        let mut id: Option<String> = None;

        id = self.name_map.iter()
                     .find(|(orig, _)| orig == name)
                     .map(|(_, mangled)| mangled.clone());

        if id.is_none()
        {
            let mangled_name = format!("__b{}_{}", self.block_counter, name);
            self.name_map.push((name.to_string(), mangled_name.clone()));
            id = Some(mangled_name);
        }
                     
        id.unwrap_or(name.clone())
    }

    fn gen_var(&mut self, var: &VarDecl) -> String
    {
        let mut res: String = String::new();

        let par = var.parallelism();

        let ty = self.get_type(var.ty());
        
        let mutable = if var.mutable()
        {
            ""
        }
        else {
            "const "
        };

        match var.value().as_ref().unwrap_or(&crate::ast::Value::Null) {
            Value::Expression(e) => {
                match e {
                    crate::ast::Expr::Range(range) => {
                        let start = match range.start().as_ref() {
                            crate::ast::Expr::Literal(val) => val.to_string(),
                            _ => { String::from("0") }
                        };
                        let step = match range.step().as_ref().unwrap_or(&Box::new(Expr::Literal("1".to_string()))).as_ref() {
                            crate::ast::Expr::Literal(val) => val.to_string(),
                            _ => { String::from("1") }
                        };
                        let end = match range.end().as_ref() {
                            crate::ast::Expr::Literal(val) => val.to_string(),
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
                            res.push_str("\t}\n");
                        }
                        
                    },
                    _ => {
                        res.push_str(format!("\t{}{} {} = {};\n", mutable, ty, var.identifier(), self.gen_expr(e)).as_str());
                    }
                }
            }
            Value::StringLiteral(s) => {
                res.push_str(format!("\tconst char* {} = \"{}\";\n", var.identifier(), s).as_str());
            },
            Value::Boolean(b) => {
                let bv = if b == &BoolValue::True
                {
                    "true"
                }
                else {
                    "false"
                };
                
                res.push_str(format!("\t{}bool {} = {};\n", mutable, var.identifier(), bv).as_str());
            }
            Value::Null => {

            },
            Value::Call(_, _) => {},
            Value::Block(items) => {
                let (val, id) = self.gen_block(items);
                res.push_str(&format!("{}", val));
                res.push_str(&format!("\t{}{} {} = {};\n", mutable, ty, var.identifier(), id));
            },
            Value::IfElse(if_else) => {},
        }

        res
    }

    fn gen_expr(&mut self, expr: &Expr) -> String
    {
        let mut res: String = String::new();
        
        match expr {
            Expr::Identifier(id) => res = self.get_set_mangled(id),
            Expr::Literal(val) => res = val.to_string(),
            Expr::Binary(b) => {
                let op = match b.op()
                {
                    Operator::Add => "+",
                    Operator::Sub => "-",
                    Operator::Mul => "*",
                    Operator::Div => "/",
                    _ => "" // unsupported operator
                };
                
                let l = self.gen_expr(b.left());
                let r = self.gen_expr(b.right());

                res = format!("{} {} {}", l, op, r);
            },
            Expr::Grouped(e) => {
                res = format!("({})", self.gen_expr(e));
            },
            Expr::Boolean(b) => {
                let op = match b.op()
                {
                    Operator::LAnd => "&&",
                    Operator::LOr => "||",
                    _ => "" // unsupported operator
                };
                
                let l = self.gen_expr(b.left());
                let r = self.gen_expr(b.right());

                res = format!("{} {} {}", l, op, r);
            },
            Expr::Unary(u) => {
                let op = match u.op()
                {
                    Operator::LNot => "!",
                    Operator::Sub => "-",
                    _ => "" // unsupported operator
                };
                
                let r = self.gen_expr(u.right());

                res = format!("{}{}", op, r);
            },
            _ => {
                self.rep.add(NyonError::throw(crate::error::Kind::UnsupportedExpression)
                                        .hint("Try writing a valid expression, like:\n\t- a binary expression: \"val1 op val2\"\n\t- a grouped expression \"(val1 op val2)\"\n\t- a boolean expression: \"a || b\" or \"a && b\"\n\t- a range: \"start:step:end\" (exclusive) or \"start:step::end\" (inclusive)\n\t- an identifier: named variable\n\t- a numeric literal: 1, 2, ... n or 1.x, 2.x, ..., n.x"));
            }
        };

        res
    }

    fn gen_block(&mut self, items: &Vec<Items>) -> (String, String)
    {
        let len = self.name_map.len();
        self.block_counter += 1;

        let mut res = String::new();
        let mut id = String::new();

        for item in items
        {
            match item {
                Items::Ret(val) => {
                    match val {
                        Value::Expression(Expr::Identifier(name)) => {
                            id = self.get_set_mangled(name);
                        },
                        Value::Expression(expr) => {
                            id = self.gen_expr(expr);
                        },
                        Value::Block(vals) => {
                            let (inner_res, inner_id) = self.gen_block(vals);
                            res.push_str(&inner_res);
                            id = inner_id;
                        },
                        _ => {}
                    }
                },
                Items::Var(v) => {
                    let mut mangled = v.clone();
                    mangled.id = self.get_set_mangled(&v.id);
                    res.push_str(&self.gen_var(&mangled));
                },
                Items::Expr(expr) => {
                    match expr {
                        Value::Block(_) => {}, // error! block inside another block without any return!
                        Value::Expression(Expr::Identifier(name)) => {
                            id = self.get_set_mangled(name);
                        },
                        Value::Expression(expr) => {
                            id = self.gen_expr(expr);
                        },
                        Value::StringLiteral(s) => id = s.to_string(),
                        Value::Boolean(b) => {
                            if b == &BoolValue::True { id = "true".to_string() } else {id = "false".to_string(); }
                        },
                        Value::Call(call_id, vals) => {
                            if call_id == "show"
                            {
                                res = self.gen_show(vals)
                            }
                            else {
                                res = format!("\tnn_{}({});\n", call_id, self.gen_call(vals));
                            }
                        },
                        Value::IfElse(ifelse) => {
                            res.push_str(&self.gen_if(&ifelse));
                        }
                        Value::Null => {},
                    }
                },
                _ => {}
            }
        }

        self.name_map.truncate(len);

        (res, id)
    }

    fn gen_if(&mut self, ifelse: &IfElse) -> String
    {
        let mut res = String::new();

        res.push_str(&format!("\tif ({})", self.gen_expr(ifelse.condition().as_ref().unwrap_or(&Expr::Null))));
        res.push_str("\n\t{\n");
        res.push_str(&format!("\t{}", self.gen_block(ifelse.body()).0));
        res.push_str("\n\t}\n");

        if let Some(elif) = ifelse.else_if()
        {
            res.push_str("\telse ");
            
            if elif.condition().is_some()
            {
                res.push_str(&self.gen_if(elif));
            }
            else {
                res.push_str("\n\t{\n");
                res.push_str(&format!("\t{}", self.gen_block(elif.body()).0));
                res.push_str("\n\t}\n");
            }
        }
        
        res
    }

    fn gen_fun(&mut self, fun: &Function) -> String
    {
        let mut res: String = String::new();

        let ret = if fun.main() { "int".to_string() } else { self.get_type(fun.ret()) };
        let id = if fun.identifier() == "main" { "main".to_string() } else {
            let mut i = String::from("nn_");
            i.push_str(fun.identifier());
            i
        };
        
        let mut params: String = String::new();

        if !fun.main()
        {
            for i in 0..fun.params().len()
            {
                let m = if fun.params()[i].mutable() { "" } else { "const " };
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
            let mut bd = " {\n\n".to_string();

            bd.push_str(&self.gen_block(fun.body()).0);
            
            bd.push_str("\n}");
            
            bd
        };

        res.push_str(format!("{} {}({}){}\n\n", ret, id, params, body).as_str());

        res
    }

    fn gen_call(&mut self, vals: &Vec<Value>) -> String
    {
        let mut res: String = String::new();

        for (i, val) in vals.into_iter().enumerate()
        {
            match val {
                Value::Expression(expr) => res.push_str(self.gen_expr(expr).as_str()),
                Value::StringLiteral(s) => res.push_str(format!("\"{}\"", s).as_str()),
                Value::Boolean(b) => {
                    res.push_str(
                        if b == &BoolValue::True
                        {
                            "true"
                        }
                        else {
                            "false"
                        }
                    );
                },
                Value::Call(id, values) => res.push_str(format!("nn_{}({})", id, self.gen_call(values)).as_str()),
                Value::Null => res.push_str(""),
                Value::Block(_) => {}
                Value::IfElse(if_else) => {},
            }

            if i < vals.len() - 1
            {
                res.push_str(", ");
            }
        }

        res
    }

    fn gen_show(&mut self, vals: &Vec<Value>) -> String
    {
        let mut res: String = String::from("\tprintf(");

        for val in vals
        {
            match val {
                Value::Expression(expr) => res.push_str(format!("\"%g\\n\", {}", self.gen_expr(expr)).as_str()),
                Value::StringLiteral(s) => res.push_str(format!("\"%s\\n\", \"{}\"", s).as_str()),
                Value::Boolean(b) => {
                    if b == &BoolValue::True
                    {
                        res.push_str(format!("\"%s\\n\", \"true\"").as_str())
                    }
                    else {
                        res.push_str(format!("\"%s\\n\", \"false\"").as_str())
                    }
                },
                Value::Call(id, values) => res.push_str(format!("\"%g\\n\", nn_{}({})", id, self.gen_call(values)).as_str()),
                Value::Null => res.push_str("\"\""),
                Value::Block(_) => {}
                Value::IfElse(if_else) => {},
            }
        }

        res.push_str(");\n");

        res
    }

    fn gen_ret(&mut self, val: &Value) -> String
    {
        let mut res: String = String::new();

        res.push_str(format!("\treturn {};\n", match val {
            Value::Expression(expr) => self.gen_expr(expr),
            Value::StringLiteral(str) => str.to_string(),
            Value::Boolean(b) => {
                if b == &BoolValue::True
                {
                    "true".to_string()
                }
                else {
                    "false".to_string()
                }
            },
            Value::Call(_, _) => "".to_string(),
            Value::Null => "".to_string(),
            Value::Block(_) => "".to_string(),
            Value::IfElse(if_else) => "".to_string(),
        }).as_str());

        res
    }
    
    pub fn reporter(&self) -> &Reporter
    {
        &self.rep
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
                crate::ast::Global::Fun(fun) => {
                    is_main = fun.main();

                    let s = self.gen_fun(fun);
                    self.out.push_str(s.as_str());
                },
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