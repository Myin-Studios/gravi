use colored::Colorize;

use crate::{error::{GraviError, Reporter}, lexer::*, ast::*};

#[derive(Clone, Debug)]
pub struct Parser
{
    prog: Program,
    rep:  Reporter,
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

impl Parser {
    pub fn new() -> Self
    {
        Self
        {
            prog: Program::new(),
            rep:  Reporter::new(),
        }
    }

    pub fn process(&mut self, tokens: &mut Vec<Token>)
    {
        tokens.reverse();

        let mut public = false;
        let mut mutable = false;
        let mut par = Parallelism::None;

        loop
        {
            if let Some(t) = tokens.pop()
            {
                match t.kind() {
                    TokenKind::Keyword(Keyword::With) => {
                        let mut spaces: Vec<Space> = Vec::new();

                        if let Some(next) = tokens.last().cloned()
                        {
                            match next.kind() {
                                TokenKind::Punctuation(Punctuation::LBrace) => {
                                    loop {
                                        if let Some(closing) = tokens.last()
                                        {
                                            if matches!(closing.kind(), TokenKind::Punctuation(Punctuation::RBrace) | TokenKind::Punctuation(Punctuation::SemiColon))
                                            {
                                                tokens.pop();
                                                break;
                                            }
                                            else if matches!(closing.kind(), TokenKind::Punctuation(Punctuation::Comma)) {
                                                tokens.pop();
                                            }
                                        }

                                        spaces.push(self.parse_space(tokens));
                                    }
                                },
                                TokenKind::Punctuation(Punctuation::SemiColon) => {
                                    tokens.pop();
                                },
                                TokenKind::Identifier(_) => {
                                    spaces.push(self.parse_space(tokens));
                                }
                                _ => {
                                    // error! unexpected token
                                }
                            }
                        }

                        if !spaces.is_empty() { self.prog.add(Global::Import(spaces)) }
                    },
                    TokenKind::Keyword(Keyword::Ext) => {
                        if matches!(tokens.last().map(|t| t.kind()), Some(TokenKind::Keyword(Keyword::Fun))) {
                            tokens.pop();
                            let fun = self.parse_function(tokens, public);
                            if let FunKind::Custom(f) = fun {
                                self.prog.add(Global::Fun(FunKind::Extern(f)));
                            }
                            public = false;
                        }
                    }
                    TokenKind::Keyword(Keyword::GPU) => par = Parallelism::GPU,
                    TokenKind::Keyword(Keyword::PAR) => par = Parallelism::CPU,
                    TokenKind::Keyword(Keyword::Mut) => mutable = true,
                    TokenKind::Keyword(Keyword::Pub) => public = true,
                    TokenKind::Keyword(Keyword::Var) => {
                        let var = self.parse_var_decl(&par, mutable, tokens);
                        self.prog.add(Global::Var(var));
                    },
                    TokenKind::Keyword(Keyword::Fun) => {
                        let fun = self.parse_function(tokens, public);
                        self.prog.add(Global::Fun(fun));
                        public = false;
                    },
                    TokenKind::Keyword(Keyword::Type) => {
                        let t = self.parse_type(public, tokens);
                        public = false;
                        self.prog.add(Global::Class(t));
                    },
                    _ => {}
                }
            }
            else {
                break;
            }
        }
    }

    fn parse_space(&mut self, tokens: &mut Vec<Token>) -> Space
    {
        let mut name: String = String::new();
        let mut sub: Option<Subspace> = None;

        loop {
            if let Some(t) = tokens.last().cloned()
            {
                match t.kind() {
                    TokenKind::Identifier(id) => {
                        tokens.pop();
                        name = id.to_string();
                    },
                    TokenKind::Punctuation(Punctuation::LBrace) => {
                        tokens.pop();
                    },
                    TokenKind::Punctuation(Punctuation::RBrace) => {
                        break;
                    },
                    TokenKind::Punctuation(Punctuation::RangeInclusive) => {
                        tokens.pop();
                        sub = self.parse_subspace(tokens);
                    },
                    TokenKind::Punctuation(Punctuation::Comma) => {
                        break;
                    },
                    _ => {
                        // error! unexpected token
                        break;
                    }
                }
            }
            else {
                break;
            }
        }
        
        Space
        {
            name,
            alias: None,
            sub,
            used: false
        }
    }

