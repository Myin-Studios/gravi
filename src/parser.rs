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
    val: Option<Expr>
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

    pub fn identifier(&self) -> &String
    {
        &self.id
    }

    pub fn ty(&self) -> &Type
    {
        &self.ty
    }

    pub fn value(&self) -> &Option<Expr>
    {
        &self.val
    }
}

#[derive(Debug, Clone)]
pub enum Expr
{
    Literal(String),
    Identifier(String),
    Range(Range),
}

#[derive(Debug)]
pub enum ExprState
{
    Single,
    RangeNoStep,
    RangeStep,
    Binary,
    Unary,
    Nothing,
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
        
        let mut val: Option<Expr> = None;

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
                            val = Some(self.parse_expr(t, tokens));

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
                                    val = Some(self.parse_expr(t, tokens));

                                    self.expects = Expects::Nothing;
                                }
                            }
                            Punctuation::SemiColon => break,
                            _ => {},
                        }
                    },
                    TokenKind::Operator(_) => {},
                    TokenKind::Value(_) => {
                        if self.expects == Expects::Assignment
                        {
                            val = Some(self.parse_expr(t, tokens));

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

    fn parse_expr(&mut self, current: Token, tokens: &mut Vec<Token>) -> Expr
    {
        let mut v1: Option<Expr> = None;
        let mut v2: Option<Expr> = None;
        let mut v3: Option<Expr> = None;
        let mut inclusive: bool = false;
        let mut state: ExprState = ExprState::Nothing;

        match current.kind() {
            TokenKind::Identifier(id) => {
                v1 = Some(Expr::Identifier(id.to_string()));
                state = ExprState::Single;
            },
            TokenKind::Value(val) => {
                v1 = Some(Expr::Literal(val.to_string()));
                state = ExprState::Single;
            },
            TokenKind::Punctuation(p) => {
                match p {
                    Punctuation::Colon => {
                        v1 = Some(Expr::Literal("0".to_string()));
                        state = ExprState::RangeNoStep;

                        if let Some(t) = tokens.pop()
                        {
                            match t.kind() {
                                TokenKind::Identifier(id) => {
                                    v2 = Some(Expr::Identifier(id.to_string()));
                                },
                                TokenKind::Value(val) => {
                                    v2 = Some(Expr::Literal(val.to_string()));
                                },
                                _ => {}
                            }
                        }
                    },
                    Punctuation::RangeInclusive => {
                        v1 = Some(Expr::Literal("0".to_string()));
                        state = ExprState::RangeNoStep;
                        inclusive = true;

                        if let Some(t) = tokens.pop()
                        {
                            match t.kind() {
                                TokenKind::Identifier(id) => {
                                    v2 = Some(Expr::Identifier(id.to_string()));
                                },
                                TokenKind::Value(val) => {
                                    v2 = Some(Expr::Literal(val.to_string()));
                                },
                                _ => {}
                            }
                        }
                    },
                    _ => {}
                }
            }
            _ => {}
        };
        
        loop {
            match tokens.pop() {
                Some(t) => {
                    match t.kind() {
                        TokenKind::Punctuation(p) => {
                            match p {
                                Punctuation::Colon => {
                                    // Code for non-inclusive range
                                    if let Some(t) = tokens.pop()
                                    {
                                        match t.kind() {
                                            TokenKind::Identifier(id) => {
                                                if v2.is_none()
                                                {
                                                    v2 = Some(Expr::Identifier(id.to_string()));
                                                    state = ExprState::RangeNoStep;
                                                }
                                                else {
                                                    v3 = Some(Expr::Identifier(id.to_string()));
                                                    state = ExprState::RangeStep;
                                                }
                                            },
                                            TokenKind::Value(val) => {
                                                if v2.is_none()
                                                {
                                                    v2 = Some(Expr::Literal(val.to_string()));
                                                    state = ExprState::RangeNoStep;
                                                }
                                                else {
                                                    v3 = Some(Expr::Literal(val.to_string()));
                                                    state = ExprState::RangeStep;
                                                }
                                            },
                                            _ => {}
                                        }
                                    }
                                },
                                Punctuation::RangeInclusive => {
                                    // Code for inclusive range
                                    inclusive = true;

                                    if let Some(t) = tokens.pop()
                                    {
                                        match t.kind() {
                                            TokenKind::Identifier(id) => {
                                                if v2.is_none()
                                                {
                                                    v2 = Some(Expr::Identifier(id.to_string()));
                                                    state = ExprState::RangeNoStep;
                                                }
                                                else {
                                                    v3 = Some(Expr::Identifier(id.to_string()));
                                                    state = ExprState::RangeStep;
                                                }
                                            },
                                            TokenKind::Value(val) => {
                                                if v2.is_none()
                                                {
                                                    v2 = Some(Expr::Literal(val.to_string()));
                                                    state = ExprState::RangeNoStep;
                                                }
                                                else {
                                                    v3 = Some(Expr::Literal(val.to_string()));
                                                    state = ExprState::RangeStep;
                                                }
                                            },
                                            _ => {}
                                        }
                                    }
                                },
                                Punctuation::SemiColon => {
                                    match state {
                                        ExprState::Single => {
                                            return v1.unwrap_or(Expr::Identifier("null".to_string()))
                                        },
                                        ExprState::RangeNoStep => {
                                            return Expr::Range(
                                                Range
                                                {
                                                    start: Box::new(v1.unwrap_or(Expr::Identifier("null".to_string()))),
                                                    step: Some(Box::new(Expr::Literal("1".to_string()))),
                                                    end: Box::new(v2.unwrap_or(Expr::Identifier("null".to_string()))),
                                                    inclusive
                                                }
                                            );
                                        },
                                        ExprState::RangeStep => {
                                            return Expr::Range(
                                                Range
                                                {
                                                    start: Box::new(v1.unwrap_or(Expr::Identifier("null".to_string()))),
                                                    step: Some(Box::new(v2.unwrap_or(Expr::Identifier("null".to_string())))),
                                                    end: Box::new(v3.unwrap_or(Expr::Identifier("null".to_string()))),
                                                    inclusive
                                                }
                                            );
                                        },
                                        ExprState::Binary => {},
                                        ExprState::Unary => {},
                                        ExprState::Nothing => {},
                                    }
                                },
                                _ => {}
                            }
                        },
                        TokenKind::Operator(_) => {
                            // Code for binary expression
                        },
                        _ => {}
                    }
                },
                None => {
                    match state {
                        ExprState::Single => {
                            return v1.unwrap_or(Expr::Identifier("null".to_string()))
                        },
                        ExprState::RangeNoStep => {
                            return Expr::Range(
                                Range
                                {
                                    start: Box::new(v1.unwrap_or(Expr::Identifier("null".to_string()))),
                                    step: Some(Box::new(Expr::Literal("1".to_string()))),
                                    end: Box::new(v2.unwrap_or(Expr::Identifier("null".to_string()))),
                                    inclusive
                                }
                            );
                        },
                        ExprState::RangeStep => {
                            return Expr::Range(
                                Range
                                {
                                    start: Box::new(v1.unwrap_or(Expr::Identifier("null".to_string()))),
                                    step: Some(Box::new(v2.unwrap_or(Expr::Identifier("null".to_string())))),
                                    end: Box::new(v3.unwrap_or(Expr::Identifier("null".to_string()))),
                                    inclusive
                                }
                            );
                        },
                        ExprState::Binary => {},
                        ExprState::Unary => {},
                        ExprState::Nothing => {},
                    }
                },
            };
        }
    }
    
    pub fn output(&self) -> &Program
    {
        &self.prog
    }
}