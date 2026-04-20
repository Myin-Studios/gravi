use colored::Colorize;

pub use crate::ast::*;
use crate::{error::{GraviError, Reporter}, lexer::{Operator, Type}, symbol::{self, SymbolTable, VariableSym}};

/// Distinguishes how numeric bounds should be parsed in `in_range`.
enum NumericKind { Unsigned, Signed, Float }

pub struct Checker
{
    rep:   Reporter,
}

impl Checker {
    pub fn new() -> Self
    {
        Self
        {
            rep:   Reporter::new(),
        }
    }

    pub fn process(&mut self, prog: &mut Program, symbol: &mut SymbolTable)
    {
        let imported = symbol.take();
        for (name, ret, params, mut body) in imported {
            symbol.push(symbol::ScopeKind::Function(name.clone(), ret));
            for (pname, pty, pmut, ppar, list) in &params {
                symbol.add(pname, symbol::Symbol::Variable(VariableSym { ty: pty.clone(), mutable: *pmut, par: ppar.clone(), list: list.clone(), value: None }));
            }
            symbol.push(symbol::ScopeKind::Block);
            self.check_body(&mut body, symbol);
            symbol.pop();
            symbol.pop();
            symbol.restore(&name, body);
        }

        for item in prog.items.iter_mut()
        {
            match item {
                Global::Fun(FunKind::Custom(fun) | FunKind::Extern(fun)) => {
                    symbol.push(crate::symbol::ScopeKind::Function(fun.id.clone(), fun.ret.clone()));
                    self.check_fun(fun, symbol);
                    symbol.pop();
                },
                Global::Fun(FunKind::Entry(fun)) => {
                    if let Some(_) = symbol.find("main")
                    {
                        self.rep.add(GraviError::throw(crate::error::Kind::TooManyEntry)
                                                .hint(&format!("Try writing only one function {}", "main".bright_blue().bold())));
                    }

                    self.check_fun(fun, symbol);
                },
                Global::Var(var) => {
                    symbol.push(symbol::ScopeKind::Global);
                    if let Some(val) = &mut var.val.clone()
                    {
                        self.check_val(val, var.ty(), symbol);
                    }
                },
                _ => {},
            }
        }
    }

    fn check_fun(&mut self, fun: &mut Function, symbol: &mut SymbolTable)
    {
        for param in fun.params()
        {
            symbol.add(param.identifier(), crate::symbol::Symbol::Variable(
                VariableSym
                {
                    mutable: param.mutable(),
                    ty:      param.ty().clone(),
                    par:     param.parallelism().clone(),
                    list:    param.list(),
                    value:   Some(crate::ast::Value::Null), // params are always "initialized" at call time
                }
            ));
        }

        symbol.push(crate::symbol::ScopeKind::Block);
        self.check_body(&mut fun.body, symbol);
        symbol.pop();
    }

