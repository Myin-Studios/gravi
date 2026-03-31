use crate::lexer::*;

#[derive(Debug)]
pub struct Program
{
    items: Vec<Items>,
}

#[derive(Debug)]
pub enum Items
{
    Var(VarDecl)
}

#[derive(PartialEq, Clone, Debug)]
pub enum Parallelism
{
    CPU,
    GPU,
    None,
}

#[derive(Debug)]
pub struct VarDecl
{
    par: Parallelism,
    mutable: bool,
    id: String,
    ty: Type,
    val: Option<Value>
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

    pub fn mutable(&self) -> &bool
    {
        &self.mutable
    }

    pub fn identifier(&self) -> &String
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

#[derive(Debug, Clone)]
pub enum Value
{
    Expression(Expr),
    StringLiteral(String),
    Null
}

#[derive(Debug, Clone)]
pub struct Binary
{
    left: Box<Expr>,
    op: Operator,
    right: Box<Expr>,
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
pub enum Expr
{
    Literal(String),
    Identifier(String),
    Range(Range),
    Binary(Binary),
    Grouped(Box<Expr>),
    Null
}

#[derive(Debug, Clone)]
pub struct Range
{
    start: Box<Expr>,
    step: Option<Box<Expr>>,
    end: Box<Expr>,
    inclusive: bool,
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

#[derive(PartialEq, Debug)]
pub enum Expects
{
    Type,
    Assignment,
    Nothing,
}

pub struct Parser
{
    prog: Program,
    expects: Expects
}

impl Program {
    pub fn new() -> Self
    {
        Self
        {
            items: Vec::new(),
        }
    }

    pub fn add(&mut self, item: Items)
    {
        self.items.push(item);
    }

    pub fn items(&self) -> &Vec<Items>
    {
        &self.items
    }
}

impl Parser {
    pub fn new() -> Self
    {
        Self
        {
            prog: Program::new(),
            expects: Expects::Nothing
        }
    }

