use std::collections::VecDeque;

use crate::{backend::Backend, error::{NyonError, Reporter}, lexer::{Operator, Type}, ast::*};

pub struct CGenerator
{
    out:           String,
    rep:           Reporter,
    block_counter: usize,
    name_map:      Vec<(String, String, Type)>,  // (original, mangled, type)
    inline:        VecDeque<String>,
}

impl CGenerator {
    pub fn new() -> Self
    {
        Self
        {
            out:           String::new(),
            rep:           Reporter::new(),
            block_counter: 0,
            name_map:      Vec::new(),
            inline:        VecDeque::new(),
        }
    }

    fn get_type(&self, ty: &Type) -> String
    {
        match ty {
            crate::lexer::Type::Numeric(numeric) => {
                match numeric {
                    crate::lexer::Numeric::U8  => "unsigned char".to_string(),
                    crate::lexer::Numeric::U16 => "unsigned short".to_string(),
                    crate::lexer::Numeric::U32 => "unsigned int".to_string(),
                    crate::lexer::Numeric::U64 => "unsigned long".to_string(),
                    crate::lexer::Numeric::I8  => "char".to_string(),
                    crate::lexer::Numeric::I16 => "short".to_string(),
                    crate::lexer::Numeric::I32 => "int".to_string(),
                    crate::lexer::Numeric::I64 => "long".to_string(),
                    crate::lexer::Numeric::F16 => "float".to_string(), // C has no native f16
                    crate::lexer::Numeric::F32 => "float".to_string(),
                    crate::lexer::Numeric::F64 => "double".to_string(),
                }
            },
            crate::lexer::Type::StringLiteral => "char*".to_string(),
            crate::lexer::Type::Boolean       => "bool".to_string(),
            crate::lexer::Type::Character     => "char".to_string(),
            crate::lexer::Type::Custom(c)     => c.to_string(),
            crate::lexer::Type::None          => "void".to_string(),
        }
    }

    fn preprocess(&mut self, input: &Program) -> String
    {
        let mut res = String::new();

        for item in input.items()
        {
            match item {
                Global::Fun(fun) => {
                    for elem in fun.body()
                    {
                        match elem {
                            Items::Var(Var::Decl(decl)) => {
                                if let Some(v) = decl.value()
                                {
                                    match v {
                                        Value::IfElse(_) | Value::Block(_, _) => {
                                            res.push_str(&self.pregen_lambda(v));
                                        },
                                        _ => {}
                                    }
                                }
                            },
                            _ => {}
                        }
                    }
                }
            }
        }

        res
    }

    fn pregen_lambda(&mut self, val: &Value) -> String
    {
        let mut res = String::new();

        match val {
            Value::Block(ty, items) => {
                let id = format!("__nn_inline_block{}", self.block_counter);
                self.inline.push_back(id.clone());
                res.push_str(&format!("static inline {} {}() {{\n{}\n}}\n",
                    self.get_type(ty), id, self.gen_block(items).0));
            },
            Value::IfElse(ifelse) => {
                let ret = self.get_type(ifelse.ret.as_ref().unwrap_or(&Type::None));

                let if_id = format!("__nn_inline_if{}", self.block_counter);
                self.inline.push_back(if_id.clone());
                res.push_str(&format!("static inline {} {}() {{\n{}\n}}\n",
                    ret, if_id, self.gen_block(ifelse.body()).0));

                if let Some(elif) = ifelse.else_if()
                {
                    let else_id = format!("__nn_inline_if{}", self.block_counter);
                    self.inline.push_back(else_id.clone());
                    res.push_str(&format!("static inline {} {}() {{\n{}\n}}\n",
                        ret, else_id, self.gen_block(elif.body()).0));
                }
            },
            _ => {}
        }

        res
    }

    fn get_set_mangled(&mut self, name: &str) -> String
    {
        if let Some((_, mangled, _)) = self.name_map.iter().find(|(orig, _, _)| orig == name)
        {
            return mangled.clone();
        }

        let mangled = format!("__b{}_{}", self.block_counter, name);
        self.name_map.push((name.to_string(), mangled.clone(), Type::None));
        mangled
    }