    fn check_body(&mut self, items: &mut Vec<Items>, symbol: &mut SymbolTable) -> Type
    {
        let mut ty = Type::None;

        for i in 0..items.len()
        {
            let (back, front) = items.split_at_mut(i);

            match &mut front[0] {
                Items::Var(var) => {
                    match var {
                        Var::Decl(v) => {
                            if v.ty() == &Type::None
                            {
                                if let Some(val) = v.val.as_mut()
                                {
                                    ty = self.check_val(val, &ty, symbol);
                                }

                                v.ty = ty.clone();
                            }
                            else
                            {
                                if let Some(val) = v.val.as_mut()
                                {
                                    let t = self.check_val(&mut val.clone(), v.ty(), symbol);
                                    if t != v.ty().to_owned()
                                    {
                                        self.rep.add(GraviError::throw(crate::error::Kind::TypeMismatch(v.ty.to_owned(), t)));
                                    }
                                }
                            }

                            symbol.add(v.identifier(), crate::symbol::Symbol::Variable(
                                VariableSym
                                {
                                    mutable: v.mutable(),
                                    ty:      v.ty().clone(),
                                    par:     v.parallelism().clone(),
                                    list:    v.list(),
                                    value:   v.value().clone(),
                                }
                            ));
                        },
                        Var::Var(v) => {
                            let name = v.name.clone();
                            let existing = symbol.find(&name).and_then(|s| match s {
                                symbol::Symbol::Variable(var) => Some(var.clone()),
                                _ => None,
                            });

                            if let Some(mut var_sym) = existing {
                                if let Some(val) = v.val.as_mut() {
                                    var_sym.ty = self.check_val(val, &Type::None, symbol);

                                    if var_sym.value.is_none()
                                    {
                                        var_sym.value = Some(val.clone());
                                    }
                                    
                                    if !var_sym.mutable { self.rep.add(GraviError::throw(crate::error::Kind::MutatingImmutable(v.name.clone()))); }
                                }

                                symbol.add(&name, symbol::Symbol::Variable(var_sym.clone()));

                                for prev in back.iter_mut() {
                                    if let Items::Var(Var::Decl(decl)) = prev {
                                        if decl.id == name && decl.ty == Type::None {
                                            decl.ty = var_sym.ty.clone();
                                            break;
                                        }
                                    }
                                }
                            }
                        },
                    }
                },
                Items::Expr(Value::Call(id, vals)) => {
                    let param_types: Vec<Type> = if let Some(symbol::Symbol::Function(f)) = symbol.find(id) {
                        f.params.iter().map(|(_, ty, _, _, _)| ty.clone()).collect()
                    } else {
                        vec![]
                    };
                    
                    self.check_call(vals, &param_types, symbol);
                },
                Items::Expr(Value::Block(_, b)) => {
                    symbol.push(symbol::ScopeKind::Block);
                    self.check_body(b, symbol);
                    symbol.pop();
                },
                Items::Expr(Value::IfElse(ifelse)) => {
                    ty = self.check_if(ifelse, symbol);
                },
                Items::Expr(val) => ty = self.check_val(val, &Type::None, symbol),
                Items::Ret(val) => {
                    let expected = symbol.nearest_fun().cloned().unwrap_or(ty.clone());
                    ty = self.check_val(val, &expected, symbol);
                }
                _ => {}
            }
        }

        ty
    }

