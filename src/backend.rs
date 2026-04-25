pub mod c;
pub mod llvm;

pub trait Backend {
    fn output(&self) -> &String;
}