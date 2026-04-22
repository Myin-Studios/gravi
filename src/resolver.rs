use std::path::Path;

use crate::{ast::{FunKind, Function, Global, Program, Space, Subspace}, error::{GraviError, Reporter}, lex, parse, symbol::{self, FunctionSym, SymbolTable}};

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

    pub fn process(&mut self, program: &Program, filename: &str, dirname: &str)
    {
        self.symbols.push(symbol::ScopeKind::Global);
        for item in program.items()
        {
            match item {
                Global::Import(spaces) => {
                    for i in 0..spaces.len()
                    {
                        self.resolve_space(&spaces[i], filename, dirname);
                    }
                },
                Global::Fun(FunKind::Custom(f)) => {
                    self.symbols.add(f.identifier(), Self::fun_sym(f, false, false));
                },
                _ => {}
            }
        }
    }
    
    fn parse_file(&mut self, path: &str) -> Option<Vec<Global>>
    {
        let mut l = lex(path);
        l.reporter().fire_all();
        if l.reporter().has_errors() { std::process::exit(1); }

        let p = parse(l.tokens_mut());
        p.reporter().fire_all();
        if p.reporter().has_errors() { std::process::exit(1); }

        Some(p.output().items().to_vec())
    }

    fn fun_sym(f: &Function, with_body: bool, ext: bool) -> symbol::Symbol
    {
        symbol::Symbol::Function(FunctionSym {
            params: f.params.iter()
                            .map(|p| (p.id.clone(), p.ty.clone(), p.mutable(), p.par.clone(), p.list.clone()))
                            .collect(),
            ret:    f.ret.clone(),
            public: f.public,
            body:   with_body.then(|| f.body.clone()),
            ext
        })
    }

    fn resolve_space(&mut self, space: &Space, filename: &str, dir: &str)
    {
        let module_dir  = format!("{}/{}", dir, space.name);
        let module_file = format!("{}.nn", module_dir);

        let is_dir      = Path::new(&module_dir).is_dir();
        let file_exists = Path::new(&module_file).exists();

        if is_dir && file_exists
        {
            self.resolve_file(space, &module_file, &module_dir, filename);
        }
        else if is_dir
        {
            match &space.sub {
                Some(Subspace::Some(subspaces)) => {
                    for s in subspaces {
                        self.resolve_space(s, filename, &module_dir);
                    }
                },
                _ => {
                    self.rep.add(GraviError::throw(crate::error::Kind::InvalidImport(space.name.clone()))
                                                .file(filename));
                }
            }
        }
        else if file_exists
        {
            self.resolve_file(space, &module_file, &module_dir, filename);
        }
        else
        {
            self.rep.add(GraviError::throw(crate::error::Kind::InvalidImport(space.name.clone()))
                                        .file(filename));
        }
    }

    fn resolve_file(&mut self, space: &Space, filename: &str, dirname: &str, origin: &str)
    {
        match &space.sub {
            Some(Subspace::Some(wanted)) => {
                let mut path_segs:  Vec<Space> = Vec::new();
                let mut file_leaves: Vec<Space> = Vec::new();
                let mut fn_names:   Vec<Space> = Vec::new();

                for s in wanted {
                    if s.sub.is_some() {
                        path_segs.push(s.clone());
                    } else if Path::new(&format!("{}/{}.nn", dirname, s.name)).exists() {
                        file_leaves.push(s.clone());
                    } else {
                        fn_names.push(s.clone());
                    }
                }

                for seg in &path_segs {
                    self.resolve_space(seg, filename, dirname);
                }

                for leaf in &file_leaves {
                    let leaf_path = format!("{}/{}.nn", dirname, leaf.name);
                    self.import_from_file(&leaf_path, dirname, &None, origin, false);
                }

                if !fn_names.is_empty() {
                    let fn_sub = Some(Subspace::Some(fn_names));
                    self.import_from_file(filename, dirname, &fn_sub, origin, true);
                }
            },
            sub => {
                self.import_from_file(filename, dirname, sub, origin, true);
            }
        }
    }

    fn import_from_file(&mut self, filename: &str, dirname: &str, sub: &Option<Subspace>, origin: &str, resolve_inner: bool)
    {
        let Some(items) = self.parse_file(filename) else { return; };

        for item in &items {
            match item {
                Global::Import(spaces) if resolve_inner => {
                    for s in spaces {
                        self.resolve_space(s, filename, dirname);
                    }
                },
                Global::Fun(FunKind::Custom(f)) | Global::Fun(FunKind::Entry(f)) => {
                    self.resolve_fun(f, sub, origin, false);
                },
                Global::Fun(FunKind::Extern(f)) => {
                    self.resolve_fun(f, sub, origin, true);
                },
                _ => {}
            }
        }
    }

    fn resolve_fun(&mut self, f: &Function, sub: &Option<Subspace>, origin: &str, ext: bool)
    {
        let should_import = match sub {
            None | Some(Subspace::All) => true,
            Some(Subspace::Some(wanted)) => wanted.iter().any(|s| s.name == f.id),
        };

        if !should_import { return; }

        if !f.public {
            self.rep.add(GraviError::throw(crate::error::Kind::PrivateImport(f.id.clone())));
        } else if self.symbols.find(f.identifier()).is_some() {
            self.rep.add(GraviError::throw(crate::error::Kind::DuplicateImport(f.id.clone()))
                                    .file(origin));
        } else {
            self.symbols.add(f.identifier(), Self::fun_sym(f, true, ext));
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