    fn check_val(&mut self, val: &mut Value, expected: &Type, symbol: &mut SymbolTable) -> Type
    {
        let mut ty = Type::None;

        match val {
            Value::Expression(expr) => {
                ty = self.check_expr(expr, expected, symbol);
            },
            Value::StringLiteral(_) => {
                ty = Type::StringLiteral;
            },
            Value::Boolean(_) => ty = Type::Boolean,
            Value::Call(id, _) => {
                ty = if let Some(sym) = symbol.find(id)
                {
                    match sym {
                        symbol::Symbol::Function(f) => f.ret.clone(),
                        symbol::Symbol::Variable(_) => { Type::None }
                    }
                } else {
                    Type::None
                }
            },
            Value::Null => ty = Type::None,
            Value::Block(bty, b) => {
                symbol.push(symbol::ScopeKind::Block);
                let inf = self.check_body(b, symbol);
                *bty = inf.clone();
                ty = inf;
                symbol.pop();
            },
            Value::IfElse(ifelse) => {
                ty = self.check_if(ifelse, symbol);
            },
            Value::Loop(_) => {},
            Value::List(List::Decl(index, values)) => {
                self.check_expr(index, &Type::Numeric(crate::lexer::Numeric::USize), symbol);
                if expected == &Type::None
                {
                    if let Some(vals) = values
                    {
                        let mut val: Vec<&mut Value> = vals.iter_mut().flatten().collect();
                        let exp: Type = self.check_val(val[0], expected, symbol);

                        for v in val
                        {
                            ty = self.check_val(v, &exp, symbol);
                        }
                    }
                }
                else {
                    if let Some(vals) = values
                    {
                        for val in vals.iter_mut().flatten()
                        {
                            let t: Type = self.check_val(val, expected, symbol);

                            if !self.is_compatible(&t, expected)
                            {
                                self.rep.add(GraviError::throw(crate::error::Kind::TypeMismatch(expected.clone(), t)));
                                break;
                            }
                        }

                        ty = expected.clone();
                    }
                }
            },
            Value::List(List::Use(id, vals, assigned)) => {
                if let Some(elem) = symbol.find(id)
                {
                    match elem {
                        symbol::Symbol::Variable(var) => {
                            ty = var.ty.clone()
                        },
                        _ => {}
                    }
                }
                else {
                    // error! undeclared variable!
                }

                if ty == Type::StringLiteral && vals[0].len() == 1 && !matches!(vals[0][0], Value::Expression(Expr::Range(_)))
                {
                    ty = Type::Character;
                }

                for val in vals.iter_mut().flatten()
                {
                    match val {
                        Value::Expression(Expr::Identifier(val_id)) => {
                            let mut existing = symbol.find(val_id).and_then(|s| match s {
                                symbol::Symbol::Variable(var) => Some(var.clone()),
                                _ => None,
                            });

                            if let Some(var_sym) = existing.as_mut()
                            {
                                if var_sym.ty == Type::None || var_sym.ty == Type::Numeric(crate::lexer::Numeric::U8) { var_sym.ty = Type::Numeric(crate::lexer::Numeric::USize); }
                            }
                        },
                        _ => {
                            
                        }
                    }
                }

                if let Some(val) = assigned
                {
                    let found = self.check_val(val, expected, symbol);
                    if !self.is_compatible(&found, &ty)
                    {
                        self.rep.add(GraviError::throw(crate::error::Kind::TypeMismatch(found, ty.clone())));
                    }
                }

                let flat: Vec<&mut Value> = vals.iter_mut().flatten().collect();

                for val in flat
                {
                    self.check_val(val, &Type::Numeric(crate::lexer::Numeric::USize), symbol);
                }
            },
            Value::Char(_) => ty = Type::Character,
            // _ => {}
        }

        ty
    }