    fn parse_subspace(&mut self, tokens: &mut Vec<Token>) -> Option<Subspace>
    {
        let mut spaces: Vec<Space> = Vec::new();
        
        loop {
            if let Some(t) = tokens.last().cloned()
            {
                match t.kind() {
                    TokenKind::Identifier(_) => {
                        spaces.push(self.parse_space(tokens));

                        if let Some(next) = tokens.last() && matches!(next.kind(), TokenKind::Punctuation(Punctuation::Comma))
                        {
                            break;
                        }
                    },
                    TokenKind::Punctuation(Punctuation::RangeInclusive) => {
                        spaces.push(self.parse_space(tokens));
                    },
                    TokenKind::Punctuation(Punctuation::Comma) => {
                        tokens.pop();

                        spaces.push(self.parse_space(tokens));
                    },
                    TokenKind::Punctuation(Punctuation::LBrace) => {
                        spaces.push(self.parse_space(tokens));
                        
                        if let Some(next) = tokens.last()
                        {
                            if matches!(next.kind(), TokenKind::Punctuation(Punctuation::RBrace))
                            {
                                tokens.pop();
                                break;
                            }
                        }
                        else {
                            // error!
                            break;
                        }
                    },
                    TokenKind::Punctuation(Punctuation::RBrace) => {
                        tokens.pop();
                        break;
                    },
                    _ => { break; }
                }
            }
            else {
                break;
            }
        }

        Some(
            Subspace::Some(spaces)
        )
    }

    fn parse_type(&mut self, public: bool, tokens: &mut Vec<Token>) -> Class
    {
        let mut name = String::new();
        let mut impls: Vec<String> = Vec::new();
        let mut body: Vec<ClassBody> = Vec::new();

        if let Some(t) = tokens.pop()
        {
            match t.kind() {
                TokenKind::Identifier(id) => {
                    name = id.to_owned();

                    if let Some(next) = tokens.last()
                    {
                        if matches!(next.kind(), TokenKind::Punctuation(Punctuation::Colon))
                        {
                            tokens.pop();

                            impls.extend(self.parse_impl(tokens));
                        }
                    }
                },
                _ => {
                    self.rep.add(GraviError::throw(crate::error::Kind::UnexpectedToken(t.clone()))
                                            .file(t.file())
                                            .at(t.line(), t.column())
                                            .hint("Write a name of a type here!"));
                    tokens.push(t);
                }
            }
        }
        else {
            self.rep.add(GraviError::throw(crate::error::Kind::UnexpectedEOF));
        }

        loop {
            
            if let Some(t) = tokens.last().cloned()
            {
                if matches!(t.kind(), TokenKind::Punctuation(Punctuation::LBrace)) {
                    tokens.pop();
                    body.extend(self.parse_type_body(tokens));
                }
                else if matches!(t.kind(), TokenKind::Punctuation(Punctuation::RBrace)) {
                    tokens.pop();
                    break;
                }
            } else {
                self.rep.add(GraviError::throw(crate::error::Kind::UnexpectedEOF));
            }
        }

        let imp = if !impls.is_empty() { Some(impls) } else { None };

        Class
        {
            public,
            name,
            imp,
            body
        }   
    }
    
    fn parse_impl(&mut self, tokens: &mut Vec<Token>) -> Vec<String>
    {
        let mut impls: Vec<String> = Vec::new();
        let mut name = String::new();

        loop {
            if let Some(t) = tokens.pop()
            {
                match t.kind() {
                    TokenKind::Identifier(id) => {
                        name = id.to_owned();
                    },
                    TokenKind::Punctuation(Punctuation::Comma) => {
                        impls.push(name.clone());
                    },
                    TokenKind::Punctuation(Punctuation::LBrace) => {
                        impls.push(name);
                        tokens.push(t);
                        break;
                    },
                    _ => {
                        self.rep.add(GraviError::throw(crate::error::Kind::UnexpectedToken(t.clone()))
                                                    .file(t.file())
                                                    .at(t.line(), t.column())
                                                    .hint("Write a name of what this type extends or implements!"));
                        tokens.push(t);
                        break;
                    }
                }
            }
        }

        impls
    }

    fn parse_type_body(&mut self, tokens: &mut Vec<Token>) -> Vec<ClassBody>
    {
        let mut body: Vec<ClassBody> = Vec::new();

        let mut public = false;
        let mut par = Parallelism::None;
        let mut mutable = false;

        loop {
            if let Some(t) = tokens.pop()
            {
                match t.kind() {
                    TokenKind::Keyword(Keyword::GPU) => par = Parallelism::GPU,
                    TokenKind::Keyword(Keyword::PAR) => par = Parallelism::CPU,
                    TokenKind::Keyword(Keyword::Pub) => public = true,
                    TokenKind::Keyword(Keyword::Mut) => mutable = true,
                    TokenKind::Keyword(Keyword::Var) => {
                        body.push(ClassBody::Var(self.parse_var_decl(&par, mutable, tokens)));
                        par = Parallelism::None;
                        // public = false;
                        mutable = false;
                    },
                    TokenKind::Keyword(Keyword::Fun) => {
                        body.push(ClassBody::Fun(self.parse_function(tokens, public)));
                        public = false;
                    },
                    TokenKind::Punctuation(Punctuation::RBrace) => {
                        tokens.push(t);
                        break;
                    }
                    _ => {
                        self.rep.add(GraviError::throw(crate::error::Kind::UnexpectedToken(t.clone()))
                                            .file(t.file())
                                            .at(t.line(), t.column())
                                            .hint("Write a valid statement in a type! Like a variable or method (function) declaration"));
                        tokens.push(t);
                        break;
                    }
                }
            } else {
                self.rep.add(GraviError::throw(crate::error::Kind::UnexpectedEOF));
            }
        }

        body
    }

