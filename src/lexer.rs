use std::{fs::File, io::Read};

use colored::Colorize;

#[derive(PartialEq, Debug, Clone)]
pub enum Keyword
{
    Pub,
    Var,
    Mut,
    GPU,
    PAR,
    If,
    For,
    While,
    Else,
    ElseIf,
    Type,
    Class,
    Ext,
    Fun,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Numeric
{
    // Integers
    U8, U16, U32, U64,
    I8, I16, I32, I64,
    
    // Floating point
    F16, F32, F64,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Type
{
    Numeric(Numeric),
    StringLiteral(String),
    Custom(String),
    None,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Punctuation
{
    Dot,
    Spread,
    Colon,
    Comma,
    SemiColon,
    RangeInclusive,
    Assignment,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Operator
{
    Add,
    Sub,
    Mul,
    Div
}

#[derive(PartialEq, Debug, Clone)]
pub enum TokenKind
{
    Type(Type),
    Keyword(Keyword),
    Identifier(String),
    Punctuation(Punctuation),
    Value(String),
    Operator(Operator),
}

#[derive(PartialEq, Debug, Clone)]
pub struct Token
{
    kind: TokenKind,
    line: usize,
    column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self
    {
        Self
        {
            kind,
            line,
            column
        }
    }

    pub fn kind(&self) -> &TokenKind
    {
        &self.kind
    }
}

pub struct Lexer
{
    content: String,
    chars: Vec<char>,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
}

impl Lexer {
    pub fn new(path: &str) -> Self
    {
        let mut f = File::open(path).expect("Unable to open this file!");
        let mut content = String::new();
        let _ = f.read_to_string(&mut content);
        content = content.replace("\r\n", "\n");

        let mut chars: Vec<char> = content.chars().collect();
        chars.reverse();
        
        Self
        {
            content,
            chars,
            line: 1,
            column: 1,
            tokens: Vec::new(),
        }
    }

    fn has(&mut self) -> bool
    {
        if self.chars.is_empty()
        {
            return false;
        }

        let mut word = String::new();
        let mut curr;
        let mut special = ' ';

        loop {
            self.column += 1;

            curr = self.advance();

            if self.current() == ';'
            {
                word.push(curr);
                break;
            }

            if self.current() == ' '
            {
                word.push(curr);
                break;
            }
            if curr == ' '
            {
                break;
            }
            else if self.current() == ':'
            {
                if curr == ':'
                {
                    word.push(curr);
                    continue;
                }

                word.push(curr);
                break;
            }
            else if curr == ':' {
                if self.current().is_alphanumeric()
                {
                    word.push(curr);
                    break;
                }
            }
            else if self.current() == '\n' || self.current() == '\0' || self.current() == ';'
            || self.current() == '+' || self.current() == '-' || self.current() == '*' || self.current() == '/' {
                word.push(curr);
                break;
            }
            else if curr == '+' || curr == '-' || curr == '*' || curr == '/' {
                if self.current().is_ascii_digit()
                {
                    word.push(curr);
                    break;
                }
                else
                {
                    let lines: Vec<&str> = self.content.lines().collect();
                    let error_base = vec![' ' as u8; lines[self.line - 1].len() - 1];
                    let mut error = String::from_utf8(error_base).unwrap();
                    error.replace_range(self.column - 1..self.column, "^");
                    println!("{}", format!("{}\n|\t{}\n\t{}", "[ERROR]".red(), lines[self.line - 1], error.red()))
                }
            }
            else if curr == '\n' {
                special = '\n';
                break;
            }

            word.push(curr);
        }

        if !word.is_empty() && word != " " && word != "\n"
        {
            if let Some(c) = word.chars().next()
            {
                if c.is_ascii_digit() {
                    self.tokens.push(Token::new(TokenKind::Value(word.clone()), self.line, self.column - word.len()));
                    return true;
                }
            }

            self.tokens.push(
                match word.as_str() {
                    // Keywords
                    "pub" => Token::new(TokenKind::Keyword(Keyword::Pub), self.line, self.column - word.len()),
                    "var" => Token::new(TokenKind::Keyword(Keyword::Var), self.line, self.column - word.len()),
                    "mut" => Token::new(TokenKind::Keyword(Keyword::Mut), self.line, self.column - word.len()),
                    "if" => Token::new(TokenKind::Keyword(Keyword::If), self.line, self.column - word.len()),
                    "else" => Token::new(TokenKind::Keyword(Keyword::Else), self.line, self.column - word.len()),
                    "else if" => Token::new(TokenKind::Keyword(Keyword::ElseIf), self.line, self.column - word.len()),
                    "for" => Token::new(TokenKind::Keyword(Keyword::For), self.line, self.column - word.len()),
                    "while" => Token::new(TokenKind::Keyword(Keyword::While), self.line, self.column - word.len()),
                    "!GPU" => Token::new(TokenKind::Keyword(Keyword::GPU), self.line, self.column - word.len()),
                    "!PAR" => Token::new(TokenKind::Keyword(Keyword::PAR), self.line, self.column - word.len()),
                    "fun" => Token::new(TokenKind::Keyword(Keyword::Fun), self.line, self.column - word.len()),
                    "class" => Token::new(TokenKind::Keyword(Keyword::Class), self.line, self.column - word.len()),
                    "type" => Token::new(TokenKind::Keyword(Keyword::Type), self.line, self.column - word.len()),
                    "ext" => Token::new(TokenKind::Keyword(Keyword::Ext), self.line, self.column - word.len()),

                    // Types
                    "u8"  => Token::new(TokenKind::Type(Type::Numeric(Numeric::U8)), self.line, self.column - word.len()),
                    "u16" => Token::new(TokenKind::Type(Type::Numeric(Numeric::U16)), self.line, self.column - word.len()),
                    "u32" => Token::new(TokenKind::Type(Type::Numeric(Numeric::U32)), self.line, self.column - word.len()),
                    "u64" => Token::new(TokenKind::Type(Type::Numeric(Numeric::U64)), self.line, self.column - word.len()),
                    "i8"  => Token::new(TokenKind::Type(Type::Numeric(Numeric::I8)), self.line, self.column - word.len()),
                    "i16" => Token::new(TokenKind::Type(Type::Numeric(Numeric::I16)), self.line, self.column - word.len()),
                    "i32" => Token::new(TokenKind::Type(Type::Numeric(Numeric::I32)), self.line, self.column - word.len()),
                    "i64" => Token::new(TokenKind::Type(Type::Numeric(Numeric::I64)), self.line, self.column - word.len()),
                    "f16" => Token::new(TokenKind::Type(Type::Numeric(Numeric::F16)), self.line, self.column - word.len()),
                    "f32" => Token::new(TokenKind::Type(Type::Numeric(Numeric::F32)), self.line, self.column - word.len()),
                    "f64" => Token::new(TokenKind::Type(Type::Numeric(Numeric::F64)), self.line, self.column - word.len()),

                    // Punctuation
                    // "." => Token::new(TokenKind::Punctuation(Punctuation::Dot), line, col),
                    ".." => Token::new(TokenKind::Punctuation(Punctuation::Spread), self.line, self.column - word.len()),
                    ":" => Token::new(TokenKind::Punctuation(Punctuation::Colon), self.line, self.column - word.len()),
                    "::" => Token::new(TokenKind::Punctuation(Punctuation::RangeInclusive), self.line, self.column - word.len()),
                    "," => Token::new(TokenKind::Punctuation(Punctuation::Comma), self.line, self.column - word.len()),
                    ";" => Token::new(TokenKind::Punctuation(Punctuation::SemiColon), self.line, self.column - word.len()),
                    "=" => Token::new(TokenKind::Punctuation(Punctuation::Assignment), self.line, self.column - word.len()),

                    // Operator
                    "+" => Token::new(TokenKind::Operator(Operator::Add), self.line, self.column - word.len()),
                    "-" => Token::new(TokenKind::Operator(Operator::Sub), self.line, self.column - word.len()),
                    "*" => Token::new(TokenKind::Operator(Operator::Mul), self.line, self.column - word.len()),
                    "/" => Token::new(TokenKind::Operator(Operator::Div), self.line, self.column - word.len()),

                    _ => Token::new(TokenKind::Identifier(word.to_string()), self.line, self.column - word.len())
                }
            );
        }
        else if word == "\n" {
            self.column = 1;
            self.line += 1;
        }
        
        if special == '\n'
        {
            self.column = 1;
            self.line += 1;
        }

        true
    }

    fn advance(&mut self) -> char
    {
        self.chars.pop().unwrap_or(' ')
    }

    fn current(&self) -> char
    {
        if self.chars.len() < 1
        {
            return '\0';
        }

        self.chars[self.chars.len() - 1]
    }

    pub fn process(&mut self)
    {
        
        loop {
            if !self.has()
            {
                break;
            }
        }
    }

    pub fn tokens(&self) -> &Vec<Token>
    {
        &self.tokens
    }

    pub fn tokens_mut(&mut self) -> &mut Vec<Token>
    {
        &mut self.tokens
    }
}