    fn register_var(&mut self, name: &str, ty: Type) -> String
    {
        if let Some((_, mangled, _)) = self.name_map.iter().find(|(orig, _, _)| orig == name)
        {
            return mangled.clone();
        }

        let mangled = format!("__b{}_{}", self.block_counter, name);
        self.name_map.push((name.to_string(), mangled.clone(), ty));
        mangled
    }

    fn type_of_var(&self, name: &str) -> Type
    {
        self.name_map.iter()
            .find(|(orig, _, _)| orig == name)
            .map(|(_, _, ty)| ty.clone())
            .unwrap_or(Type::None)
    }

    fn printf_fmt(ty: &Type) -> &'static str
    {
        match ty {
            Type::Numeric(n) => match n {
                crate::lexer::Numeric::F16 | crate::lexer::Numeric::F32 => "%g",
                crate::lexer::Numeric::F64 => "%lg",
                crate::lexer::Numeric::U8
                | crate::lexer::Numeric::U16
                | crate::lexer::Numeric::U32 => "%u",
                crate::lexer::Numeric::U64 => "%lu",
                crate::lexer::Numeric::I8
                | crate::lexer::Numeric::I16
                | crate::lexer::Numeric::I32 => "%d",
                crate::lexer::Numeric::I64 => "%ld",
            },
            Type::StringLiteral => "%s",
            Type::Character     => "%c",
            _ => "%s",
        }
    }

    fn gen_var(&mut self, var: &VarDecl) -> String
    {
        let mut res: String = String::new();

        let par = var.parallelism();
        let ty  = self.get_type(var.ty());

        let mutable = if var.mutable() { "" } else { "const " };

        if let Some(val) = var.value()
        {
            match val {
                Value::Expression(e) => {
                    match e {
                        crate::ast::Expr::Range(range) => {
                            let start = match range.start() {
                                crate::ast::Expr::Literal(val) => val.to_string(),
                                _ => String::from("0"),
                            };
                            let step = match range.step() {
                                Some(crate::ast::Expr::Literal(val)) => val.to_string(),
                                _ => String::from("1"),
                            };
                            let end = match range.end() {
                                crate::ast::Expr::Literal(val) => val.to_string(),
                                _ => String::from("1"),
                            };
                            let incl = if range.inclusive() { " + 1" } else { "" };

                            if start == end
                            {
                                res.push_str(&format!("\t{} {}[{}];\n", ty, var.identifier(), step));

                                match par {
                                    Parallelism::CPU => res.push_str("\t#pragma omp parallel for\n"),
                                    _ => {}
                                };

                                res.push_str(&format!("\tfor (int i = 0; i < {}; i++) {{\n", step));
                                res.push_str(&format!("\t\t{}[i] = {};\n", var.identifier(), start));
                                res.push_str(&format!("\t\tprintf(\"%f\\n\", {}[i]);\n", var.identifier()));
                                res.push_str("\t}\n\n");
                            }
                            else {
                                res.push_str(&format!("\tint sz_{} = (int)floor(({} - {}) / {}){};\n",
                                    var.identifier(), end, start, step, incl));
                                res.push_str(&format!("\t{}* {} = malloc(sz_{} * sizeof({}));\n",
                                    ty, var.identifier(), var.identifier(), ty));

                                match par {
                                    Parallelism::CPU => res.push_str("\t#pragma omp parallel for\n"),
                                    _ => {}
                                };

                                res.push_str(&format!("\tfor (int i = 0; i < sz_{}; i++) {{\n", var.identifier()));
                                res.push_str(&format!("\t\t{}[i] = {} + i * {};\n", var.identifier(), start, step));
                                res.push_str("\t}\n");
                            }
                        },
                        _ => {
                            res.push_str(&format!("\t{}{} {} = {};\n",
                                mutable, ty, var.identifier(), self.gen_expr(e)));
                        }
                    }
                },
                Value::StringLiteral(s) => {
                    res.push_str(&format!("\tconst char* {} = \"{}\";\n", var.identifier(), s));
                },
                Value::Boolean(b) => {
                    let bv = if b == &BoolValue::True { "true" } else { "false" };
                    res.push_str(&format!("\t{}bool {} = {};\n", mutable, var.identifier(), bv));
                },
                Value::Null => {},
                Value::Call(_, _) => {},
                Value::Block(_, _) => {
                    let blk = self.inline.pop_front().unwrap_or_default();
                    res.push_str(&format!("\t{}{} {} = {}();\n", mutable, ty, var.identifier(), blk));
                },
                Value::IfElse(ifelse) => {
                    res.push_str(&format!("\t{}{} {} = {};\n",
                        mutable, ty, var.identifier(), self.gen_if_ternary(ifelse)));
                },
                Value::Loop(_) => {},
            }
        }
        else {
            res.push_str(&format!("\t{}{} {};\n", mutable, ty, var.identifier()));
        }

        res
    }