    fn parse_var_decl(&mut self, par: &Parallelism, mutable: bool, tokens: &mut Vec<Token>) -> VarDecl
    {
        let mut id:  String = String::new();
        let mut ty:  Type   = Type::None;
        let mut val: Option<Value> = None;
        let mut list: bool = false;

        loop {
            if let Some(t) = tokens.pop() {
                match t.kind()
                {
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
                                if let Some(next) = tokens.last() {
                                    match next.kind() {
                                        TokenKind::Type(t) => {
                                            ty = t.to_owned();
                                            tokens.pop();

                                            if tokens.last().map(|t| t.kind()) == Some(&TokenKind::Punctuation(Punctuation::LBracket)) {
                                                tokens.pop();
                                                if tokens.last().map(|t| t.kind()) == Some(&TokenKind::Punctuation(Punctuation::RBracket)) {
                                                    tokens.pop();
                                                    list = true;
                                                }
                                            }
                                        },
                                        _ => {
                                            self.rep.add(GraviError::throw(crate::error::Kind::ExpectedReturnType)
                                                                    .file(t.file())
                                                                    .at(t.line(), t.column())
                                                                    .hint(format!("Try writing a valid type, like numerics (u16, i16, f16, ...), string, bool or a user-defined type.\nBefore that, I'll consider this function with \"{}\" as its type!", "none".bright_blue().bold()).as_str()));

                                            break;
                                        }
                                    }
                                }
                            },
                            Punctuation::RParen => {
                                tokens.push(t);
                                break;
                            },
                            _ => break,
                        }
                    },
                    _ => {
                        self.rep.add(GraviError::throw(crate::error::Kind::UnexpectedToken(t.clone()))
                                                .file(t.file())
                                                .at(t.line(), t.column())
                                                .hint(format!("Try writing a valid token here. I don't know, like \"{}: {} = {};\"", "myvar".bright_blue().bold(), "mytype".bright_blue().bold(), "myvalue".bright_blue().bold()).as_str()));

                        break;
                    }
                };
            }
            else {
                break;
            }
        }

        VarDecl { par: par.clone(), mutable, id, ty, val, list }
    }

    fn parse_var(&mut self, tokens: &mut Vec<Token>) -> Variable
    {
        let id: String = if let Some(t) = tokens.pop()
        {
            match t.kind() {
                TokenKind::Identifier(name) => name.to_string(),
                _ => "".to_string()
            }
        }
        else {
            "".to_string()
        };

        let val = if let Some(t) = tokens.pop()
        {
            if t.kind() == &TokenKind::Punctuation(Punctuation::Assignment)
            {
                if let Some(val) = tokens.last()
                {
                    match val.kind() {
                        TokenKind::Identifier(_) | TokenKind::Value(_) |
                        TokenKind::Operator(_) | TokenKind::Keyword(Keyword::If) |
                        TokenKind::Punctuation(Punctuation::LBrace | Punctuation::LParen | Punctuation::LBracket) => Some(self.parse_value(tokens)),
                        _ => None
                    }
                }
                else {
                    None
                }
            }
            else {
                None
            }
        }
        else {
            None
        };

        Variable { name: id, val }
    }

    fn parse_value(&mut self, tokens: &mut Vec<Token>) -> Value
    {
        let mut val: Value = Value::Null;

        loop {
            if let Some(t) = tokens.last().cloned()
            {
                match t.kind() {
                    TokenKind::Punctuation(Punctuation::Colon | Punctuation::RangeInclusive) => {
                        val = Value::Expression(self.parse_binary(tokens, 0));
                        break;
                    },
                    TokenKind::Punctuation(Punctuation::Quote) => {
                        tokens.pop();

                        if let Some(next) = tokens.pop()
                        {
                            match next.kind() {
                                TokenKind::Identifier(v) | TokenKind::Value(ValueKind::String(v)) => {
                                    val = Value::StringLiteral(v.to_string());
                                    tokens.pop(); // consume closing quote
                                    break;
                                },
                                _ => break
                            }
                        }
                    },
                    TokenKind::Punctuation(Punctuation::LParen) => {
                        val = Value::Expression(self.parse_binary(tokens, 0));
                        break;
                    },
                    TokenKind::Operator(Operator::LNot | Operator::Sub) => {
                        val = Value::Expression(self.parse_binary(tokens, 0));
                        break;
                    },
                    // Dedicated boolean token (true / false)
                    TokenKind::Boolean(b) => {
                        tokens.pop();
                        val = Value::Boolean(if *b { BoolValue::True } else { BoolValue::False });
                        break;
                    },
                    TokenKind::Char(c) => {
                        tokens.pop();
                        val = Value::Char(*c);
                        break;
                    }
                    TokenKind::Identifier(v) | TokenKind::Value(ValueKind::String(v) | ValueKind::Numeric(v)) => {
                        let v = v.clone();
                        let temp = tokens.pop().unwrap();

                        if let Some(next) = tokens.last()
                        {
                            if next.kind() == &TokenKind::Punctuation(Punctuation::LParen)
                            {
                                let params: Vec<Value> = self.parse_args(tokens);
                                return Value::Call(v, params);
                            }
                            else if next.kind() == &TokenKind::Punctuation(Punctuation::LBracket) {
                                tokens.push(temp);
                                return Value::Expression(self.parse_binary(tokens, 0));
                            }
                            else {
                                tokens.push(temp);
                                return Value::Expression(self.parse_binary(tokens, 0));
                            };
                        }

                        val = Value::Null;
                        break;
                    },
                    TokenKind::Punctuation(Punctuation::RParen) => {
                        tokens.pop();
                        break;
                    },
                    TokenKind::Punctuation(Punctuation::LBrace) => {
                        tokens.pop();
                        let block = self.parse_block(tokens, false);
                        if tokens.last().map(|t| t.kind()) == Some(&TokenKind::Punctuation(Punctuation::RBrace)) {
                            tokens.pop();
                        }
                        val = Value::Block(Type::None, block);
                        break;
                    },
                    TokenKind::Keyword(Keyword::If) => {
                        tokens.pop();
                        val = Value::IfElse(self.parse_if(tokens, false));
                        break;
                    },
                    TokenKind::Punctuation(Punctuation::RBrace) => {
                        self.rep.add(GraviError::throw(crate::error::Kind::UnclosedParenthesis)
                                                .file(t.file())
                                                .at(t.line(), t.column())
                                                .hint(format!("Try writing {} to close the expression before this token.", "}".bright_blue().bold()).as_str()));

                        break;
                    },
                    TokenKind::Punctuation(Punctuation::LBracket) => {
                        let (s, v) = self.parse_list_literal(tokens);
                        val = Value::List(List::Decl(s, v));
                        break;
                    },
                    TokenKind::Punctuation(Punctuation::SemiColon) => break,
                    _ => {
                        self.rep.add(GraviError::throw(crate::error::Kind::ExpectedValue)
                                                .file(t.file())
                                                .at(t.line(), t.column())
                                                .hint("Write a valid value here, like a binary expression, an identifier, a literal (string or numeric), a range and so on."));

                        break;
                    }
                }
            }
            else {
                break;
            }
        }

        val
    }

    fn parse_range(&mut self, start: Expr, default_inclusive: bool, tokens: &mut Vec<Token>) -> Expr
    {
        let _ = tokens.pop();
        let sec = self.parse_term(tokens);

        if let Some(tk) = tokens.last()
        {
            match tk.kind() {
                TokenKind::Punctuation(Punctuation::Colon) => {
                    let _ = tokens.pop();
                    let thi = self.parse_term(tokens);

                    Expr::Range(Range {
                        start:     Box::new(start),
                        step:      Some(Box::new(sec)),
                        end:       Box::new(thi),
                        inclusive: false,
                    })
                },
                TokenKind::Punctuation(Punctuation::RangeInclusive) => {
                    let _ = tokens.pop();
                    let thi = self.parse_term(tokens);

                    Expr::Range(Range {
                        start:     Box::new(start),
                        step:      Some(Box::new(sec)),
                        end:       Box::new(thi),
                        inclusive: true,
                    })
                },
                _ => {
                    Expr::Range(Range {
                        start:     Box::new(start),
                        step:      None,
                        end:       Box::new(sec),
                        inclusive: default_inclusive,
                    })
                }
            }
        } else {
            Expr::Range(Range {
                start:     Box::new(start),
                step:      None,
                end:       Box::new(sec),
                inclusive: default_inclusive,
            })
        }
    }

    fn parse_binary(&mut self, tokens: &mut Vec<Token>, lvl: usize) -> Expr
    {
        if lvl > 5
        {
            return self.parse_expr(tokens);
        }

        let mut l = self.parse_binary(tokens, lvl + 1);

        loop {
            if let Some(t) = tokens.last().cloned()
            {
                match t.kind() {
                    TokenKind::Operator(o) => {

                        let (op, _, r) = match lvl {
                            0 if o == &Operator::LOr                       => (o.to_owned(), tokens.pop(), self.parse_binary(tokens, lvl + 1)),
                            1 if o == &Operator::LAnd                      => (o.to_owned(), tokens.pop(), self.parse_binary(tokens, lvl + 1)),
                            2 if o == &Operator::BWOr                      => (o.to_owned(), tokens.pop(), self.parse_binary(tokens, lvl + 1)),
                            3 if o == &Operator::BWAnd                     => (o.to_owned(), tokens.pop(), self.parse_binary(tokens, lvl + 1)),
                            4 if o == &Operator::Eq || o == &Operator::NEq => (o.to_owned(), tokens.pop(), self.parse_binary(tokens, lvl + 1)),
                            5 if o == &Operator::G || o == &Operator::L
                            || o == &Operator::GE || o == &Operator::LE    => (o.to_owned(), tokens.pop(), self.parse_binary(tokens, lvl + 1)),
                            _ => return l
                        };

                        l = Expr::Boolean(BinaryOp {
                            left:  Box::new(l),
                            op,
                            right: Box::new(r),
                        });
                    },
                    _ => return l
                }
            } else {
                return l;
            }
        }
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

                                l = Expr::Binary(BinaryOp {
                                    left:  Box::new(l),
                                    op:    o.to_owned(),
                                    right: Box::new(r),
                                });
                            },
                            _ => return l
                        }
                    },
                    TokenKind::Punctuation(p) => {
                        match p {
                            Punctuation::Colon | Punctuation::RangeInclusive => {
                                let default_inclusive = *p == Punctuation::RangeInclusive;
                                return self.parse_range(l, default_inclusive, tokens);
                            },
                            Punctuation::RParen => return l,
                            _ => return l
                        }
                    }
                    _ => return l
                }
            } else {
                return l;
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
                            Operator::Mul | Operator::Div | Operator::Mod | Operator::Pow => {

                                let _ = tokens.pop();
                                let r = self.parse_factor(tokens);

                                l = Expr::Binary(BinaryOp {
                                    left:  Box::new(l),
                                    op:    o.to_owned(),
                                    right: Box::new(r),
                                });
                            },
                            _ => return l
                        }
                    }
                    _ => return l
                }
            } else {
                return l;
            }
        }
    }

    fn parse_factor(&mut self, tokens: &mut Vec<Token>) -> Expr
    {
        if let Some(t) = tokens.pop()
        {
            match t.kind() {
                TokenKind::Identifier(id) => {
                    let id = id.to_string();

                    if tokens.last().map(|t| t.kind()) == Some(&TokenKind::Punctuation(Punctuation::LParen)) {
                        let args = self.parse_args(tokens);
                        Expr::Call(id, args)
                    } else if tokens.last().map(|t| t.kind()) == Some(&TokenKind::Punctuation(Punctuation::LBracket)) {
                        tokens.pop();
                        let idx = self.parse_binary(tokens, 0);
                        if tokens.last().map(|t| t.kind()) == Some(&TokenKind::Punctuation(Punctuation::RBracket)) {
                            tokens.pop();
                        }
                        Expr::Index(id, Box::new(idx))
                    } else {
                        Expr::Identifier(id)
                    }
                },
                TokenKind::Value(ValueKind::String(val))     => Expr::StringLiteral(val.to_string()),
                TokenKind::Value(ValueKind::Numeric(val))    => Expr::Literal(val.to_string()),
                TokenKind::Punctuation(Punctuation::LParen) => {
                    let inner = self.parse_binary(tokens, 0);

                    if let Some(closing) = tokens.last() {
                        if closing.kind() == &TokenKind::Punctuation(Punctuation::RParen) {
                            tokens.pop();
                        } else if closing.kind() == &TokenKind::Punctuation(Punctuation::SemiColon) {
                            tokens.pop();
                            return Expr::Grouped(Box::new(inner));
                        } else {
                            self.rep.add(GraviError::throw(crate::error::Kind::UnclosedParenthesis)
                                .file(t.file())
                                .at(t.line(), t.column())
                                .hint(format!("Try writing {} to close the grouped expression.", ")".bright_blue().bold()).as_str()));
                        }
                    } else {
                        self.rep.add(GraviError::throw(crate::error::Kind::UnclosedParenthesis)
                            .file(t.file())
                            .at(t.line(), t.column())
                            .hint(format!("Try writing {} to close the grouped expression.", ")".bright_blue().bold()).as_str()));
                    }

                    Expr::Grouped(Box::new(inner))
                },
                TokenKind::Operator(o) => {
                    match o {
                        Operator::LNot => {
                            Expr::Unary(Unary {
                                op:    Operator::LNot,
                                right: Box::new(self.parse_factor(tokens))
                            })
                        },
                        Operator::Sub => {
                            Expr::Unary(Unary {
                                op:    Operator::Sub,
                                right: Box::new(self.parse_factor(tokens))
                            })
                        },
                        _ => {
                            self.rep.add(GraviError::throw(crate::error::Kind::UnexpectedToken(t.clone()))
                                                .file(t.file())
                                                .at(t.line(), t.column())
                                                .hint("Try writing a valid expression here, like a binary expression, a boolean expression, an identifier, a literal or a range."));

                            Expr::Null
                        }
                    }
                },
                TokenKind::Char(c) => Expr::CharLiteral(*c),
                _ => {
                    self.rep.add(GraviError::throw(crate::error::Kind::UnexpectedToken(t.clone()))
                                                .file(t.file())
                                                .at(t.line(), t.column())
                                                .hint("Try writing a valid expression here, like a binary expression, a boolean expression, an identifier, a literal or a range."));

                    Expr::Null
                }
            }
        }
        else {
            Expr::Null
        }
    }

    fn parse_list_literal(&mut self, tokens: &mut Vec<Token>) -> (Expr, Option<Vec<Vec<Value>>>)
    {
        let (mut size, mut val) = (Expr::Null, Vec::new());
        let mut col: Vec<Value> = Vec::new();

        tokens.pop(); // consume '['

        if let Some(t) = tokens.last()
        {
            match t.kind() {
                TokenKind::Identifier(_) | TokenKind::Value(_) => {
                    size = self.parse_binary(tokens, 0);
                },
                _ => {
                    // error! unexpected token
                }
            }
        }

        tokens.pop(); // consume ']'

        loop {
            if let Some(t) = tokens.last()
            {
                match t.kind() {
                    TokenKind::Punctuation(Punctuation::LParen) | TokenKind::Punctuation(Punctuation::Comma) => {
                        tokens.pop();
                    },
                    TokenKind::Punctuation(Punctuation::RParen) => {
                        tokens.pop();
                        if !col.is_empty() { val.push(col.clone()); col.clear(); }
                        break;
                    },
                    TokenKind::Punctuation(Punctuation::SemiColon) => {
                        tokens.pop();
                        if !col.is_empty() { val.push(col.clone()); col.clear(); }
                    },
                    TokenKind::Identifier(_) | TokenKind::Value(_) | TokenKind::Punctuation(Punctuation::Quote) | TokenKind::Char(_) => {
                        col.push(self.parse_value(tokens));
                    }
                    _ => {
                        // error! unexpected token!
                        break;
                    }
                }
            }
        }

        (size, Some(val))
    }

    fn parse_function(&mut self, tokens: &mut Vec<Token>, public: bool) -> FunKind
    {
        let id = if let Some(t) = tokens.pop()
        {
            match t.kind() {
                TokenKind::Identifier(s) => s.to_string(),
                _ => {
                    self.rep.add(GraviError::throw(crate::error::Kind::ExpectedFunctionName)
                                            .file(t.file())
                                            .at(t.line(), t.column())
                                            .hint("Try typing a name without numbers or special characters as first character!"));
                    "".to_string()
                }
            }
        } else {
            "".to_string()
        };

        let main = id == "main";

        let mut params = Vec::new();
        let mut ret    = Type::None;
        let mut body   = Vec::new();

        loop {
            match tokens.last().map(|t| t.kind().clone()) {
                Some(TokenKind::Punctuation(Punctuation::LParen)) => {
                    params = self.parse_params(tokens); // gestisce `(` internamente
                },
                Some(TokenKind::Punctuation(Punctuation::Colon)) => {
                    tokens.pop();
                    if let Some(t) = tokens.pop() {
                        if let TokenKind::Type(ty) = t.kind() {
                            ret = ty.clone();
                        }
                    }
                },
                Some(TokenKind::Punctuation(Punctuation::LBrace)) => {
                    tokens.pop();
                    body = self.parse_block(tokens, false);
                    break;
                },
                Some(TokenKind::Punctuation(Punctuation::SemiColon | Punctuation::RBrace)) => {
                    tokens.pop();
                    break;
                },
                _ => break,
            }
        }

        if main {
            FunKind::Entry(Function { public, lambda: false, id: "main".to_string(), params, ret, body })
        } else {
            FunKind::Custom(Function { public, lambda: false, id, params, ret, body })
        }
    }

    fn parse_params(&mut self, tokens: &mut Vec<Token>) -> Vec<VarDecl> {
        let mut params = Vec::new();

        tokens.pop();

        loop {
            match tokens.last().map(|t| t.kind().clone()) {
                Some(TokenKind::Punctuation(Punctuation::RParen)) => {
                    tokens.pop();
                    break;
                },
                Some(TokenKind::Punctuation(Punctuation::Comma)) => {
                    tokens.pop();
                },
                None => break,
                _ => {
                    let mut par     = Parallelism::None;
                    let mut mutable = false;

                    loop {
                        match tokens.last().map(|t| t.kind().clone()) {
                            Some(TokenKind::Keyword(Keyword::Mut)) => { mutable = true; tokens.pop(); },
                            Some(TokenKind::Keyword(Keyword::PAR)) => { par = Parallelism::CPU; tokens.pop(); },
                            Some(TokenKind::Keyword(Keyword::GPU)) => { par = Parallelism::GPU; tokens.pop(); },
                            _ => break,
                        }
                    }

                    params.push(self.parse_var_decl(&par, mutable, tokens));
                }
            }
        }

        params
    }

    fn parse_block(&mut self, tokens: &mut Vec<Token>, top_level: bool) -> Vec<Items>
    {
        let mut mutable: bool        = false;
        let mut par: Parallelism     = Parallelism::None;
        let mut stmts: Vec<Items>    = Vec::new();

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
                                stmts.push(Items::Var(Var::Decl(self.parse_var_decl(&par, mutable, tokens))));
                                par     = Parallelism::None;
                                mutable = false;
                            },
                            Keyword::Fun if !top_level => {
                                self.rep.add(GraviError::throw(crate::error::Kind::UnsupportedStatement)
                                            .file(t.file())
                                            .at(t.line(), t.column())
                                            .hint(format!("Write a valid statement, like variable declarations, if-else statement, loop...\n\tYour nice function can't be declared inside a code block: \"{} ... {}\"!", "{".bright_blue().bold(), "}".bright_blue().bold()).as_str()));
                            },
                            Keyword::Ret if !top_level => {
                                stmts.push(Items::Ret(self.parse_value(tokens)));
                            },
                            Keyword::If => {
                                stmts.push(Items::Expr(Value::IfElse(self.parse_if(tokens, false))));
                            },
                            Keyword::Loop => {
                                stmts.push(Items::Expr(Value::Loop(self.parse_loop(tokens))));
                            },
                            Keyword::Stop => {
                                stmts.push(Items::Stop);
                            },
                            Keyword::Skip => {
                                stmts.push(Items::Skip);
                            },
                            _ => {
                                self.rep.add(GraviError::throw(crate::error::Kind::UnsupportedStatement)
                                            .file(t.file())
                                            .at(t.line(), t.column())
                                            .hint("Write a valid statement, like variable declarations, if-else statement, loop...\n\tNot function/class/interface declaration!"));

                                break;
                            }
                        }
                    },
                    TokenKind::Identifier(id) => {
                        let id = id.clone();
                        if tokens.last().map(|n| n.kind()) == Some(&TokenKind::Punctuation(Punctuation::LParen)) {
                            let params = self.parse_args(tokens);
                            stmts.push(Items::Expr(Value::Call(id, params)));
                        } else if tokens.last().map(|n| n.kind()) == Some(&TokenKind::Punctuation(Punctuation::LBracket)) {
                            let (indices, values) = self.parse_list(tokens);
                            stmts.push(Items::Expr(Value::List(List::Use(id, indices, values))));
                        } else {
                            tokens.push(t);
                            stmts.push(Items::Var(Var::Var(self.parse_var(tokens))));
                        }
                    },
                    TokenKind::Punctuation(Punctuation::LBrace) => {
                        stmts.push(Items::Expr(Value::Block(Type::None, self.parse_block(tokens, false))));

                        if let Some(next) = tokens.last() {
                            if next.kind() == &TokenKind::Punctuation(Punctuation::RBrace) {
                                tokens.pop();
                            }
                        }
                    },
                    TokenKind::Punctuation(Punctuation::RBrace) if !top_level => {
                        tokens.push(t);
                        break;
                    },
                    _ => {}
                }
            }
        }

        stmts
    }

    fn parse_args(&mut self, tokens: &mut Vec<Token>) -> Vec<Value>
    {
        let mut vals: Vec<Value> = Vec::new();

        let tok = tokens.pop(); // consume '('

        loop {
            if let Some(next) = tokens.last() {
                if next.kind() == &TokenKind::Punctuation(Punctuation::RParen) {
                    tokens.pop();
                    break;
                }
            } else {
                if let Some(t) = tok
                {
                    self.rep.add(GraviError::throw(crate::error::Kind::UnclosedParenthesis)
                                            .file(t.file())
                                            .at(t.line(), t.column())
                                            .hint(format!("Try writing {} to close the parameters list.", ")".bright_blue().bold()).as_str()));
                }
                break;
            }

            let v = self.parse_value(tokens);

            if matches!(v, Value::Null)
            {
                break;
            }

            vals.push(v);

            if let Some(next) = tokens.last() {
                if next.kind() == &TokenKind::Punctuation(Punctuation::Comma) {
                    tokens.pop();
                }
            }
        }

        vals
    }

    fn parse_if(&mut self, tokens: &mut Vec<Token>, is_else: bool) -> IfElse
    {
        let mut cond: Option<Box<Value>>      = None;
        let mut body: Vec<Items>        = Vec::new();
        let mut elif: Option<Box<IfElse>> = None;

        loop {
            if let Some(t) = tokens.last().cloned()
            {
                match t.kind() {
                    TokenKind::Punctuation(Punctuation::LParen) | TokenKind::Operator(_) | TokenKind::Value(_) | TokenKind::Identifier(_) => {
                        if is_else {
                            break;
                        }
                        cond = Some(Box::new(self.parse_value(tokens)));
                    },
                    TokenKind::Punctuation(Punctuation::LBrace) => {
                        tokens.pop();
                        body.extend(self.parse_block(tokens, false));
                    },
                    TokenKind::Punctuation(Punctuation::RBrace) => {
                        tokens.pop();

                        if let Some(next) = tokens.last()
                        {
                            if matches!(next.kind(), TokenKind::Keyword(Keyword::Else))
                            {
                                if is_else {
                                    break;
                                }
                                tokens.pop();

                                let mut next_is_else = true;

                                if let Some(is_elif) = tokens.last()
                                {
                                    if matches!(is_elif.kind(), TokenKind::Keyword(Keyword::If))
                                    {
                                        tokens.pop();
                                        next_is_else = false;
                                    }
                                }
                                else {
                                    break;
                                }

                                elif = Some(Box::new(self.parse_if(tokens, next_is_else)));
                                break;
                            }
                            else {
                                break;
                            }
                        }
                        else {
                            break;
                        }
                    },
                    _ => {
                        break;
                    }
                }
            }
            else {
                break;
            }
        }

        IfElse { cond, body, elif, ret: None }
    }

    fn parse_loop(&mut self, tokens: &mut Vec<Token>) -> Loop
    {
        let mut cond: Option<Box<VarDecl>> = None;
        let mut body: Vec<Items>         = Vec::new();

        let mut index = String::new();
        let mut val = Value::Null;

        loop {
            if let Some(t) = tokens.pop()
            {
                match t.kind() {
                    TokenKind::Identifier(id) => {
                        if let Some(next) = tokens.last()
                        {
                            if matches!(next.kind(), TokenKind::Keyword(Keyword::In))
                            {
                                index = id.to_owned();
                                tokens.pop();
                                val = self.parse_value(tokens);
                            } else {
                                tokens.push(t);
                                val = self.parse_value(tokens);
                            }
                        }
                    },
                    TokenKind::Punctuation(Punctuation::LParen) => {
                        tokens.push(t);
                        val = self.parse_value(tokens);
                    },
                    TokenKind::Value(_) => {
                        val = self.parse_value(tokens);
                    },
                    TokenKind::Punctuation(Punctuation::LBrace) => {
                        body = self.parse_block(tokens, false);
                    },
                    TokenKind::Punctuation(Punctuation::RBrace) => {
                        break;
                    },
                    _ => { break; }
                }
            }
            else {
                break;
            }
        }
        
        match val.clone() {
            Value::Expression(_) => {
                cond = Some(Box::new(VarDecl
                {
                    par: Parallelism::None,
                    mutable: false,
                    id: index,
                    ty: Type::Numeric(Numeric::U32),
                    val: Some(val),
                    list: false,
                }));
            },
            _ => {}
        }

        Loop
        {
            cond: cond,
            body: body
        }
    }

    fn parse_list(&mut self, tokens: &mut Vec<Token>) -> (Vec<Vec<Value>>, Option<Box<Value>>)
    {
        let mut indices = Vec::new();
        let mut v = Vec::new();
        let mut val = None;
        
        tokens.pop(); // consume '['
        
        loop {
            if let Some(t) = tokens.pop()
            {
                match t.kind() {
                    TokenKind::Identifier(_) | TokenKind::Value(_) => {
                        tokens.push(t);
                        v.push(self.parse_value(tokens));
                    },
                    TokenKind::Punctuation(p) => {
                        if matches!(p, Punctuation::Comma) {
                            v.push(self.parse_value(tokens));
                        } else if matches!(p, Punctuation::SemiColon) {
                            if !v.is_empty() { indices.push(v.clone()); }
                            v.clear();
                        } else if matches!(p, Punctuation::RBracket) {
                            if !v.is_empty() { indices.push(v.clone()); }
                            break;
                        } else {
                            // error! invalid token
                            self.rep.add(GraviError::throw(crate::error::Kind::UnexpectedToken(t)));
                            break;
                        }
                    },
                    _ => {
                        break;
                    }
                }
            }
            else {
                break;
            }
        }

        if let Some(t) = tokens.pop()
        {
            if matches!(t.kind(), TokenKind::Punctuation(Punctuation::Assignment))
            {
                val = Some(Box::new(self.parse_value(tokens)));
            }
        }

        (indices, val)
    }

    pub fn reporter(&self) -> &Reporter          { &self.rep }
    pub fn output(&self)   -> &Program           { println!("{:#?}", self.prog); &self.prog }
    pub fn output_mut(&mut self) -> &mut Program { &mut self.prog }
}
