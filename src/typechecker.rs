use std::collections::HashMap;

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
    id: IDInfo,
    ty: TypeInfo,
}

#[derive(Clone, Debug)]
pub struct Symbol
{
    info: SymbolInfo,
    scope: Option<Vec<Symbol>>
}

pub struct Checker
{
    symbols: Vec<Symbol>
}

impl Checker {
    pub fn new() -> Self
    {
        Self
        {
            symbols: Vec::new(),
        }
    }

    pub fn process(&mut self, prog: &Program)
    {
        for item in prog.items()
        {
            match item {
                Global::Fun(fun) => {
                    self.check_fun(fun, 0);
                },
            }
        }

        println!("\n{:#?}", self.symbols);
    }

    fn check_fun(&mut self, fun: &Function, lvl: usize)
    {
        let id_info = IDInfo
        {
            id: fun.identifier().to_string(),
            scope: lvl,
        };
        let type_info = TypeInfo
        {
            mutable: false,
            ty: fun.ret().clone(),
        };

        let body = self.check_body(fun.body());
        
        self.symbols.push(
            Symbol
            {
                info: SymbolInfo
                {
                    id: id_info,
                    ty: type_info
                },
                scope: Some(body)
            }
        );
    }
    
    fn check_body(&mut self, items: &Vec<Items>) -> Vec<Symbol>
    {
        let mut body = Vec::new();
        
        for item in items
        {
            match item {
                Items::Var(var) => body.push(self.check_var(var, 1)),
                Items::Block(b) => body.extend(self.check_body(b)),
                _ => {}
            }
        }

        body
    }

    fn check_var(&mut self, var: &VarDecl, lvl: usize) -> Symbol
    {
        let id_info = IDInfo
        {
            id: var.identifier().to_string(),
            scope: lvl
        };
        let ty = var.ty();

        if ty == &Type::None
        {
            for sym in self.symbols.iter().rev()
            {
                // check for some corresponding symbol in the current/previous symbol-tree level(s)
            }
        }

        let type_info = TypeInfo
        {
            mutable: var.mutable(),
            ty: var.ty().clone()
        };

        Symbol
        {
            info: SymbolInfo
            {
                id: id_info,
                ty: type_info
            },
            scope: None
        }
    }
}