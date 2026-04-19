use std::collections::{HashMap, HashSet, VecDeque};

use colored::Colorize;

use crate::{ast::*, backend::Backend, error::{GraviError, Reporter}, lexer::{Operator, Type}, symbol::{self, SymbolTable}};

pub struct CGenerator
{
    out:           String,
    rep:           Reporter,
    block_counter: usize,
    name_map:      Vec<(String, String, Type, bool)>,  // (original, mangled, type, is_list)
    inline:        VecDeque<String>,
    fun_map:       HashMap<String, Type>,
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
            fun_map:       HashMap::new(),
        }
    }

    fn get_type(&self, ty: &Type) -> String
    {
        match ty {
            crate::lexer::Type::Numeric(numeric) => {
                match numeric {
                    crate::lexer::Numeric::USize => "size_t".to_string(),
                    crate::lexer::Numeric::U8    => "unsigned char".to_string(),
                    crate::lexer::Numeric::U16   => "unsigned short".to_string(),
                    crate::lexer::Numeric::U32   => "unsigned int".to_string(),
                    crate::lexer::Numeric::U64   => "unsigned long".to_string(),
                    crate::lexer::Numeric::I8    => "signed char".to_string(),
                    crate::lexer::Numeric::I16   => "short".to_string(),
                    crate::lexer::Numeric::I32   => "int".to_string(),
                    crate::lexer::Numeric::I64   => "long".to_string(),
                    crate::lexer::Numeric::F16   => "float".to_string(), // C has no native f16
                    crate::lexer::Numeric::F32   => "float".to_string(),
                    crate::lexer::Numeric::F64   => "double".to_string(),
                }
            },
            crate::lexer::Type::StringLiteral => "char*".to_string(),
            crate::lexer::Type::Boolean       => "bool".to_string(),
            crate::lexer::Type::Character     => "char".to_string(),
            crate::lexer::Type::Custom(c)     => c.to_string(),
            crate::lexer::Type::None          => "void".to_string(),
        }
    }

    fn preprocess(&mut self, input: &Program, symbols: &SymbolTable) -> String
    {
        let mut res = String::new();

        for scope in symbols.scopes.clone()
        {
            if let symbol::ScopeKind::Global = scope.kind {
                for (id, sym) in &scope.symbols {
                    if let symbol::Symbol::Function(fun) = sym {
                        if !fun.ext {
                            res.push_str(&format!("{} nn_{}({});\n",
                                self.get_type(&fun.ret),
                                id,
                                self.gen_prototype_params(&fun.params)
                            ));
                        }
                    }
                }
            }
        }

        for scope in symbols.scopes.clone()
        {
            match scope.kind {
                symbol::ScopeKind::Global => {
                    for (id, sym) in scope.symbols
                    {
                        match sym {
                            symbol::Symbol::Function(fun) if !fun.ext => {
                                if let Some(body) = fun.body.clone() {
                                    let params_start = self.name_map.len();

                                    for (_, (name, ty, _, _, list)) in fun.params.iter().enumerate() {
                                        self.register_var(name, ty.clone(), list.clone());
                                    }

                                    let mut helpers = String::new();
                                    for item in &body {
                                        match item {
                                            Items::Var(Var::Decl(decl)) => {
                                                if let Some(v) = decl.value() {
                                                    match v {
                                                        Value::IfElse(_) | Value::Block(_, _) => {
                                                            helpers.push_str(&self.pregen_lambda(v));
                                                        },
                                                        _ => {}
                                                    }
                                                }
                                            },
                                            Items::Var(Var::Var(var)) => {
                                                if let Some(v) = &var.val {
                                                    match v {
                                                        Value::IfElse(_) | Value::Block(_, _) => {
                                                            helpers.push_str(&self.pregen_lambda(v));
                                                        },
                                                        _ => {}
                                                    }
                                                }
                                            },
                                            _ => {}
                                        }
                                    }
                                    res.push_str(&helpers);

                                    self.fun_map.insert(id.clone().to_string(), fun.ret.clone());
                                    res.push_str(&format!("{} nn_{}(", self.get_type(&fun.ret), id));
                                    for (i, (name, ty, mutable, _, list)) in fun.params.iter().enumerate() {
                                        let n = self.get_set_mangled(name);
                                        let mut t = self.get_type(&ty);
                                        if list.clone() { t.push('*'); }

                                        if !mutable { res.push_str("const "); }
                                        res.push_str(&format!("{} {}", t, n));
                                        if i < fun.params.len() - 1 { res.push_str(", "); }
                                    }
                                    res.push_str(")\n");

                                    let bd = self.gen_block(&body).0;
                                    res.push_str(&format!("{{\n{}\n}}\n", bd));

                                    self.name_map.truncate(params_start);
                                }
                            },
                            symbol::Symbol::Function(fun) if fun.ext => {
                                let params_start = self.name_map.len();

                                for (_, (name, ty, _, _, list)) in fun.params.iter().enumerate() {
                                    self.register_var(name, ty.clone(), list.clone());
                                }

                                self.fun_map.insert(id.clone().to_string(), fun.ret.clone());
                                res.push_str(&format!("static inline {} nn_{}(", self.get_type(&fun.ret), id));
                                for (i, (name, ty, mutable, _, list)) in fun.params.iter().enumerate() {
                                    let n = self.get_set_mangled(name);
                                    let mut t = self.get_type(&ty);
                                    if list.clone() { t.push('*'); }

                                    if !mutable { res.push_str("const "); }
                                    res.push_str(&format!("{} {}", t, n));
                                    if i < fun.params.len() - 1 { res.push_str(", "); }
                                }
                                res.push_str(")");
                                
                                res.push_str(&format!(" {{ return {}(", id));

                                for (i, (name, _, _, _, _)) in fun.params.iter().enumerate() {
                                    let n = self.get_set_mangled(name);

                                    res.push_str(&format!("{}", n));
                                    if i < fun.params.len() - 1 { res.push_str(", "); }
                                }
                                
                                res.push_str("); };\n");
                                
                                self.name_map.truncate(params_start);
                            },
                            symbol::Symbol::Variable(_) => {
                                // error!
                            },
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        for item in input.items()
        {
            match item {
                Global::Fun(FunKind::Custom(fun)) | Global::Fun(FunKind::Entry(fun)) => {
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
                            Items::Var(Var::Var(var)) => {
                                if let Some(v) = &var.val
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
                },
                Global::Fun(FunKind::Extern(fun)) => {
                    res.push_str(&format!("static inline {} nn_{}({}) {{ return {}({}); }}\n", self.get_type(fun.ret()),
                                                                                                    fun.identifier(),
                                                                                                    self.gen_params(fun.params()),
                                                                                                    fun.identifier(),
                                                                                                    self.gen_args_from_params(fun.params())));
                }
                Global::Import(_) => {},
                _ => {}
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

                let refs = self.collect_refs(ifelse.body());
                let params = refs.iter().map(|(m, t)| format!("{} {}", self.get_type(t), m)).collect::<Vec<_>>().join(", ");
                let args   = refs.iter().map(|(m, _)| m.clone()).collect::<Vec<_>>().join(", ");

                let id = format!("__nn_inline_if{}", self.block_counter);
                self.inline.push_back(format!("{}({})", id, args));  // call completa, non solo il nome
                res.push_str(&format!("static inline {} {}({}) {{\n{}\n}}\n",
                    ret, id, params, self.gen_block(ifelse.body()).0));

                if let Some(elif) = ifelse.else_if()
                {
                    let refs = self.collect_refs(elif.body());
                    let params = refs.iter().map(|(m, t)| format!("{} {}", self.get_type(t), m)).collect::<Vec<_>>().join(", ");
                    let args   = refs.iter().map(|(m, _)| m.clone()).collect::<Vec<_>>().join(", ");

                    let id = format!("__nn_inline_if{}", self.block_counter);
                    self.inline.push_back(format!("{}({})", id, args));  // call completa, non solo il nome
                    res.push_str(&format!("static inline {} {}({}) {{\n{}\n}}\n",
                        ret, id, params, self.gen_block(elif.body()).0));
                }
            },
            _ => {}
        }

        res
    }

    fn collect_refs(&self, items: &[Items]) -> Vec<(String, Type)> {
        let mut refs = Vec::new();
        for item in items {
            match item {
                Items::Ret(val) | Items::Expr(val) => self.collect_val_refs(val, &mut refs),
                Items::Var(Var::Decl(d)) => { if let Some(v) = d.value() { self.collect_val_refs(v, &mut refs); } },
                Items::Var(Var::Var(v)) => { if let Some(v) = &v.val { self.collect_val_refs(v, &mut refs); } },
                _ => {}
            }
        }
        refs
    }

    fn collect_val_refs(&self, val: &Value, refs: &mut Vec<(String, Type)>) {
        match val {
            Value::Expression(expr) => self.collect_expr_refs(expr, refs),
            Value::Block(_, items) => { self.collect_refs(items); },
            _ => {}
        }
    }

    fn collect_expr_refs(&self, expr: &Expr, refs: &mut Vec<(String, Type)>) {
        match expr {
            Expr::Identifier(id) => {
                if let Some((_, mangled, ty, _)) = self.name_map.iter().find(|(orig, _, _, _)| orig == id) {
                    if !refs.iter().any(|(m, _)| m == mangled) {
                        refs.push((mangled.clone(), ty.clone()));
                    }
                }
            },
            Expr::Binary(b) | Expr::Boolean(b) => {
                self.collect_expr_refs(b.left(), refs);
                self.collect_expr_refs(b.right(), refs);
            },
            Expr::Unary(u)   => self.collect_expr_refs(u.right(), refs),
            Expr::Grouped(e) => self.collect_expr_refs(e, refs),
            _ => {}
        }
    }

    fn get_set_mangled(&mut self, name: &str) -> String
    {
        if let Some((_, mangled, _, _)) = self.name_map.iter().find(|(orig, _, _, _)| orig == name)
        {
            return mangled.clone();
        }

        let mangled = format!("__b{}_{}", self.block_counter, name);
        self.name_map.push((name.to_string(), mangled.clone(), Type::None, false));
        mangled
    }

    fn register_var(&mut self, name: &str, ty: Type, is_list: bool) -> String
    {
        if let Some((_, mangled, _, _)) = self.name_map.iter().find(|(orig, _, _, _)| orig == name)
        {
            return mangled.clone();
        }

        let mangled = format!("__b{}_{}", self.block_counter, name);
        self.name_map.push((name.to_string(), mangled.clone(), ty, is_list));
        mangled
    }

    fn type_of_var(&self, name: &str) -> Type
    {
        self.name_map.iter()
            .find(|(orig, _, _, _)| orig == name)
            .map(|(_, _, ty, _)| ty.clone())
            .unwrap_or(Type::None)
    }

    fn printf_fmt(ty: &Type) -> &'static str
    {
        match ty {
            Type::Numeric(n) => match n {
                crate::lexer::Numeric::USize => "%zu",
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

    fn is_list_var(&self, mangled: &str) -> bool {
        self.name_map.iter()
            .find(|(_, m, _, _)| m == mangled)
            .map(|(_, _, _, is_list)| *is_list)
            .unwrap_or(false)
    }

    fn expand_indices(&mut self, vals: &[&Value]) -> Vec<String> {
        let mut result = Vec::new();
        for v in vals {
            match v {
                Value::Expression(Expr::Range(rng)) => {
                    let s: usize = self.gen_expr(rng.start()).parse().unwrap_or(0);
                    let e: usize = self.gen_expr(rng.end()).parse().unwrap_or(0);
                    let iter: Box<dyn Iterator<Item = usize>> = match (s <= e, rng.inclusive()) {
                        (true,  true)  => Box::new(s..=e),
                        (true,  false) => Box::new(s..e),
                        (false, true)  => Box::new((e..=s).rev()),
                        (false, false) => Box::new((e..s).rev()),
                    };
                    for i in iter { result.push(i.to_string()); }
                },
                _ => result.push(self.gen_val(v)),
            }
        }
        result
    }

    fn gen_list_use_assign(&mut self, id: &str, indices: &[&Value], rhs: &Value) -> String {
        let dest   = self.get_set_mangled(id);
        let lhs    = self.expand_indices(indices);
        let mut res = String::new();

        match rhs {
            Value::List(List::Use(src_id, src_indices, None)) => {
                // list[lhs...] = other[rhs...]
                let src     = self.get_set_mangled(src_id);
                let src_flat: Vec<&Value> = src_indices.iter().flatten().collect();
                let rhs     = self.expand_indices(&src_flat);

                for (l, r) in lhs.iter().zip(rhs.iter()) {
                    res.push_str(&format!("\t{}[{}] = {}[{}];\n", dest, l, src, r));
                }
            },
            Value::Expression(Expr::Range(_)) => {
                // list[start:end] (or list[start::end]) = start:end (or start::end)
                let rhs = self.expand_indices(&[rhs]);
                for (l, r) in lhs.iter().zip(rhs.iter()) {
                    res.push_str(&format!("\t{}[{}] = {};\n", dest, l, r));
                }
            },
            _ => {
                // list[lhs...] = scalar/expr
                let val = self.gen_val(rhs);
                for i in &lhs {
                    res.push_str(&format!("\t{}[{}] = {};\n", dest, i, val));
                }
            }
        }

        res
    }

    fn gen_fun(&mut self, fun: &Function) -> String
    {
        let mut res = String::new();

        self.fun_map.insert(fun.identifier().to_string(), fun.ret().clone());
        res.push_str(&format!("{} nn_{}({})\n{{\n{}\n}}\n", self.get_type(fun.ret()), fun.identifier(), self.gen_params(fun.params()), self.gen_block(fun.body()).0));

        res
    }

    fn gen_prototype_params(&self, params: &[(String, Type, bool, Parallelism, bool)]) -> String
    {
        let mut res = String::new();

        for (i, (_, ty, mutable, _, list)) in params.iter().enumerate()
        {
            let mut t = self.get_type(ty);
            if *list { t.push('*'); }
            let m = if *mutable { "" } else { "const " };
            res.push_str(&format!("{}{}", m, t));
            if i < params.len() - 1 { res.push_str(", "); }
        }

        res
    }

    fn gen_params(&mut self, params: &[VarDecl]) -> String
    {
        let mut res = String::new();
        
        for (i, var) in params.iter().enumerate()
        {
            let mangled = self.register_var(var.identifier(), var.ty().clone(), var.list());
            let mut ty = self.get_type(var.ty());
            
            if var.list() { ty.push('*'); }
            
            let mutable = if var.mutable() { "" } else { "const " };
            
            res.push_str(&format!("{}{} {}", mutable, ty, mangled));
            if i < params.len() - 1 { res.push_str(", "); }
        }

        res
    }

    fn gen_args_from_params(&mut self, args: &[VarDecl]) -> String
    {
        let mut res = String::new();

        for (i, arg) in args.iter().enumerate()
        {
            res.push_str(&format!("{}", self.get_set_mangled(arg.identifier())));

            if i < args.len() - 1 { res.push_str(", "); }
        }

        res
    }

    fn gen_var(&mut self, var: &VarDecl) -> String
    {
        let mut res: String = String::new();

        let par = var.parallelism();
        let mut ty  = self.get_type(var.ty());
        if var.list { ty.push('*'); }

        let mutable = if var.mutable() { "" } else { "const " };

        if var.value().is_none() {
            let zero = match var.ty() {
                Type::Numeric(_)    => " = 0",
                Type::Boolean       => " = false",
                Type::Character     => " = '\\0'",
                Type::StringLiteral => " = \"\\0\"",
                _                   => "",
            };
            res.push_str(&format!("\t{}{} {}{};\n", mutable, ty, self.get_set_mangled(var.identifier()), zero));
        }
        
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
                                let s_val: i64 = start.parse().unwrap_or(0);
                                let e_val: i64 = end.parse().unwrap_or(0);
                                let descending = s_val > e_val;
                                let (bigger, smaller) = if descending { (&start, &end) } else { (&end, &start) };
                                let dir = if descending { "-" } else { "+" };

                                res.push_str(&format!("\tint sz_{} = (int)floor(({} - {}) / {}){};\n",
                                    var.identifier(), bigger, smaller, step, incl));
                                res.push_str(&format!("\t{}* {} = malloc(sz_{} * sizeof({}));\n",
                                    ty, var.identifier(), var.identifier(), ty));

                                match par {
                                    Parallelism::CPU => res.push_str("\t#pragma omp parallel for\n"),
                                    _ => {}
                                };

                                res.push_str(&format!("\tfor (int i = 0; i < sz_{}; i++) {{\n", var.identifier()));
                                res.push_str(&format!("\t\t{}[i] = {} {} i * {};\n", var.identifier(), start, dir, step));
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
                Value::Call(id, val) => { res.push_str(&format!("\t{}{} {} = nn_{}({});\n", mutable, ty, var.identifier(), id, self.gen_call(val))); },
                Value::Block(_, _) => {
                    let blk = self.inline.pop_front().unwrap_or_default();
                    res.push_str(&format!("\t{}{} {} = {}();\n", mutable, ty, var.identifier(), blk));
                },
                Value::IfElse(ifelse) => {
                    res.push_str(&format!("\t{}{} {} = {};\n",
                        mutable, ty, var.identifier(), self.gen_if_ternary(ifelse)));
                },
                Value::Loop(_) => {},
                Value::List(List::Decl(sz, Some(vals))) => {
                    let flat: Vec<&Value> = vals.iter().flatten().collect();
                    let mut all: Vec<String> = Vec::new();
                    let size = self.gen_expr(sz);
                    
                    for v in &flat {
                        match v {
                            Value::List(List::Use(src_id, src_indices, None)) => {
                                let src      = self.get_set_mangled(src_id);
                                let src_flat: Vec<&Value> = src_indices.iter().flatten().collect();
                                for idx in self.expand_indices(&src_flat) {
                                    all.push(format!("{}[{}]", src, idx));
                                }
                            },
                            _ => all.push(self.gen_val(v)),
                        }
                    }

                    let list_val = all.join(", ");

                    res.push_str(&format!("\tsize_t sz_{} = {};\n", var.identifier(), size));
                    if var.ty() == &Type::StringLiteral {
                        res.push_str(&format!("\tchar* {} = (char*)malloc((sz_{} + 1) * sizeof(char));\n", var.identifier(), var.identifier()));
                        res.push_str(&format!("\tmemset({}, ({}), sz_{});\n", var.identifier(), list_val, var.identifier()));
                        res.push_str(&format!("\t{}[sz_{}] = '\\0';\n", var.identifier(), var.identifier()));
                    } else {
                        res.push_str(&format!("\tchar* {} = (char*)malloc((sz_{}) * sizeof(char));\n", var.identifier(), var.identifier()));
                        res.push_str(&format!("\tmemset({}, ({}), sz_{});\n", var.identifier(), list_val, var.identifier()));
                    }
                },
                Value::List(List::Use(id, vals, _)) => {
                    let flat:    Vec<&Value> = vals.iter().flatten().collect();
                    let src      = self.get_set_mangled(id);
                    let expanded = self.expand_indices(&flat);
                    let n        = expanded.len();
                    let list_val = expanded.iter()
                        .map(|idx| format!("{}[{}]", src, idx))
                        .collect::<Vec<_>>()
                        .join(", ");

                    res.push_str(&format!("\tint sz_{} = {};\n", var.identifier(), n));
                    if var.ty() == &Type::StringLiteral {
                        res.push_str(&format!("\tchar {}[{}] = {{{}, '\\0'}};\n", var.identifier(), n + 1, list_val));
                    } else {
                        res.push_str(&format!("\t{} {}[{}] = {{{}}};\n", ty, var.identifier(), n, list_val));
                    }
                },
                _ => {}
            }
        }
        else {
            if self.is_list_var(var.identifier()) {
                res.push_str(&format!("\t{}{}* {} = NULL;\n", mutable, ty, var.identifier()));
            } else {
                res.push_str(&format!("\t{}{} {};\n", mutable, ty, var.identifier()));
            }
        }

        res
    }

    fn gen_expr(&mut self, expr: &Expr) -> String
    {
        let mut res: String = String::new();

        match expr {
            Expr::Identifier(id) => {
                res = self.get_set_mangled(id);
            },
            Expr::Index(id, idx) => {
                let mangled = self.get_set_mangled(id);
                res = format!("{}[{}]", mangled, self.gen_expr(idx));
            },
            Expr::Call(id, vals) => res = format!("nn_{}({})", id, self.gen_call(vals)),
            Expr::StringLiteral(val)   => res = format!("\"{}\"", val.to_string()),
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
            Expr::CharLiteral(c) => res = format!("'{}'", c.to_string()),
            Expr::Range(_) => {} // just ignore this. it's handled separately
            _ => {
                self.rep.add(GraviError::throw(crate::error::Kind::UnsupportedExpression)
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

        let future_list_names: HashSet<String> = items.iter()
        .filter_map(|item| match item {
            Items::Var(Var::Var(v)) if matches!(&v.val, Some(Value::List(_))) => {
                Some(v.name.clone())
            },
            _ => None,
        })
        .collect();
        
        for item in items
        {
            match item {
                Items::Ret(val) => {
                    res.push_str(&self.gen_ret(val));
                },
                Items::Var(v) => {
                    match v {
                        Var::Decl(v) => {
                            let _is_list = if let Some(val) = &v.val
                            {
                                if matches!(val, Value::List(_))
                                {
                                    true
                                }
                                else {
                                    false
                                }
                            }
                            else {
                                false
                            };

                            let mut mangled = v.clone();
                            let is_list = if let Some(val) = &v.val {
                                matches!(val, Value::List(_)) || matches!(val, Value::Expression(crate::ast::Expr::Range(_)))
                            } else {
                                future_list_names.contains(&v.id)
                            };
                            mangled.id = self.register_var(&v.id, v.ty().clone(), is_list);
                            res.push_str(&self.gen_var(&mangled));
                        },
                        Var::Var(v) => {
                            if let Some(val) = &v.val {
                                let mangled = self.get_set_mangled(&v.name);
                                match val {
                                    Value::List(List::Decl(_, Some(values))) => {
                                        let ty = self.get_type(&self.type_of_var(&v.name)); // ← v.name = originale
                                        let vals_str = values.iter().flatten()
                                            .map(|v| self.gen_val(v))
                                            .collect::<Vec<_>>()
                                            .join(", ");
                                        res.push_str(&format!("\t{} = ({}[]){{{}}};\n", mangled, ty, vals_str));
                                    },
                                    _ => res.push_str(&format!("\t{} = {};\n", mangled, self.gen_val(val))),
                                }
                            }
                        }
                    }
                },
                Items::Expr(expr) => {
                    match expr {
                        Value::Char(_) => {},
                        Value::Block(_, _) => {},
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
                                if let Some(val) = cond.value()
                                {
                                    match val {
                                        Value::Expression(expr) => {
                                            match expr {
                                                Expr::Range(r) => {
                                                    let start = self.gen_expr(r.start());
                                                    let end   = self.gen_expr(r.end());
                                                    let id    = self.register_var(cond.identifier(), cond.ty().to_owned(), false);

                                                    let (cond_asc, cond_desc) = if r.inclusive() {
                                                        (format!("{} <= {}", id, end), format!("{} >= {}", id, end))
                                                    } else {
                                                        (format!("{} < {}",  id, end), format!("{} > {}",  id, end))
                                                    };

                                                    res.push_str(&format!(
                                                        "\tfor (int {} = {}; ({} <= {}) ? ({}) : ({}); ({} <= {}) ? {}++ : {}--)\n\t{{\n{}\n\t}}\n",
                                                        id, start,
                                                        start, end, cond_asc, cond_desc,
                                                        start, end, id, id,
                                                        self.gen_block(&l.body).0
                                                    ));
                                                },
                                                Expr::Boolean(_) => {
                                                    res.push_str(&format!("\twhile ({})\n\t{{\n\t{}\n\t}}", self.gen_expr(expr), self.gen_block(&l.body).0));
                                                },
                                                _ => {}
                                            }
                                        }
                                        _ => {} // error?
                                    }
                                }
                            }
                            else {
                                res.push_str("\twhile (true)\n");
                                res.push_str(&format!("\t{{\n\t{}\n\t}}\n", self.gen_block(&l.body).0));
                            }
                        },
                        Value::List(List::Use(id, indices, Some(rhs))) => {
                            let flat: Vec<&Value> = indices.iter().flatten().collect();
                            res.push_str(&self.gen_list_use_assign(id, &flat, rhs));
                        },
                        Value::List(_) => {},
                        // _ => {}
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
            Value::List(List::Use(id, vals, None)) => {
                let flat:    Vec<&Value> = vals.iter().flatten().collect();
                let mangled  = self.get_set_mangled(id);
                let expanded = self.expand_indices(&flat);
                res.push_str(
                    &expanded.iter()
                        .map(|idx| format!("{}[{}]", mangled, idx))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            },
            Value::Char(c) => res.push_str(&format!("'{}'", c)),
            _ => {}
        }

        res
    }

    fn gen_if(&mut self, ifelse: &IfElse) -> String
    {
        let mut res = String::new();

        res.push_str(&format!("\tif ({})", self.gen_val(ifelse.condition().as_ref().unwrap_or(&Box::new(Value::Null)))));
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
        format!("({}) ? {} : {}",   // niente () extra
            self.gen_val(ifelse.condition().as_ref().unwrap_or(&Box::new(Value::Null))),
            self.inline.pop_front().unwrap_or_default(),
            self.inline.pop_front().unwrap_or_default())
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
                Value::List(_)     => {},
                Value::Char(c) => { res.push_str(&format!("'{}'", c)); },
            }

            if i < vals.len() - 1 {
                res.push_str(", ");
            }
        }

        res
    }

    fn gen_show(&mut self, vals: &[Value]) -> String {
        let mut res = String::new();

        for val in vals {
            match val {
                Value::Expression(Expr::Identifier(name)) => {
                    let mangled = self.get_set_mangled(name);
                    let ty      = self.type_of_var(name);

                    if self.is_list_var(&mangled) {
                        if ty == Type::StringLiteral {
                            res.push_str(&format!("\tprintf(\"%s\\n\", {});\n", mangled));
                        } else {
                            let fmt = Self::printf_fmt(&ty);
                            res.push_str(&format!(
                                "\tfor (int __i = 0; __i < sz_{}; __i++) {{\n\t\tprintf(\"{}\\n\", {}[__i]);\n\t}}\n",
                                mangled, fmt, mangled
                            ));
                        }
                    } else if ty == Type::Boolean {
                        res.push_str(&format!("\tprintf(\"%s\\n\", {} ? \"true\" : \"false\");\n", mangled));
                    } else {
                        res.push_str(&format!("\tprintf(\"{}\\n\", {});\n", Self::printf_fmt(&ty), mangled));
                    }
                },
                Value::List(List::Use(id, values, _)) => {
                    let mangled  = self.get_set_mangled(id);
                    let var_ty   = self.type_of_var(id);
                    let elem_fmt = if var_ty == Type::StringLiteral { "%c" } else { Self::printf_fmt(&var_ty) };
                    let flat: Vec<&Value> = values.iter().flatten().collect();

                    for v in &flat {
                        res.push_str(&format!("\tprintf(\"{}\\n\", {}[{}]);\n",
                            elem_fmt, mangled, self.gen_val(v)));
                    }
                },
                Value::Expression(Expr::Index(id, idx)) => {
                    let mangled  = self.get_set_mangled(id);
                    let ty       = self.type_of_var(id);
                    let elem_fmt = if ty == Type::StringLiteral { "%c" } else { Self::printf_fmt(&ty) };
                    res.push_str(&format!("\tprintf(\"{}\\n\", {}[{}]);\n", elem_fmt, mangled, self.gen_expr(idx)));
                },
                Value::Expression(Expr::Literal(lit)) => {
                    let fmt = if lit.contains('.') { "%g" } else { "%d" };
                    res.push_str(&format!("\tprintf(\"{}\\n\", {});\n", fmt, lit));
                },
                Value::Expression(expr) => {
                    res.push_str(&format!("\tprintf(\"%g\\n\", {});\n", self.gen_expr(expr)));
                },
                Value::StringLiteral(s) => {
                    res.push_str(&format!("\tprintf(\"%s\\n\", \"{}\");\n", s));
                },
                Value::Boolean(b) => {
                    let bstr = if b == &BoolValue::True { "true" } else { "false" };
                    res.push_str(&format!("\tprintf(\"%s\\n\", \"{}\");\n", bstr));
                },
                Value::Call(id, values) => {
                    let ret_ty = self.fun_map.get(id).cloned().unwrap_or(Type::None);
                    let fmt = Self::printf_fmt(&ret_ty);
                    res.push_str(&format!("\tprintf(\"{}\\n\", nn_{}({}));\n", fmt, id, self.gen_call(values)));
                },
                _ => {}
            }
        }

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
            Value::Call(id, vals) => format!("{}({})", id, self.gen_call(vals)),
            Value::IfElse(_)   => String::new(),
            Value::Null        => String::new(),
            Value::Loop(_)     => String::new(),
            Value::List(_)     => String::new(),
            Value::Char(c) => format!("'{}'", c),
        };

        format!("\treturn {};\n", inner)
    }

    pub fn reporter(&self) -> &Reporter
    {
        &self.rep
    }
}

impl Backend for CGenerator {
    fn process(&mut self, prog: &Program, symbols: &SymbolTable) {
        self.out.push_str("#include <stdio.h>\n");
        self.out.push_str("#include <stdlib.h>\n");
        self.out.push_str("#include <math.h>\n");
        self.out.push_str("#include <stdbool.h>\n\n");
        self.out.push_str("#include <string.h>\n\n");

        let preprocessed = self.preprocess(prog, symbols);
        self.out.push_str(&preprocessed);

        let mut is_main = false;

        for item in prog.items() {
            if let Global::Var(var) = item {
                self.name_map.push((
                    var.id.clone(),
                    var.id.clone(),
                    var.ty().clone(),
                    false,
                ));
            }
        }

        for item in prog.items()
        {
            match item {
                Global::Fun(FunKind::Custom(fun)) => {
                    let s = self.gen_fun(fun);
                    self.out.push_str(&s);
                },
                Global::Fun(FunKind::Entry(fun)) => {
                    is_main = true;
                    let bd = self.gen_block(fun.body()).0;
                    self.out.push_str(&format!("int main()\n{{\n{}\n\treturn 0;\n}}", bd));
                },
                Global::Import(_) => {},
                Global::Var(var) => {
                    let c_ty = self.get_type(var.ty());
                    let mut_kw = if var.mutable() { "" } else { "const " };
                    if let Some(val) = var.value() {
                        let v = self.gen_val(val);
                        self.out.push_str(&format!("{}{} {} = {};\n",
                            mut_kw, c_ty, var.id, v));
                    } else {
                        self.out.push_str(&format!("{}{} {};\n",
                            mut_kw, c_ty, var.id));
                    }
                },
                 _ => {}
            }
        }

        if !is_main
        {
            self.rep.add(GraviError::throw(crate::error::Kind::EntryNotFound)
                                    .severity(crate::error::Severity::Warning)
                                    .hint(&format!("Write one and only one function {}. Not 0, not 2, not N, just 1!\n\tFor now I'll generate it for you... but be careful next time!", "main".bright_blue().bold())));
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
