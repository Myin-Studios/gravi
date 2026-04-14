use crate::{ast::Program, symbol::SymbolTable};

pub mod c;

pub trait Backend {
    fn process(&mut self, prog: &Program, symbols: &SymbolTable);
    fn output(&self) -> &String;
}