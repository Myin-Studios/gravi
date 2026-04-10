pub use crate::ast::*;
use crate::{error::{NyonError, Reporter}, lexer::Type};

#[derive(Clone, Debug)]
pub struct TypeInfo
{
    ty: Type,
    mutable: bool
}

#[derive(Clone, Debug)]
pub struct SymbolInfo
{
    id: String,
    ty: TypeInfo,
}

impl SymbolInfo {
    pub fn new(k: String, v: TypeInfo) -> Self
    {
        Self
        {
            id: k,
            ty: v
        }
    }
}

pub struct Checker
{
    stack: Vec<Vec<SymbolInfo>>,
    rep: Reporter,
}

impl Checker {
    pub fn new() -> Self
    {
        Self
        {
            stack: Vec::new(),
            rep: Reporter::new(),
        }
    }

    pub fn process(&mut self, prog: &mut Program)
    {
        for item in prog.items.iter_mut()
        {
            match item {
                Global::Fun(fun) => {
                    let mut map = Vec::new();
                    map.push(
                        SymbolInfo::new(fun.identifier().to_string(),
                        TypeInfo
                        {
                            ty: fun.ret().to_owned(),
                            mutable: false,
                        })
                    );

                    self.stack.push(map);

                    self.check_fun(fun);
                },
            }
        }
    }

    fn has(&self, what: &String) -> Option<&SymbolInfo>
    {
        let mut sym = None;
        for symbols in &self.stack
        {
            sym = symbols.iter().find(|s| s.id == *what)
            .map(|s| s);

            if sym.is_some()
            {
                break;
            }
        }

        sym
    }

    fn set_type(&mut self, name: &String, ty: Type)
    {
        for symbols in &mut self.stack
        {
            if let Some(s) = symbols.iter_mut().find(|s| s.id == *name)
            {
                s.ty.ty = ty;
                return;
            }
        }
    }

    fn check_fun(&mut self, fun: &mut Function)
    {
        for param in fun.params()
        {
            let mut map = Vec::new();
            map.push(SymbolInfo::new(
                param.identifier().to_string(),
                TypeInfo
                {
                    ty: param.ty().to_owned(),
                    mutable: param.mutable()
                }
            ));

            self.stack.push(map);
        }

        self.stack.push(Vec::new());
        self.check_body(&mut fun.body);
        self.stack.pop();
    }
    
