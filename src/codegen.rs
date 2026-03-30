use crate::backend::c::CGenerator;

pub enum BackendType
{
    GCC,
    ZIG,
    LLVM,
}

#[derive(PartialEq, Debug)]
pub enum Target
{
    Debug,
    Release
}

#[derive(PartialEq, Debug)]
pub enum BuildFlag
{
    KeepCode,
    RemoveCode
}

pub enum Generators
{
    C(CGenerator)
}