    fn check_expr(&mut self, expr: &mut Expr, expected: &Type, symbol: &mut SymbolTable) -> Type
    {
        let mut ty = Type::None;

        match expr {
            Expr::Identifier(id) => {
                let sym = symbol.find(id);
                if let Some(s) = sym
                {
                    match s {
                        symbol::Symbol::Variable(var) => {
                            if var.value.is_none()
                            {
                                self.rep.add(GraviError::throw(crate::error::Kind::UninitializedVariable(id.clone())));
                            }

                            ty = var.ty.clone()
                        },
                        _ => {}
                    }
                }

                if *expected != Type::None && !self.is_compatible(&ty, &expected)
                {
                    self.rep.add(GraviError::throw(crate::error::Kind::TypeMismatch(expected.clone(), ty.clone())));
                }
            },
            Expr::Literal(val) => {
                if expected == &Type::Numeric(crate::lexer::Numeric::USize)
                {
                    ty = Type::Numeric(crate::lexer::Numeric::USize);
                }
                else {
                    ty = self.map_numeric(val, expected);
                }
            },
            Expr::Index(id, _) => {
                let existing = symbol.find(id).and_then(|s| match s {
                    symbol::Symbol::Variable(var) => Some(var.clone()),
                    _ => None,
                });
                if let Some(sym) = existing
                {
                    if sym.ty != Type::None { ty = sym.ty; } else {
                        self.rep.add(GraviError::throw(crate::error::Kind::UntypedVariable(id.to_owned())));
                    }
                } else {
                    self.rep.add(GraviError::throw(crate::error::Kind::UndeclaredVariable(id.to_owned())));
                }
            }
            Expr::Range(ran) => {
                ty = self.check_range(ran, expected, symbol);
            },
            Expr::Binary(b) => {
                let l = self.check_val(&mut Value::Expression(b.left().clone()), expected, symbol);
                let mut r = self.check_val(&mut Value::Expression(b.right().clone()), &l, symbol);

                if b.op == Operator::Sub { r = self.sign_converter(&r); }
                else if b.op == Operator::Div { r = Type::Numeric(crate::lexer::Numeric::F32); }

                if !self.is_compatible(&r, &l) && l != Type::None && r != Type::None {
                    self.rep.add(GraviError::throw(crate::error::Kind::TypeMismatch(l.clone(), r.clone())));
                }
                ty = if l != Type::None { l } else { r };
            },
            Expr::Boolean(b) => {
                let l = self.check_val(&mut Value::Expression(b.left().clone()), &Type::None, symbol);
                let r_expected = if l != Type::None { l.clone() } else { Type::None };
                let r = self.check_val(&mut Value::Expression(b.right().clone()), &r_expected, symbol);
                if l != r && l != Type::None && r != Type::None {
                    self.rep.add(GraviError::throw(crate::error::Kind::TypeMismatch(l.clone(), r)));
                }
                ty = Type::Boolean;
            },
            Expr::Grouped(expr) => {
                ty = self.check_expr(expr, expected, symbol);
            },
            Expr::Unary(u) => {
                let mut exp = Type::None;

                if matches!(expected, Type::Numeric(crate::lexer::Numeric::USize |
                                                    crate::lexer::Numeric::U8    |
                                                    crate::lexer::Numeric::U16   |
                                                    crate::lexer::Numeric::U32   |
                                                    crate::lexer::Numeric::U64))
                {
                    exp = self.check_expr(&mut u.right, &Type::None, symbol);
                    if u.op == Operator::Sub { ty = self.sign_converter(&exp); }
                } else {
                    match u.op() {
                        crate::lexer::Operator::LNot => { exp = Type::Boolean; }
                        _ => {} // error?
                    }

                    ty = self.check_expr(&mut u.right, &exp, symbol);
                }
            },
            Expr::Call(id, vals) => {
                let mut params = Vec::new();
                for val in vals.into_iter()
                {
                    params.push(self.check_val(val, &Type::None, symbol));
                }

                self.check_call(vals, &params, symbol);
                
                if let Some(sym) = symbol.find(id)
                {
                    match sym {
                        symbol::Symbol::Function(fun) => {
                            ty = fun.ret.clone()
                        }
                        _ => {}
                    }
                }

                if *expected != Type::None && !self.is_compatible(&ty, &expected)
                {
                    self.rep.add(GraviError::throw(crate::error::Kind::TypeMismatch(expected.clone(), ty.clone())));
                }
            },
            Expr::CharLiteral(_) => ty = Type::Character,
            _ => {}
        }

        ty
    }

    fn sign_converter(&self, ty: &Type) -> Type
    {
        let t: Type;

        match ty {
            Type::Numeric(n) => {
                match n {
                    crate::lexer::Numeric::U8 => t = Type::Numeric(crate::lexer::Numeric::I8),
                    crate::lexer::Numeric::U16 => t = Type::Numeric(crate::lexer::Numeric::I16),
                    crate::lexer::Numeric::U32 => t = Type::Numeric(crate::lexer::Numeric::I32),
                    crate::lexer::Numeric::U64 => t = Type::Numeric(crate::lexer::Numeric::I64),
                    _ => { return ty.to_owned() }
                }
            }
            _ => { return ty.to_owned() }
        }

        t
    }

