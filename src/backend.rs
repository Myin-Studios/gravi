use crate::ast::Program;

pub mod c;

pub trait Backend {
    fn process(&mut self, prog: &Program);
    fn output(&self) -> &String;
}