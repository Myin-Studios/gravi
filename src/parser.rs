use crate::lexer::*;

#[derive(Debug)]
pub struct Program
{
    items: Vec<Items>,
}

#[derive(Debug)]
pub enum Items
{
    Var(VarDecl),
    Fun(Function),
    Ret(Value),
    None,
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
    Boolean(String),
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

#[derive(Debug)]
pub struct Function
{
    lambda: bool,
    main: bool,
    id: String,
    params: Vec<VarDecl>,
    ret: Type,
    body: Vec<Items>
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

    pub fn lambda(&self) -> &bool
    {
        &self.lambda
    }

    pub fn main(&self) -> &bool
    {
        &self.main
    }
    
    pub fn identifier(&self) -> &String
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

#[derive(PartialEq, Debug)]
pub enum Expects
{
    Type,
    Assignment,
    Body,
    Nothing,
}

pub struct Parser
{
    prog: Program,
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
                                let var = Items::Var(self.parse_var_decl(&par, mutable, tokens));
                                self.prog.add(var);

                                par = Parallelism::None;
                                mutable = false;
                            },
                            Keyword::Fun => {
                                let fun = self.parse_function(tokens);
                                self.prog.add(fun);
                            }
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    fn parse_var_decl(&mut self, par: &Parallelism, mutable: bool, tokens: &mut Vec<Token>) -> VarDecl
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
                        id = idt.to_string();
                    },
                    TokenKind::Punctuation(p) => {
                        match p {
                            Punctuation::Assignment => 
                            {
                                val = Some(self.parse_value(tokens));
                                break;
                            },
                            Punctuation::Colon => {
                                if let Some(next) = tokens.last()
                                {
                                    match next.kind() {
                                        TokenKind::Type(t) => {
                                            ty = t.to_owned();
                                        },
                                        _ => break
                                    };

                                    tokens.pop();
                                }
                            },
                            Punctuation::RParen => {
                                tokens.push(t);
                                break;
                            }
                            _ => break,
                        }
                    },
                    _ => break
                };
            }
            else {
                break;
            }
        }

        VarDecl
        {
            par: par.clone(),
            mutable,
            id,
            ty,
            val
        }
    }

    fn parse_value(&mut self, tokens: &mut Vec<Token>) -> Value
    {
        let mut val: Value = Value::Null;

        loop {
            if let Some(t) = tokens.last()
            {
                match t.kind() {
                    TokenKind::Punctuation(Punctuation::Colon | Punctuation::RangeInclusive) => { // :end or ::end or :step:end or :step::end
                        val = Value::Expression(self.parse_expr(tokens));
                        break;
                    },
                    TokenKind::Punctuation(Punctuation::Quote) => { // "some string literal"
                        tokens.pop();
                        
                        if let Some(next) = tokens.pop()
                        {
                            match next.kind() {
                                TokenKind::Identifier(v) | TokenKind::Value(v) => {
                                    val = Value::StringLiteral(v.to_string());
                                    break;
                                },
                                _ => break
                            }
                        }
                    },
                    TokenKind::Punctuation(Punctuation::LParen) => {
                        val = Value::Expression(self.parse_expr(tokens));                        
                    }
                    TokenKind::Identifier(v) | TokenKind::Value(v) => { // true/false or some identifier
                        val = if v == &"true".to_string()
                        {
                            tokens.pop();
                            Value::Boolean("true".to_string())
                        }
                        else if v == &"false".to_string() {
                            tokens.pop();
                            Value::Boolean("false".to_string())
                        }
                        else {
                            Value::Expression(self.parse_expr(tokens))
                        }
                    },
                    _ => break
                }
            }
        }

        val
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

    fn parse_function(&mut self, tokens: &mut Vec<Token>) -> Items
    {
        let id = if let Some(t) = tokens.pop()
        {
            match t.kind() {
                TokenKind::Identifier(s) => s.to_string(),
                _ => "".to_string() // error!
            }
        } else {
            "".to_string() // error!
        };

        let main = id == "main".to_string();

        let mut params: Vec<VarDecl> = Vec::new();
        
        let mut ret = Type::None;

        let mut body: Vec<Items> = Vec::new();

        loop {
            if let Some(t) = tokens.pop()
            {
                if t.kind() == &TokenKind::Punctuation(Punctuation::LParen)
                {
                    loop {
                        let mut par = Parallelism::None;
                        let mut mutable = false;

                        if let Some(next) = tokens.last() {
                            if next.kind() == &TokenKind::Punctuation(Punctuation::RParen) {
                                tokens.pop();
                                break;
                            }
                            else if next.kind() == &TokenKind::Keyword(Keyword::Mut)
                            {
                                mutable = true;
                                tokens.pop();
                            }
                            else if next.kind() == &TokenKind::Keyword(Keyword::PAR) {
                                par = Parallelism::CPU;
                                tokens.pop();
                            }
                            else if next.kind() == &TokenKind::Keyword(Keyword::GPU) {
                                par = Parallelism::GPU;
                                tokens.pop();
                            }
                        } else {
                            break;
                        }

                        params.push(self.parse_var_decl(&par, mutable, tokens));
                    }
                }
                else if t.kind() == &TokenKind::Punctuation(Punctuation::Colon) {
                    if let Some(next) = tokens.last()
                    {
                        match next.kind() {
                            TokenKind::Type(ty) => {
                                ret = ty.to_owned();
                                tokens.pop();
                            },
                            _ => {} // error! unsupported type!
                        }
                    }
                }
                else if t.kind() == &TokenKind::Punctuation(Punctuation::LBrace) {
                    body.extend(self.parse_block(tokens));
                }
                else if t.kind() == &TokenKind::Punctuation(Punctuation::SemiColon)
                    || t.kind() == &TokenKind::Punctuation(Punctuation::RBrace) {
                    break;
                }
            }
        }

        Items::Fun(
            Function
            {
                lambda: false,
                main,
                id,
                params,
                ret,
                body,
            }
        )
    }

    fn parse_block(&mut self, tokens: &mut Vec<Token>) -> Vec<Items>
    {
        let mut mutable: bool = false;
        let mut par: Parallelism = Parallelism::None;
        let mut stmts: Vec<Items> = Vec::new();

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
                                stmts.push(Items::Var(self.parse_var_decl(&par, mutable, tokens)));
                                par = Parallelism::None;
                                mutable = false;
                            },
                            Keyword::Ret => {
                                stmts.push(Items::Ret(self.parse_value(tokens)));
                            }
                            _ => { break; } // error! unsupported code inside a code block! (like function declarations, only lambda functions are supported)
                        }
                    },
                    TokenKind::Punctuation(Punctuation::RBrace) => {
                        tokens.push(t);
                        break;
                    }
                    _ => {}
                }
            }
        }

        stmts
    }
    
    pub fn output(&self) -> &Program
    {
        println!("{:#?}", self.prog);
        &self.prog
    }
}