    fn map_numeric(&self, val: &str, expected: &Type) -> Type
    {
        let mut ty = Type::None;

        if expected == &Type::None
        {
            ty = if self.in_range(val, "0", "255", NumericKind::Unsigned)                             { Type::Numeric(crate::lexer::Numeric::U8) }
                 else if self.in_range(val, "0", "65535", NumericKind::Unsigned)                      { Type::Numeric(crate::lexer::Numeric::U16) }
                 else if self.in_range(val, "0", "4294967295", NumericKind::Unsigned)                 { Type::Numeric(crate::lexer::Numeric::U32) }
                 else if self.in_range(val, "0", "18446744073709551615", NumericKind::Unsigned)        { Type::Numeric(crate::lexer::Numeric::U64) }
                 else if self.in_range(val, "-128", "127", NumericKind::Signed)                       { Type::Numeric(crate::lexer::Numeric::I8) }
                 else if self.in_range(val, "-32768", "32767", NumericKind::Signed)                   { Type::Numeric(crate::lexer::Numeric::I16) }
                 else if self.in_range(val, "-2147483648", "2147483647", NumericKind::Signed)         { Type::Numeric(crate::lexer::Numeric::I32) }
                 else if self.in_range(val, "-9223372036854775808", "9223372036854775807", NumericKind::Signed) { Type::Numeric(crate::lexer::Numeric::I64) }
                 else if self.in_range(val, "-65504.0", "65504.0", NumericKind::Float)                { Type::Numeric(crate::lexer::Numeric::F16) }
                 else if self.in_range(val, "-3.4e38", "3.4e38", NumericKind::Float)                  { Type::Numeric(crate::lexer::Numeric::F32) }
                 else if self.in_range(val, "-1.8e308", "1.8e308", NumericKind::Float)                { Type::Numeric(crate::lexer::Numeric::F64) }
                 else { Type::None }
        }

        match expected {
            Type::Numeric(num) => {
                ty = match num {
                    crate::lexer::Numeric::U8  if self.in_range(val, "0", "255", NumericKind::Unsigned)                      => Type::Numeric(crate::lexer::Numeric::U8),
                    crate::lexer::Numeric::U16 if self.in_range(val, "0", "65535", NumericKind::Unsigned)                    => Type::Numeric(crate::lexer::Numeric::U16),
                    crate::lexer::Numeric::U32 if self.in_range(val, "0", "4294967295", NumericKind::Unsigned)               => Type::Numeric(crate::lexer::Numeric::U32),
                    crate::lexer::Numeric::U64 if self.in_range(val, "0", "18446744073709551615", NumericKind::Unsigned)      => Type::Numeric(crate::lexer::Numeric::U64),
                    crate::lexer::Numeric::I8  if self.in_range(val, "-128", "127", NumericKind::Signed)                     => Type::Numeric(crate::lexer::Numeric::I8),
                    crate::lexer::Numeric::I16 if self.in_range(val, "-32768", "32767", NumericKind::Signed)                 => Type::Numeric(crate::lexer::Numeric::I16),
                    crate::lexer::Numeric::I32 if self.in_range(val, "-2147483648", "2147483647", NumericKind::Signed)       => Type::Numeric(crate::lexer::Numeric::I32),
                    crate::lexer::Numeric::I64 if self.in_range(val, "-9223372036854775808", "9223372036854775807", NumericKind::Signed) => Type::Numeric(crate::lexer::Numeric::I64),
                    crate::lexer::Numeric::F16 if self.in_range(val, "-65504.0", "65504.0", NumericKind::Float)              => Type::Numeric(crate::lexer::Numeric::F16),
                    crate::lexer::Numeric::F32 if self.in_range(val, "-3.4e38", "3.4e38", NumericKind::Float)                => Type::Numeric(crate::lexer::Numeric::F32),
                    crate::lexer::Numeric::F64 if self.in_range(val, "-1.8e308", "1.8e308", NumericKind::Float)              => Type::Numeric(crate::lexer::Numeric::F64),
                    _ => self.map_numeric(val, &Type::None)
                }
            },
            _ => {}
        }

        ty
    }

