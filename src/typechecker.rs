pub use crate::ast::*;
use crate::lexer::Type;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IDInfo
{
    id: String,
    scope: usize
}

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
}

impl Checker {
    pub fn new() -> Self
    {
        Self
        {
            stack: Vec::new(),
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

        for item in items
        {
            match item {
                Items::Var(var) => {
                    if var.ty() == &Type::None
                    {
                        if let Some(val) = var.val.as_mut()
                        {
                            ty = self.check_val(val);
                        }

                        var.ty = ty.clone();
                    }
                    
                    if let Some(last) = self.stack.last_mut()
                    {
                        last.push(SymbolInfo::new(
                            var.identifier().to_string(),
                            TypeInfo
                            {
                                ty: ty.to_owned(),
                                mutable: var.mutable()
                            })
                        );
                    }
                    
                    if let Some(val) = var.val.as_mut()
                    {
                        self.check_val(val);
                    }
                },
                Items::Block(b) => {
                    self.stack.push(Vec::new());
                    self.check_body(b);
                    self.stack.pop();
                },
                Items::Ret(val) => {
                    ty = self.check_val(val);
                }
                _ => {}
            }
        }

        println!("\n{:#?}\n", self.stack);

        ty
    }

    fn check_val(&mut self, val: &mut Value) -> Type
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
                    _ => {}
                }
            },
            Value::StringLiteral(_) => {
                ty = Type::StringLiteral;
            },
            Value::Boolean(_) => ty = Type::Boolean,
            Value::Call(_, values) => todo!(),
            Value::Null => ty = Type::None,
            Value::Block(b) => {
                self.stack.push(Vec::new());
                ty = self.check_body(b);
                self.stack.pop();
            },
        }
        
        ty
    }
}