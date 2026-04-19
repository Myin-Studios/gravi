use std::collections::HashMap;

use crate::{ast::{Items, Parallelism, Value}, lexer::Type};

#[derive(Clone, Debug)]
pub enum Symbol {
    Function(FunctionSym),
    Variable(VariableSym),
}

#[derive(Clone, Debug)]
pub struct FunctionSym {
    pub params: Vec<(String, Type, bool, Parallelism, bool)>,
    pub ret:    Type,
    pub public: bool,
    pub body:   Option<Vec<Items>>,
    pub ext:    bool,
}

#[derive(Clone, Debug)]
pub struct VariableSym {
    pub ty:      Type,
    pub mutable: bool,
    pub par:     Parallelism,
    pub list:    bool,
    pub value:   Option<Value>,
}

#[derive(Clone, Debug)]
pub struct SymbolTable {
    pub scopes: Vec<Scope>,
}

impl SymbolTable {
    pub fn new() -> Self
    {
        Self
        {
            scopes: Vec::new(),
        }
    }
    
    pub fn push(&mut self, kind: ScopeKind)
    {
        self.scopes.push(Scope { kind, symbols: HashMap::new() });
    }

    pub fn pop(&mut self)
    {
        self.scopes.pop();
    }
    
    pub fn add(&mut self, name: &str, symbol: Symbol)
    {
        if let Some(scope) = self.scopes.last_mut() {
            scope.symbols.insert(name.to_string(), symbol);
        }
    }

    pub fn find(&self, name: &str) -> Option<&Symbol>
    {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.symbols.get(name) {
                return Some(sym);
            }
        }
        None
    }

    pub fn nearest_fun(&self) -> Option<&Type>
    {
        for scope in self.scopes.iter().rev() {
            if let ScopeKind::Function(_, ref ret) = scope.kind {
                return Some(ret);
            }
        }
        None
    }
    
    pub fn in_loop(&self) -> bool
    {
        self.scopes.iter().rev().any(|s| matches!(s.kind, ScopeKind::Loop))
    }

    pub fn take(&mut self) -> Vec<(String, Type, Vec<(String, Type, bool, Parallelism, bool)>, Vec<Items>)> {
        let mut result = Vec::new();
        if let Some(scope) = self.scopes.first_mut() {
            for (name, sym) in scope.symbols.iter_mut() {
                if let Symbol::Function(fun) = sym {
                    if let Some(body) = fun.body.take() {
                        result.push((name.clone(), fun.ret.clone(), fun.params.clone(), body));
                    }
                }
            }
        }
        result
    }

    pub fn restore(&mut self, name: &str, body: Vec<Items>) {
        if let Some(scope) = self.scopes.first_mut() {
            if let Some(Symbol::Function(fun)) = scope.symbols.get_mut(name) {
                fun.body = Some(body);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Scope {
    pub kind:    ScopeKind,
    pub symbols: HashMap<String, Symbol>,
}

#[derive(Clone, Debug)]
pub enum ScopeKind {
    Global,
    Function(String, Type),
    Loop,
    Block,
}