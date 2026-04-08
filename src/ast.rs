use crate::lexer::{Operator, Type};

#[derive(Clone, Debug)]
pub struct Program
{
    pub items: Vec<Global>,
}

#[derive(Clone, Debug)]
pub enum Global
{
    // Import,
    Fun(Function),
    // Class,
    // Inter,
}

#[derive(Clone, Debug)]
pub enum Items
{
    Var(VarDecl),
    Ret(Value),
    Call(String, Vec<Value>),
    Lambda(Function),
    Block(Vec<Items>),
    None,
}

#[derive(PartialEq, Clone, Debug)]
pub enum Parallelism
{
    CPU,
    GPU,
    None,
}

#[derive(Clone, Debug)]
pub struct VarDecl
{
    pub par: Parallelism,
    pub mutable: bool,
    pub id: String,
    pub ty: Type,
    pub val: Option<Value>
}

impl VarDecl {
    pub fn new() -> Self
    {
        Self
        {
            par: Parallelism::None,
            mutable: false,
            id: "".to_string(),
            ty: Type::None,
            val: None
        }
    }

    pub fn parallelism(&self) -> &Parallelism
    {
        &self.par
    }

    pub fn mutable(&self) -> bool
    {
        self.mutable
    }

    pub fn identifier(&self) -> &str
    {
        &self.id
    }

    pub fn ty(&self) -> &Type
    {
        &self.ty
    }

    pub fn value(&self) -> &Option<Value>
    {
        &self.val
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum BoolValue
{
    True,
    False
}

#[derive(Debug, Clone)]
pub enum Value
{
    Expression(Expr),
    StringLiteral(String),
    Boolean(BoolValue),
    Call(String, Vec<Value>),
    Block(Vec<Items>),
    Null
}

#[derive(Debug, Clone)]
pub struct Binary
{
    pub left: Box<Expr>,
    pub op: Operator,
    pub right: Box<Expr>,
}

impl Binary {
    pub fn new() -> Self
    {
        Self
        {
            left: Box::new(Expr::Null),
            op: Operator::None,
            right: Box::new(Expr::Null),
        }
    }
    
    pub fn left(&self) -> &Expr
    {
        &self.left
    }

    pub fn op(&self) -> &Operator
    {
        &self.op
    }

    pub fn right(&self) -> &Expr
    {
        &self.right
    }
}

#[derive(Debug, Clone)]
pub struct Boolean
{
    pub left: Box<Expr>,
    pub op: Operator,
    pub right: Box<Expr>,
}

impl Boolean {
    pub fn new() -> Self
    {
        Self
        {
            left: Box::new(Expr::Null),
            op: Operator::None,
            right: Box::new(Expr::Null),
        }
    }
    
    pub fn left(&self) -> &Expr
    {
        &self.left
    }

    pub fn op(&self) -> &Operator
    {
        &self.op
    }

    pub fn right(&self) -> &Expr
    {
        &self.right
    }
}

#[derive(Debug, Clone)]
pub struct Unary
{
    pub op: Operator,
    pub right: Box<Expr>,
}

impl Unary {
    pub fn new() -> Self
    {
        Self
        {
            op: Operator::None,
            right: Box::new(Expr::Null),
        }
    }
    
    pub fn op(&self) -> &Operator
    {
        &self.op
    }

    pub fn right(&self) -> &Expr
    {
        &self.right
    }
}

#[derive(Debug, Clone)]
pub enum Expr
{
    Literal(String),
    Identifier(String),
    Range(Range),
    Binary(Binary),
    Boolean(Boolean),
    Unary(Unary),
    Grouped(Box<Expr>),
    Call(Vec<Value>),
    Null
}

#[derive(Debug, Clone)]
pub struct Range
{
    pub start: Box<Expr>,
    pub step: Option<Box<Expr>>,
    pub end: Box<Expr>,
    pub inclusive: bool,
}

impl Range {
    pub fn new() -> Self
    {
        Self
        {
            start: Box::new(Expr::Literal("0".to_string())),
            step: Some(Box::new(Expr::Literal("1".to_string()))),
            end: Box::new(Expr::Literal("1".to_string())),
            inclusive: true,
        }
    }

    pub fn start(&self) -> &Box<Expr>
    {
        &self.start
    }

    pub fn step(&self) -> &Option<Box<Expr>>
    {
        &self.step
    }

    pub fn end(&self) -> &Box<Expr>
    {
        &self.end
    }

    pub fn inclusive(&self) -> bool
    {
        self.inclusive
    }
}

#[derive(Clone, Debug)]
pub struct Function
{
    pub lambda: bool,
    pub main: bool,
    pub id: String,
    pub params: Vec<VarDecl>,
    pub ret: Type,
    pub body: Vec<Items>
}

impl Function {
    pub fn new() -> Self
    {
        Self
        {
            lambda: false,
            main: false,
            id: "".to_string(),
            params: Vec::new(),
            ret: Type::None,
            body: Vec::new()
        }
    }

    pub fn lambda(&self) -> bool
    {
        self.lambda
    }

    pub fn main(&self) -> bool
    {
        self.main
    }
    
    pub fn identifier(&self) -> &str
    {
        &self.id
    }

    pub fn params(&self) -> &Vec<VarDecl>
    {
        &self.params
    }

    pub fn ret(&self) -> &Type
    {
        &self.ret
    }

    pub fn body(&self) -> &Vec<Items>
    {
        &self.body
    }
}