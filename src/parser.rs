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

#[derive(Debug)]
pub struct VarDecl
{
    gpu: bool,
    mutable: bool,
    id: String,
    ty: Type,
    val: Option<Expr>
}

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Range
{
    start: Box<Expr>,
    step: Option<Box<Expr>>,
    end: Box<Expr>,
    inclusive: bool,
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

        let mut gpu: bool = false;
        let mut mutable: bool = false;

        loop {
            if tokens.is_empty()
            {
                break;
            }

            if let Some(t) = tokens.pop()
            {
                if t.kind() == &TokenKind::Keyword(Keyword::GPU)
                {
                    gpu = true;
                    continue;
                }
                if t.kind() == &TokenKind::Keyword(Keyword::Mut)
                {
                    mutable = true;
                    continue;
                }
                if t.kind() == &TokenKind::Keyword(Keyword::Var)
                {
                    self.parse_var_decl(gpu, mutable, tokens);
                }
            }
        }

        for item in self.prog.items.iter().clone()
        {
            println!("{:?}", item);
        }
    }

    fn parse_var_decl(&mut self, gpu: bool, mutable: bool, tokens: &mut Vec<Token>)
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
                    TokenKind::Keyword(kw) => {},
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
                    TokenKind::Operator(op) => {},
                    TokenKind::Value(v) => {
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
                gpu,
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
                        TokenKind::Operator(op) => {
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
}