    pub fn process(&mut self, tokens: &mut Vec<Token>)
    {
        tokens.reverse();

        let mut mutable: bool = false;
        let mut par: Parallelism = Parallelism::None;

        loop {
            if tokens.is_empty()
            {
                break;
            }

            if let Some(t) = tokens.pop()
            {
                match t.kind() {
                    TokenKind::Keyword(kw) => 
                    {
                        match kw {
                            Keyword::GPU => {
                                par = Parallelism::GPU;
                            },
                            Keyword::PAR => {
                                par = Parallelism::CPU
                            },
                            Keyword::Mut => {
                                mutable = true;
                            },
                            Keyword::Var => {
                                self.parse_var_decl(par.clone(), mutable, tokens);

                                par = Parallelism::None;
                                mutable = false;
                            },
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    fn parse_var_decl(&mut self, par: Parallelism, mutable: bool, tokens: &mut Vec<Token>)
    {
        let mut id: String = String::new();
        let mut ty: Type = Type::None;
        
        let mut val: Option<Value> = None;

        loop {
            if let Some(t) = tokens.pop() {
                match t.kind()
                {
                    TokenKind::Type(t) => {
                        ty = t.to_owned();
                    },
                    TokenKind::Keyword(_) => {},
                    TokenKind::Identifier(idt) => {
                        if self.expects == Expects::Assignment
                        {
                            tokens.push(t);
                            val = Some(Value::Expression(self.parse_expr(tokens)));

                            self.expects = Expects::Nothing;
                        }
                        else {
                            id = idt.to_string();
                        }
                    },
                    TokenKind::Punctuation(p) => {
                        match p {
                            Punctuation::Assignment => 
                            {
                                self.expects = Expects::Assignment;
                            },
                            Punctuation::Colon | Punctuation::RangeInclusive => {
                                if self.expects == Expects::Assignment
                                {
                                    tokens.push(t);
                                    val = Some(Value::Expression(self.parse_expr(tokens)));

                                    self.expects = Expects::Nothing;
                                }
                            },
                            Punctuation::LParen => {
                                if self.expects == Expects::Assignment {
                                    tokens.push(t);
                                    val = Some(Value::Expression(self.parse_expr(tokens)));
                                    self.expects = Expects::Nothing;
                                }
                            },
                            Punctuation::SemiColon => break,
                            Punctuation::Quote => {
                                if let Some(s) = tokens.pop()
                                {
                                    val = Some(match s.kind() {
                                        TokenKind::Identifier(v) => Value::StringLiteral(v.to_string()),
                                        TokenKind::Value(v) => Value::StringLiteral(v.to_string()),
                                        _ => Value::StringLiteral("".to_string())
                                    });
                                }

                                let _next = tokens.pop(); // for unclosed quote
                            }
                            _ => {},
                        }
                    },
                    TokenKind::Operator(_) => {},
                    TokenKind::Value(_) => {
                        if self.expects == Expects::Assignment
                        {
                            tokens.push(t);
                            val = Some(Value::Expression(self.parse_expr(tokens)));

                            self.expects = Expects::Nothing;
                        }
                    },
                };
            }
            else {
                break;
            }
        }

        self.prog.add(Items::Var(
            VarDecl
            {
                par,
                mutable,
                id,
                ty,
                val
            }
        ));
    }

    fn parse_expr(&mut self, tokens: &mut Vec<Token>) -> Expr
    {
        let mut l = self.parse_term(tokens);

        loop {
            if let Some(t) = tokens.last().cloned()
            {
                match t.kind() {
                    TokenKind::Operator(o) => {
                        match o {
                            Operator::Add | Operator::Sub => {

                                let _ = tokens.pop();
                                let r = self.parse_term(tokens);

                                l = Expr::Binary(
                                    Binary
                                    {
                                        left: Box::new(l),
                                        op: o.to_owned(),
                                        right: Box::new(r),
                                    }
                                )
                            }
                            _ => return l
                        }
                    },
                    TokenKind::Punctuation(p) => {
                        match p {
                            Punctuation::Colon => {

                                let _ = tokens.pop();
                                
                                let sec = self.parse_term(tokens);

                                if let Some(tk) = tokens.last()
                                {
                                    match tk.kind() {
                                        TokenKind::Punctuation(p) => {
                                            match p {
                                                Punctuation::Colon => {

                                                    let _ = tokens.pop();

                                                    let thi = self.parse_term(tokens);

                                                    l = Expr::Range(
                                                        Range
                                                        {
                                                            start: Box::new(l),
                                                            step: Some(Box::new(sec)),
                                                            end: Box::new(thi),
                                                            inclusive: false
                                                        }
                                                    )
                                                },
                                                Punctuation::RangeInclusive => {
                                                    
                                                    let _ = tokens.pop();

                                                    let thi = self.parse_term(tokens);

                                                    l = Expr::Range(
                                                        Range
                                                        {
                                                            start: Box::new(l),
                                                            step: Some(Box::new(sec)),
                                                            end: Box::new(thi),
                                                            inclusive: true
                                                        }
                                                    )
                                                },
                                                _ => {
                                                    l = Expr::Range(
                                                        Range
                                                        {
                                                            start: Box::new(l),
                                                            step: None,
                                                            end: Box::new(sec),
                                                            inclusive: true
                                                        }
                                                    )
                                                }
                                            }
                                        },
                                        _ => {
                                            l = Expr::Range(
                                                Range
                                                {
                                                    start: Box::new(l),
                                                    step: None,
                                                    end: Box::new(sec),
                                                    inclusive: false
                                                }
                                            )
                                        }
                                    }
                                }
                            },
                            Punctuation::RangeInclusive => {
                                
                                let _ = tokens.pop();
                                
                                let sec = self.parse_term(tokens);

                                if let Some(tk) = tokens.last()
                                {
                                    match tk.kind() {
                                        TokenKind::Punctuation(p) => {
                                            match p {
                                                Punctuation::Colon => {

                                                    let _ = tokens.pop();
                                                    
                                                    let thi = self.parse_term(tokens);

                                                    l = Expr::Range(
                                                        Range
                                                        {
                                                            start: Box::new(l),
                                                            step: Some(Box::new(sec)),
                                                            end: Box::new(thi),
                                                            inclusive: false
                                                        }
                                                    )
                                                },
                                                Punctuation::RangeInclusive => {

                                                    let _ = tokens.pop();
                                                    
                                                    let thi = self.parse_term(tokens);

                                                    l = Expr::Range(
                                                        Range
                                                        {
                                                            start: Box::new(l),
                                                            step: Some(Box::new(sec)),
                                                            end: Box::new(thi),
                                                            inclusive: true
                                                        }
                                                    )
                                                },
                                                _ => {
                                                    l = Expr::Range(
                                                        Range
                                                        {
                                                            start: Box::new(l),
                                                            step: None,
                                                            end: Box::new(sec),
                                                            inclusive: true
                                                        }
                                                    )
                                                }
                                            }
                                        },
                                        _ => {
                                            l = Expr::Range(
                                                Range
                                                {
                                                    start: Box::new(l),
                                                    step: None,
                                                    end: Box::new(sec),
                                                    inclusive: true
                                                }
                                            )
                                        }
                                    }
                                }
                            },
                            Punctuation::RParen => return l,
                            _ => {
                                return l
                            }
                        }
                    }
                    _ => return l
                }
            }
        }
    }

    fn parse_term(&mut self, tokens: &mut Vec<Token>) -> Expr
    {
        let mut l = self.parse_factor(tokens);

        loop {
            if let Some(t) = tokens.last().cloned()
            {
                match t.kind() {
                    TokenKind::Operator(o) => {
                        match o {
                            Operator::Mul | Operator::Div => {

                                let _ = tokens.pop();
                                let r = self.parse_factor(tokens);

                                l = Expr::Binary(
                                    Binary
                                    {
                                        left: Box::new(l),
                                        op: o.to_owned(),
                                        right: Box::new(r),
                                    }
                                )
                            }
                            _ => return l
                        }
                    }
                    _ => return l
                }
            }
        }
    }

    fn parse_factor(&mut self, tokens: &mut Vec<Token>) -> Expr
    {
        if let Some(t) = tokens.pop()
        {
            match t.kind() {
                TokenKind::Identifier(id) => Expr::Identifier(id.to_string()),
                TokenKind::Value(val) => Expr::Literal(val.to_string()),
                TokenKind::Punctuation(Punctuation::LParen) => {
                    let inner = self.parse_expr(tokens);
                    let _ = tokens.pop();
                    Expr::Grouped(Box::new(inner))
                }
                _ => Expr::Null
            }
        }
        else {
            Expr::Null
        }
    }
    
    pub fn output(&self) -> &Program
    {
        println!("{:#?}", self.prog);
        &self.prog
    }
}