    fn in_range(&self, val: &str, l: &str, u: &str, kind: NumericKind) -> bool
    {
        match kind {
            NumericKind::Unsigned => {
                if val.contains('.') { return false; }
                let parsed: u64 = val.parse().unwrap_or(0);
                let plow:   u64 = l.parse().unwrap_or(0);
                let pup:    u64 = u.parse().unwrap_or(0);
                parsed >= plow && parsed <= pup
            },
            NumericKind::Signed => {
                if val.contains('.') { return false; }
                let parsed: i64 = val.parse().unwrap_or(0);
                let plow:   i64 = l.parse().unwrap_or(0);
                let pup:    i64 = u.parse().unwrap_or(0);
                parsed >= plow && parsed <= pup
            },
            NumericKind::Float => {
                let parsed: f64 = val.parse().unwrap_or(0.0);
                let plow:   f64 = l.parse().unwrap_or(0.0);
                let pup:    f64 = u.parse().unwrap_or(0.0);
                parsed >= plow && parsed <= pup
            },
        }
    }

    fn is_compatible(&self, found: &Type, expected: &Type) -> bool {
        if found == expected { return true; }

        match (found, expected) {
            (Type::Numeric(f), Type::Numeric(e)) => {
                use crate::lexer::Numeric::*;
                matches!((f, e),
                    (USize | U8 | U16 | U32 | U64, USize | U8 | U16 | U32 | U64) |
                    (I8 | I16 | I32 | I64, I8 | I16 | I32 | I64) |
                    (F16 | F32 | F64,      F16 | F32 | F64)
                )
            },
            (Type::Character, Type::Numeric(crate::lexer::Numeric::U8)) => true,
            (Type::Numeric(crate::lexer::Numeric::U8), Type::Character) => true,
            (Type::Character, Type::StringLiteral) | (Type::StringLiteral, Type::Character) => true,
            _ => false,
        }
    }


    fn check_range(&mut self, range: &mut Range, expected: &Type, symbol: &mut SymbolTable) -> Type
    {
        let ty: Type;

        let start = self.check_expr(&mut range.start, expected, symbol);
        let step = if let Some(st) = &mut range.step
        {
            Some(self.check_expr(st, expected, symbol))
        }
        else {
            None
        };
        let end = self.check_expr(&mut range.end, expected, symbol);

        ty = if let Some(st) = step
        {
            if start == st && st == end
            {
                start
            }
            else {
                Type::None
            }
        }
        else {
            if start == end
            {
                start
            }
            else {
                Type::None
            }
        };

        ty
    }

    fn check_if(&mut self, ifelse: &mut IfElse, symbol: &mut SymbolTable) -> Type
    {
        let ty: Type;

        symbol.push(symbol::ScopeKind::Block);
        ty = self.check_body(ifelse.body.as_mut(), symbol);
        symbol.pop();

        if let Some(elif) = ifelse.elif.as_mut()
        {
            let elif_ty = self.check_if(elif, symbol);

            if ty != elif_ty
            {
                self.rep.add(GraviError::throw(crate::error::Kind::TypeMismatch(ty.clone(), elif_ty)));
            }
            else if elif.ret.is_none()
            {
                elif.ret = Some(ty.clone());
            }
        }

        if ifelse.ret.is_none()
        {
            ifelse.ret = Some(ty.clone());
        }

        ty
    }

    fn check_call(&mut self, vals: &mut Vec<Value>, params: &[Type], symbol: &mut SymbolTable) -> Type {
        for (i, val) in vals.iter_mut().enumerate() {
            let param_ty = params.get(i).unwrap_or(&Type::None);
            let ty = self.check_val(val, param_ty, symbol);
            
            if !self.is_compatible(&ty, param_ty) && param_ty != &Type::None {
                self.rep.add(GraviError::throw(crate::error::Kind::TypeMismatch(param_ty.clone(), ty)));
            }
        }

        Type::None
    }


    pub fn reporter(&mut self) -> &Reporter
    {
        &self.rep
    }
}
