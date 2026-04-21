use crate::lexer::{Operator, Type};

#[derive(Clone, Debug)]
pub struct Program
{
    pub items: Vec<Global>,
}

impl Program {
    pub fn new() -> Self
    {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, item: Global)
    {
        self.items.push(item);
    }

    pub fn items(&self) -> &Vec<Global>
    {
        &self.items
    }
}

#[derive(Clone, Debug)]
pub enum Global
{
    Import(Vec<Space>),
    Fun(FunKind),
    Var(VarDecl),
    Class(Class),
    // Inter,
}

#[derive(Clone, Debug)]
pub struct Space
{
    pub name: String,
    pub alias: Option<String>,
    pub sub: Option<Subspace>,
    pub used: bool,
}

#[derive(Clone, Debug)]
pub enum Subspace
{
    All,
    Some(Vec<Space>),
}

#[derive(Clone, Debug)]
pub struct Class
{
    pub public: bool,
    pub name:   String,
    pub imp:    Option<Vec<String>>,
    pub body:   Vec<ClassBody>,
}

#[derive(Clone, Debug)]
pub enum ClassBody
{
    Var(VarDecl),
    Fun(FunKind),
}

#[derive(Clone, Debug)]
pub enum FunKind
{
    Entry(Function),
    Custom(Function),
    Extern(Function),
}

#[derive(Clone, Debug)]
pub enum Items
{
    Var(Var),
    Ret(Value),
    Expr(Value),
    Stop,
    Skip,
}

#[derive(Clone, Debug)]
pub enum Var
{
    Decl(VarDecl),
    Var(Variable),
}

#[derive(Clone, Debug)]
pub struct Variable
{
    pub name: String,
    pub val:  Option<Value>
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
    pub par:     Parallelism,
    pub mutable: bool,
    pub id:      String,
    pub ty:      Type,
    pub val:     Option<Value>,
    pub list:    bool,
}

impl VarDecl {
    pub fn new() -> Self
    {
        Self
        {
            par:     Parallelism::None,
            mutable: false,
            id:      "".to_string(),
            ty:      Type::None,
            val:     None,
            list:    false
        }
    }

    pub fn parallelism(&self) -> &Parallelism   { &self.par }
    pub fn mutable(&self)     -> bool           { self.mutable }
    pub fn identifier(&self)  -> &str           { &self.id }
    pub fn ty(&self)          -> &Type          { &self.ty }
    pub fn value(&self)       -> &Option<Value> { &self.val }
    pub fn list(&self)        -> bool           { self.list }
}   

#[derive(PartialEq, Debug, Clone)]
pub enum BoolValue
{
    True,
    False
}

#[derive(Debug, Clone)]
pub struct IfElse
{
    pub cond: Option<Box<Value>>,
    pub body: Vec<Items>,
    pub elif: Option<Box<IfElse>>,
    pub ret:  Option<Type>
}

impl IfElse {
    pub fn new() -> Self
    {
        Self
        {
            cond: None,
            body: Vec::new(),
            elif: None,
            ret:  None
        }
    }

    pub fn condition(&self) -> &Option<Box<Value>>  { &self.cond }
    pub fn body(&self)      -> &[Items]        { &self.body }
    pub fn else_if(&self)   -> Option<&IfElse> { self.elif.as_deref() }
    pub fn ret(&self)       -> &Option<Type>   { &self.ret }
}

#[derive(Debug, Clone)]
pub struct Loop
{
    pub cond: Option<Box<VarDecl>>,
    pub body: Vec<Items>
}

#[derive(Debug, Clone)]
pub enum Value
{
    Expression(Expr),
    StringLiteral(String),
    Char(char),
    Boolean(BoolValue),
    Call(String, Vec<Value>),
    List(List), // vector: a[index] --- matrix (future implementation): a[row; column] --- array as result: a[i1, i1, ... in]
    Block(Type, Vec<Items>),
    IfElse(IfElse),
    Loop(Loop),
    Null
}

#[derive(Debug, Clone)]
pub enum List
{
    Decl(Expr, Option<Vec<Vec<Value>>>),
    Use(String, Vec<Vec<Value>>, Option<Box<Value>>),
}

/// Single struct used for both arithmetic and logical binary expressions.
/// `Expr::Binary` holds arithmetic ops (+, -, *, /);
/// `Expr::Boolean` holds logical ops (&&, ||, |, &).
#[derive(Debug, Clone)]
pub struct BinaryOp
{
    pub left:  Box<Expr>,
    pub op:    Operator,
    pub right: Box<Expr>,
}

impl BinaryOp {
    pub fn new() -> Self
    {
        Self
        {
            left:  Box::new(Expr::Null),
            op:    Operator::None,
            right: Box::new(Expr::Null),
        }
    }

    pub fn left(&self)  -> &Expr     { &self.left }
    pub fn op(&self)    -> &Operator { &self.op }
    pub fn right(&self) -> &Expr     { &self.right }
}

#[derive(Debug, Clone)]
pub struct Unary
{
    pub op:    Operator,
    pub right: Box<Expr>,
}

impl Unary {
    pub fn new() -> Self
    {
        Self
        {
            op:    Operator::None,
            right: Box::new(Expr::Null),
        }
    }

    pub fn op(&self)    -> &Operator { &self.op }
    pub fn right(&self) -> &Expr     { &self.right }
}

#[derive(Debug, Clone)]
pub enum Expr
{
    Literal(String),
    CharLiteral(char),
    StringLiteral(String),
    Identifier(String),
    Call(String, Vec<Value>),
    Index(String, Box<Expr>),
    Range(Range),
    Binary(BinaryOp),
    Boolean(BinaryOp),
    Unary(Unary),
    Grouped(Box<Expr>),
    Cast(Cast),
    Null
}

#[derive(Debug, Clone)]
pub struct Range
{
    pub start:     Box<Expr>,
    pub step:      Option<Box<Expr>>,
    pub end:       Box<Expr>,
    pub inclusive: bool,
}

impl Range {
    pub fn new() -> Self
    {
        Self
        {
            start:     Box::new(Expr::Literal("0".to_string())),
            step:      Some(Box::new(Expr::Literal("1".to_string()))),
            end:       Box::new(Expr::Literal("1".to_string())),
            inclusive: true,
        }
    }

    pub fn start(&self)     -> &Expr         { &self.start }
    pub fn step(&self)      -> Option<&Expr> { self.step.as_deref() }
    pub fn end(&self)       -> &Expr         { &self.end }
    pub fn inclusive(&self) -> bool          { self.inclusive }
}

#[derive(Debug, Clone)]
pub struct Cast
{
    pub what: Box<Expr>,
    pub to:   Type,
}

#[derive(Clone, Debug)]
pub struct Function
{
    pub public: bool,
    pub lambda: bool,
    pub id:     String,
    pub params: Vec<VarDecl>,
    pub ret:    Type,
    pub body:   Vec<Items>
}

impl Function {
    pub fn new() -> Self
    {
        Self
        {
            public: false,
            lambda: false,
            id:     "".to_string(),
            params: Vec::new(),
            ret:    Type::None,
            body:   Vec::new()
        }
    }

    pub fn lambda(&self)     -> bool     { self.lambda }
    pub fn identifier(&self) -> &str     { &self.id }
    pub fn params(&self)     -> &[VarDecl] { &self.params }
    pub fn ret(&self)        -> &Type    { &self.ret }
    pub fn body(&self)       -> &[Items] { &self.body }
}
