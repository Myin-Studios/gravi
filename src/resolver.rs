use std::path::Path;

use crate::{ast::{FunKind, Global, Program, Space}, error::{NyonError, Reporter}, lex, parse, symbol::{self, FunctionSym, SymbolTable}};

pub struct Resolver
{
    symbols: SymbolTable,
    rep: Reporter,
}

impl Resolver {
    pub fn new() -> Self
    {
        Self
        {
            symbols: SymbolTable::new(),
            rep: Reporter::new(),
        }
    }

    pub fn process(&mut self, program: &Program)
    {
        for item in program.items()
        {
            match item {
                Global::Import(spaces) => {
                    for i in 0..spaces.len()
                    {
                        self.symbols.push(symbol::ScopeKind::Global);
                        self.resolve_space(&spaces[i], "examples");
                    }
                },
                Global::Fun(FunKind::Custom(f)) => {
                    self.symbols.add(f.identifier(), symbol::Symbol::Function(FunctionSym {
                                                                                    params: f.params.iter().map(|p| (p.id.clone(), p.ty.clone(), p.mutable(), p.par.clone())).collect(),
                                                                                    ret:    f.ret.clone(),
                                                                                    public: f.public,
                                                                                    body:   None,
                                                                                }));
                },
                _ => {}
            }
        }
    }
    
    fn resolve_space(&mut self, space: &Space, path: &str)
    {
        let mut name = String::from(path);
        name.push_str(&format!("/{}", space.name));
        let mut file = name.clone();
        file.push_str(".nn");

        let is_dir: bool = Path::new(name.as_str()).is_dir();
        
        if is_dir
        {
            if let Some(sub) = space.sub.clone()
            {
                match sub
                {
                    crate::ast::Subspace::All => {},
                    crate::ast::Subspace::Some(spaces) => {
                        for s in spaces
                        {
                            self.resolve_space(&s, &name);
                        }
                    },
                }
            }
            else {
                // error! importing directory only is not supported!
            }
        }
        else if Path::new(&file).exists() {
            if let Some(sub) = space.sub.clone()
            {
                let mut l = lex(&file);
                l.reporter().fire_all();
                if l.reporter().has_errors() { std::process::exit(1); }

                let p = parse(l.tokens_mut());                
                p.reporter().fire_all();
                if p.reporter().has_errors() { std::process::exit(1); }

                for item in p.output().items()
                {
                    match item {
                        Global::Import(spaces) => {
                            for space in spaces
                            {
                                self.resolve_space(space, &name);
                            }
                        },
                        Global::Fun(FunKind::Custom(f)) | Global::Fun(FunKind::Entry(f)) => {
                            match sub.clone() {
                                crate::ast::Subspace::All => {
                                    if !f.public { self.rep.add(NyonError::throw(crate::error::Kind::PrivateImport(f.id.clone()))); }
                                    self.symbols.add(f.identifier(), symbol::Symbol::Function(FunctionSym {
                                                                                    params: f.params.iter().map(|p| (p.id.clone(), p.ty.clone(), p.mutable(), p.par.clone())).collect(),
                                                                                    ret:    f.ret.clone(),
                                                                                    public: f.public,
                                                                                    body:   Some(f.body.clone()),
                                                                                }));
                                },
                                crate::ast::Subspace::Some(spaces) => {
                                    if spaces.iter().any(|s| s.name == f.id) {
                                        if !f.public { self.rep.add(NyonError::throw(crate::error::Kind::PrivateImport(f.id.clone()))); }
                                        
                                        self.symbols.add(f.identifier(), symbol::Symbol::Function(FunctionSym {
                                                                                    params: f.params.iter().map(|p| (p.id.clone(), p.ty.clone(), p.mutable(), p.par.clone())).collect(),
                                                                                    ret:    f.ret.clone(),
                                                                                    public: f.public,
                                                                                    body:   Some(f.body.clone()),
                                                                                }));
                                    }
                                },
                            }
                        },
                    }
                }
            }
            else {
            }
        }
        else {
        }
    }
    
    pub fn reporter(&self) -> &Reporter
    {
        &self.rep
    }
    
    pub fn output(&mut self) -> &mut SymbolTable
    {
        &mut self.symbols
    }
}