    fn check_body(&mut self, items: &mut Vec<Items>) -> Type
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
                                    ty = self.check_val(val, &ty);
                                }
                                
                                v.ty = ty.clone();
                            }
                            else
                            {
                                if let Some(val) = v.val.as_mut()
                                {
                                    let t = self.check_val(&mut val.clone(), v.ty());
                                    if t != v.ty().to_owned()
                                    {
                                        self.rep.add(NyonError::throw(crate::error::Kind::TypeMismatch(v.ty.to_owned(), t)));
                                    }
                                }
                            }
                            
                            if let Some(last) = self.stack.last_mut()
                            {
                                last.push(SymbolInfo::new(
                                    v.identifier().to_string(),
                                    TypeInfo
                                    {
                                        ty: v.ty().to_owned(),
                                        mutable: v.mutable()
                                    })
                                );
                            }
                        },
                        Var::Var(v) => {
                            let name = v.name.clone();
                            let current_ty = self.has(&name).map(|s| s.ty.ty.clone());

                            if let Some(Type::None) = current_ty {
                                if let Some(val) = v.val.as_mut() {
                                    let inferred = self.check_val(val, &Type::None);
                                    self.set_type(&name, inferred.clone());

                                    for prev in back.iter_mut() {
                                        if let Items::Var(Var::Decl(decl)) = prev {
                                            if decl.id == name && decl.ty == Type::None {
                                                decl.ty = inferred.clone();
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }
                },
                Items::Expr(Value::Block(_, b)) => {
                    self.stack.push(Vec::new());
                    self.check_body(b);
                    self.stack.pop();
                },
                Items::Expr(Value::IfElse(ifelse)) => {
                    ty = self.check_if(ifelse);
                },
                Items::Ret(val) => {
                    ty = self.check_val(val, &ty);
                }
                _ => {}
            }
        }

        ty
    }

    fn check_val(&mut self, val: &mut Value, expected: &Type) -> Type
    {
        let mut ty = Type::None;

        match val {
            Value::Expression(expr) => {
                match expr {
                    Expr::Identifier(id) => {
                        for outer in self.stack.clone()
                        {
                            for inner in outer
                            {
                                if inner.id == id.to_string()
                                {
                                    ty = inner.ty.ty;
                                }
                            }
                        }
                    },
                    Expr::Literal(val) => {
                        if ty == Type::None
                        {
                            ty = self.map_numeric(val, expected);
                        }
                        else if ty != self.map_numeric(val, &ty) {
                            // error!
                        }
                    },
                    _ => {}
                }
            },
            Value::StringLiteral(_) => {
                ty = Type::StringLiteral;
            },
            Value::Boolean(_) => ty = Type::Boolean,
            Value::Call(_, values) => {},
            Value::Null => ty = Type::None,
            Value::Block(bty, b) => {
                self.stack.push(Vec::new());
                let inf = self.check_body(b);
                *bty = inf.clone();
                ty = inf;
                self.stack.pop();
            },
            Value::IfElse(ifelse) => {
                ty = self.check_if(ifelse);
            },
        }
        
        ty
    }

    fn map_numeric(&self, val: &String, expected: &Type) -> Type
    {
        let mut ty = Type::None;

        if expected == &Type::None
        {
            ty = if self.in_range(val, "0", "255", 0) { Type::Numeric(crate::lexer::Numeric::U8) }
                 else if self.in_range(val, "0", "64535", 0) { Type::Numeric(crate::lexer::Numeric::U16) }
                 else if self.in_range(val, "0", "4294967295", 0) { Type::Numeric(crate::lexer::Numeric::U32) }
                 else if self.in_range(val, "0", "18446744073709551615", 0) { Type::Numeric(crate::lexer::Numeric::U64) }
                 else if self.in_range(val, "-128", "127", 1) { Type::Numeric(crate::lexer::Numeric::I8) }
                 else if self.in_range(val, "-32768", "32767",1) { Type::Numeric(crate::lexer::Numeric::I16) }
                 else if self.in_range(val, "-2147483648", "2147483647",1) { Type::Numeric(crate::lexer::Numeric::I32) }
                 else if self.in_range(val, "-9223372036854775808", "9223372036854775807",1) { Type::Numeric(crate::lexer::Numeric::I64) }
                 else if self.in_range(val, "-65.504", "-65.504", 2) { Type::Numeric(crate::lexer::Numeric::I16) }
                 else if self.in_range(val, "-3.4e38", "3.4e38", 2) { Type::Numeric(crate::lexer::Numeric::F32) }
                 else if self.in_range(val, "-1.8e308", "1.8e308", 2) { Type::Numeric(crate::lexer::Numeric::F64) }
                 else { Type::None }
        }

        match expected {
            Type::Numeric(num) => {
                ty = match num {
                    crate::lexer::Numeric::U8 if self.in_range(val, "0", "255", 0) => Type::Numeric(crate::lexer::Numeric::U8),
                    crate::lexer::Numeric::U16 if self.in_range(val, "0", "64535", 0) => Type::Numeric(crate::lexer::Numeric::U16),
                    crate::lexer::Numeric::U32 if self.in_range(val, "0", "4294967295", 0) => Type::Numeric(crate::lexer::Numeric::U32),
                    crate::lexer::Numeric::U64 if self.in_range(val, "0", "18446744073709551615", 0) => Type::Numeric(crate::lexer::Numeric::U64),
                    crate::lexer::Numeric::I8 if self.in_range(val, "-128", "127", 1) => Type::Numeric(crate::lexer::Numeric::I8),
                    crate::lexer::Numeric::I16 if self.in_range(val, "-32768", "32767",1) => Type::Numeric(crate::lexer::Numeric::I16),
                    crate::lexer::Numeric::I32 if self.in_range(val, "-2147483648", "2147483647",1) => Type::Numeric(crate::lexer::Numeric::I32),
                    crate::lexer::Numeric::I64 if self.in_range(val, "-9223372036854775808", "9223372036854775807",1) => Type::Numeric(crate::lexer::Numeric::I64),
                    crate::lexer::Numeric::F16 if self.in_range(val, "-65.504", "-65.504", 2) => Type::Numeric(crate::lexer::Numeric::I16),
                    crate::lexer::Numeric::F32 if self.in_range(val, "-3.4e38", "3.4e38", 2) => Type::Numeric(crate::lexer::Numeric::F32),
                    crate::lexer::Numeric::F64 if self.in_range(val, "-1.8e308", "1.8e308", 2) => Type::Numeric(crate::lexer::Numeric::F64),
                    _ => self.map_numeric(val, &Type::None)
                }
            },
            _ => {}
        }

        ty
    }

    fn in_range(&self, val: &String, l: &str, u: &str, mut what: usize) -> bool
    {
        if what > 2 { what = 2 }

        if what == 0
        {
            if val.contains(".")
            {
                return false;
            }

            let parsed: u64 = val.parse().unwrap_or(0);
            let plow: u64 = l.parse().unwrap_or(0);
            let pup: u64 = u.parse().unwrap_or(0);

            return parsed >= plow && parsed <= pup;
        }
        else if what == 1 {
            if val.contains(".")
            {
                return false;
            }
            
            let parsed: i64 = val.parse().unwrap_or(0);
            let plow: i64 = l.parse().unwrap_or(0);
            let pup: i64 = u.parse().unwrap_or(0);

            return parsed >= plow && parsed <= pup;
        }
        else {
            let parsed: f64 = val.parse().unwrap_or(0.0);
            let plow: f64 = l.parse().unwrap_or(0.0);
            let pup: f64 = u.parse().unwrap_or(0.0);

            return parsed >= plow && parsed <= pup;
        }
    }
    
    fn check_if(&mut self, ifelse: &mut IfElse) -> Type
    {
        let ty: Type;

        self.stack.push(Vec::new());
        ty = self.check_body(ifelse.body.as_mut());
        self.stack.pop();

        if let Some(elif) = ifelse.elif.as_mut()
        {
            let elif_ty = self.check_if(elif);

            if ty != elif_ty
            {
                self.rep.add(NyonError::throw(crate::error::Kind::TypeMismatch(ty.clone(), elif_ty)));
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

    pub fn reporter(&mut self) -> &Reporter
    {
        &self.rep
    }
}