    fn gen_expr(&mut self, expr: &Expr) -> String
    {
        let mut res: String = String::new();

        match expr {
            Expr::Identifier(id) => res = self.get_set_mangled(id),
            Expr::Literal(val)   => res = val.to_string(),
            Expr::Binary(b) => {
                let op = match b.op() {
                    Operator::Add => "+",
                    Operator::Sub => "-",
                    Operator::Mul => "*",
                    Operator::Div => "/",
                    Operator::Mod => "%",
                    Operator::Pow => "^",
                    _ => ""
                };
                let l = self.gen_expr(b.left());
                let r = self.gen_expr(b.right());
                res = format!("{} {} {}", l, op, r);
            },
            Expr::Grouped(e) => {
                res = format!("({})", self.gen_expr(e));
            },
            Expr::Boolean(b) => {
                let op = match b.op() {
                    Operator::LAnd => "&&",
                    Operator::LOr  => "||",
                    Operator::NEq => "!=",
                    Operator::Eq => "==",
                    Operator::LE => "<=",
                    Operator::GE => ">=",
                    Operator::L => "<",
                    Operator::G => ">",
                    Operator::BWAnd => "&",
                    Operator::BWOr => "|",
                    _ => ""
                };
                let l = self.gen_expr(b.left());
                let r = self.gen_expr(b.right());
                res = format!("{} {} {}", l, op, r);
            },
            Expr::Unary(u) => {
                let op = match u.op() {
                    Operator::LNot => "!",
                    Operator::Sub  => "-",
                    _ => ""
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

    fn gen_block(&mut self, items: &[Items]) -> (String, String)
    {
        let len = self.name_map.len();
        self.block_counter += 1;

        let mut res = String::new();
        let mut id  = String::new();

        for item in items
        {
            match item {
                Items::Ret(val) => {
                    res.push_str(&self.gen_ret(val));
                },
                Items::Var(v) => {
                    match v {
                        Var::Decl(v) => {
                            let mut mangled = v.clone();
                            mangled.id = self.register_var(&v.id, v.ty().clone());
                            res.push_str(&self.gen_var(&mangled));
                        },
                        Var::Var(v) => {
                            if let Some(val) = &v.val
                            {
                                let mangled = self.get_set_mangled(&v.name);
                                res.push_str(&format!("\t{} = {};\n", mangled, self.gen_val(val)));
                            }
                        }
                    }
                },
                Items::Expr(expr) => {
                    match expr {
                        Value::Block(_, _) => {}, // block without return — nothing to emit
                        Value::Expression(Expr::Identifier(name)) => {
                            id = self.get_set_mangled(name);
                        },
                        Value::Expression(expr) => {
                            id = self.gen_expr(expr);
                        },
                        Value::StringLiteral(s) => id = s.to_string(),
                        Value::Boolean(b) => {
                            id = if b == &BoolValue::True { "true".to_string() } else { "false".to_string() };
                        },
                        Value::Call(call_id, vals) => {
                            if call_id == "show" {
                                res.push_str(&self.gen_show(vals));
                            } else {
                                res.push_str(&format!("\tnn_{}({});\n", call_id, self.gen_call(vals)));
                            }
                        },
                        Value::IfElse(ifelse) => {
                            res.push_str(&self.gen_if(ifelse));
                        },
                        Value::Null => {},
                        Value::Loop(l) => {
                            if let Some(cond) = l.cond.clone()
                            {
                                let mut start = String::new();
                                let mut end   = String::new();
                                let mut is_inclusive = false;

                                if let Some(val) = cond.value()
                                {
                                    match val {
                                        Value::Expression(expr) => {
                                            match expr {
                                                Expr::Range(r) => {
                                                    start = self.gen_expr(r.start());
                                                    end = self.gen_expr(r.end());
                                                    is_inclusive = r.inclusive();
                                                },
                                                _ => {}
                                            }
                                        }
                                        _ => {} // error?
                                    }
                                }

                                let mut s: i32 = start.parse().unwrap_or(0);
                                let mut e: i32 = end.parse().unwrap_or(0);
                                
                                if !is_inclusive
                                {
                                    if s < e { e = e - 1; }
                                    else if e < s { s = s - 1; }
                                }

                                let id = self.register_var(cond.identifier(), cond.ty().to_owned());

                                if s < e
                                {
                                    res.push_str(&format!("\tfor (int {} = {}; {} <= {}; {}++)\n", id, s,
                                                                                                      id, e,
                                                                                                      id
                                                                                                    ));
                                    res.push_str(&format!("\t{{\n\t{}\n\t}}\n", self.gen_block(&l.body).0));
                                }
                                else {
                                    res.push_str(&format!("\tfor (int {} = {}; {} >= {}; {}--)\n", id, s,
                                                                                                      id, e,
                                                                                                      id
                                                                                                    ));
                                    res.push_str(&format!("\t{{\n\t{}\n\t}}\n", self.gen_block(&l.body).0));
                                }
                            }
                            else {
                                res.push_str("\twhile (true)\n");
                                res.push_str(&format!("\t{{\n\t{}\n\t}}\n", self.gen_block(&l.body).0));
                            }
                        },
                    }
                },
                Items::Stop => {
                    res.push_str("\tbreak;\n");
                },
                Items::Skip => {
                    res.push_str("\tcontinue;\n");
                },
            }
        }

        self.name_map.truncate(len);

        (res, id)
    }

    fn gen_val(&mut self, val: &Value) -> String
    {
        let mut res = String::new();

        match val {
            Value::Expression(expr)     => res.push_str(&self.gen_expr(expr)),
            Value::StringLiteral(s)     => res.push_str(&format!("\"{}\"", s)),
            Value::Boolean(b)           => res.push_str(if b == &BoolValue::True { "true" } else { "false" }),
            Value::Call(id, vals)       => res.push_str(&format!("nn_{}({})", id, self.gen_call(vals))),
            Value::Block(_, _)          => res.push_str(&self.inline.pop_front().unwrap_or_default()),
            Value::IfElse(ifelse)       => res.push_str(&self.gen_if_ternary(ifelse)),
            Value::Null                 => {},
            Value::Loop(_) => {},
        }

        res
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

            if elif.condition().is_some() {
                res.push_str(&self.gen_if(elif));
            } else {
                res.push_str("\n\t{\n");
                res.push_str(&format!("\t{}", self.gen_block(elif.body()).0));
                res.push_str("\n\t}\n");
            }
        }

        res
    }

    fn gen_if_ternary(&mut self, ifelse: &IfElse) -> String
    {
        format!("({}) ? {}() : {}()",
            self.gen_expr(ifelse.condition().as_ref().unwrap_or(&Expr::Null)),
            self.inline.pop_front().unwrap_or_default(),
            self.inline.pop_front().unwrap_or_default())
    }

    fn gen_fun(&mut self, fun: &Function) -> String
    {
        let mut res: String = String::new();

        let ret = if fun.main() { "int".to_string() } else { self.get_type(fun.ret()) };
        let id  = if fun.identifier() == "main" {
            "main".to_string()
        } else {
            format!("nn_{}", fun.identifier())
        };

        let mut params = String::new();

        if !fun.main()
        {
            for i in 0..fun.params().len()
            {
                let m   = if fun.params()[i].mutable() { "" } else { "const " };
                let t   = self.get_type(fun.params()[i].ty());
                let pid = fun.params()[i].identifier();
                let v   = fun.params()[i].value();

                if v.is_none() // C doesn't support default values
                {
                    if i == fun.params().len() - 1 {
                        params.push_str(&format!(" {}{} {}", m, t, pid));
                    } else {
                        params.push_str(&format!("{}{} {},", m, t, pid));
                    }
                }
            }
        }

        let body = if fun.body().is_empty()
        {
            ";".to_string()
        } else {
            let mut bd = " {\n\n".to_string();
            bd.push_str(&self.gen_block(fun.body()).0);
            bd.push_str("\n}");
            bd
        };

        res.push_str(&format!("{} {}({}){}\n\n", ret, id, params, body));

        res
    }

    fn gen_call(&mut self, vals: &[Value]) -> String
    {
        let mut res: String = String::new();

        for (i, val) in vals.iter().enumerate()
        {
            match val {
                Value::Expression(expr)  => res.push_str(&self.gen_expr(expr)),
                Value::StringLiteral(s)  => res.push_str(&format!("\"{}\"", s)),
                Value::Boolean(b)        => res.push_str(if b == &BoolValue::True { "true" } else { "false" }),
                Value::Call(id, values)  => res.push_str(&format!("nn_{}({})", id, self.gen_call(values))),
                Value::Null        => {},
                Value::Block(_, _) => {},
                Value::IfElse(_)   => {},
                Value::Loop(_)     => {},
            }

            if i < vals.len() - 1 {
                res.push_str(", ");
            }
        }

        res
    }

    fn gen_show(&mut self, vals: &[Value]) -> String
    {
        let mut res: String = String::from("\tprintf(");

        for val in vals
        {
            match val {
                Value::Expression(Expr::Identifier(name)) => {
                    let ty      = self.type_of_var(name);
                    let mangled = self.get_set_mangled(name);

                    if ty == Type::Boolean {
                        res.push_str(&format!("\"%s\\n\", {} ? \"true\" : \"false\"", mangled));
                    } else {
                        let fmt = Self::printf_fmt(&ty);
                        res.push_str(&format!("\"{}\\n\", {}", fmt, mangled));
                    }
                },
                Value::Expression(Expr::Literal(lit)) => {
                    if lit.contains('.') {
                        res.push_str(&format!("\"%g\\n\", {}", lit));
                    } else {
                        res.push_str(&format!("\"%d\\n\", {}", lit));
                    }
                },
                Value::Expression(expr) => {
                    let generated = self.gen_expr(expr);
                    res.push_str(&format!("\"%g\\n\", {}", generated));
                },
                Value::StringLiteral(s) => {
                    res.push_str(&format!("\"%s\\n\", \"{}\"", s));
                },
                Value::Boolean(b) => {
                    let bstr = if b == &BoolValue::True { "true" } else { "false" };
                    res.push_str(&format!("\"%s\\n\", \"{}\"", bstr));
                },
                Value::Call(id, values) => {
                    res.push_str(&format!("\"%g\\n\", nn_{}({})", id, self.gen_call(values)));
                },
                Value::Null        => res.push_str("\"\""),
                Value::Block(_, _) => {},
                Value::IfElse(_)   => {},
                Value::Loop(_)     => {},
            }
        }

        res.push_str(");\n");
        res
    }

    fn gen_ret(&mut self, val: &Value) -> String
    {
        let inner = match val {
            Value::Expression(Expr::Identifier(name)) => self.get_set_mangled(name),
            Value::Expression(expr)                   => self.gen_expr(expr),
            Value::Block(_, vals) => {
                let (_, inner_id) = self.gen_block(vals);
                inner_id
            },
            Value::StringLiteral(s) => format!("\"{}\"", s),
            Value::Boolean(b) => {
                if b == &BoolValue::True { "true".to_string() } else { "false".to_string() }
            },
            Value::Call(_, _)  => String::new(),
            Value::IfElse(_)   => String::new(),
            Value::Null        => String::new(),
            Value::Loop(_)     => String::new(),
        };

        format!("\treturn {};\n", inner)
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

        let preprocessed = self.preprocess(prog);
        self.out.push_str(&preprocessed);

        let mut is_main = false;

        for item in prog.items()
        {
            match item {
                Global::Fun(fun) => {
                    is_main = fun.main();
                    let s = self.gen_fun(fun);
                    self.out.push_str(&s);
                },
            }
        }

        if !is_main
        {
            self.out.push_str("int main()\n{\n");
            self.out.push_str("\treturn 0;\n");
            self.out.push_str("}\n");
        }
    }

    fn output(&self) -> &String
    {
        &self.out
    }
}
