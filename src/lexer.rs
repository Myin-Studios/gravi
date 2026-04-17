use std::{fmt, fs::File, io::Read};

use colored::Colorize;

use crate::error::{GraviError, Reporter};

#[derive(PartialEq, Debug, Clone)]
pub enum Keyword
{
    With,
    Pub,
    Var,
    Mut,
    GPU,
    PAR,
    If,
    Loop,
    In,
    Stop,
    Skip,
    Else,
    Type,
    Class,
    Ext,
    Fun,
    Ret,
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

impl fmt::Display for Numeric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Numeric::U8  => "u8",
            Numeric::U16 => "u16",
            Numeric::U32 => "u32",
            Numeric::U64 => "u64",
            Numeric::I8  => "i8",
            Numeric::I16 => "i16",
            Numeric::I32 => "i32",
            Numeric::I64 => "i64",
            Numeric::F16 => "f16",
            Numeric::F32 => "f32",
            Numeric::F64 => "f64",
        };
        write!(f, "{}", s)
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Type
{
    Numeric(Numeric),
    StringLiteral,
    Boolean,
    Character,
    Custom(String),
    None,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Numeric(n)  => write!(f, "{}", n),
            Type::StringLiteral => write!(f, "string"),
            Type::Boolean       => write!(f, "bool"),
            Type::Character     => write!(f, "char"),
            Type::Custom(c)     => write!(f, "{}", c),
            Type::None          => write!(f, "none"),
        }
    }
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
    SingleQuote,
    Quote,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace
}

#[derive(PartialEq, Debug, Clone)]
pub enum Operator
{
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    LAnd,
    LOr,
    LNot,
    BWOr,  // bit-wise
    BWAnd, // bit-wise
    Eq,
    NEq,
    GE,
    LE,
    G,
    L,
    None
}

#[derive(PartialEq, Debug, Clone)]
pub enum TokenKind
{
    Type(Type),
    Keyword(Keyword),
    Identifier(String),
    Punctuation(Punctuation),
    Value(String),
    Char(char),
    Boolean(bool),
    Operator(Operator),
}

#[derive(PartialEq, Debug, Clone)]
pub struct Token
{
    kind:   TokenKind,
    file:   String,
    line:   usize,
    column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, file: &String, line: usize, column: usize) -> Self
    {
        Self { kind, file: file.to_string(), line, column }
    }

    pub fn kind(&self) -> &TokenKind { &self.kind }
    pub fn file(&self) -> &str       { &self.file }
    pub fn line(&self) -> usize      { self.line }
    pub fn column(&self) -> usize    { self.column }
}

pub struct Lexer
{
    chars:  Vec<char>,
    line:   usize,
    column: usize,
    tokens: Vec<Token>,

    file: String,
    rep:  Reporter
}

impl Lexer {
    pub fn new(path: &str) -> Self
    {
        let readed_file = File::open(path);

        match readed_file {
            Ok(mut f) => {
                let mut content = String::new();
                let _ = f.read_to_string(&mut content);
                content = content.replace("\r\n", "\n");

                let mut chars: Vec<char> = content.chars().collect();
                chars.reverse();

                drop(content);

                Self
                {
                    chars,
                    line: 1,
                    column: 1,
                    tokens: Vec::new(),

                    file: std::path::Path::new(path).file_name().unwrap_or_default().to_str().unwrap_or_default().to_string(),
                    rep: Reporter::new(),
                }
            },
            Err(_) => {
                let mut rep = Reporter::new();

                rep.add(GraviError::throw(crate::error::Kind::FileNotFound(path.to_string()))
                                    .hint("Try opening another file."));

                Self
                {
                    chars: Vec::new(),
                    line: 1,
                    column: 1,
                    tokens: Vec::new(),

                    file: std::path::Path::new(path).file_name().unwrap_or_default().to_str().unwrap_or_default().to_string(),
                    rep,
                }
            },
        }
    }

    fn is_special(&mut self)
    {
        if self.next() == ' '
        {
            let _ = self.advance();
        }
        else if self.next() == '\n' {
            let _ = self.advance();
            self.column = 1;
            self.line += 1;
        }
        else if self.next() == '\t' {
            let _ = self.advance();
        }
    }

    fn is_comment(&mut self)
    {
        if self.next() == '/'
        {
            let mut c = self.advance();

            if self.next() == '/'
            {
                loop {
                    self.advance();

                    if self.next() == '\n' || self.next() == '\0'
                    {
                        self.advance();
                        self.line += 1;
                        self.column = 1;
                        break;
                    }
                }
            }
            else if self.next() == '*' {
                let start_line = self.line;
                let start_col  = self.column;
                let mut nested: usize = 0;

                loop {
                    self.is_special();

                    c = self.advance();

                    if c == '/' && self.next() == '*'
                    {
                        let _ = self.advance();
                        nested += 1;
                    }
                    else if c == '*' && self.next() == '/'
                    {
                        let _ = self.advance();

                        if nested > 0 {
                            for _ in 0..nested {
                                self.rep.add(GraviError::throw(crate::error::Kind::UnterminatedComment)
                                    .severity(crate::error::Severity::Warning)
                                    .file(&self.file)
                                    .at(start_line, start_col)
                                    .hint(format!("There are {} unclosed inner {} inside this block comment.", nested, "/*".bright_blue().bold()).as_str()));
                            }
                        }

                        break;
                    }
                    else if self.next() == '\0'
                    {
                        self.rep.add(GraviError::throw(crate::error::Kind::UnterminatedComment)
                                                .file(&self.file)
                                                .at(start_line, start_col)
                                                .hint(format!("Try writing {} to end your monologue.", "*/".bright_blue().bold()).as_str()));

                        break;
                    }
                }
            }
            else {
                self.chars.push(c);
            }
        }
    }

    fn is_punctuation(&self, ignore: Option<char>) -> bool {
        let c = self.next();
        if Some(c) == ignore { return false; }
        matches!(c, '.' | ',' | ':' | ';' | '=' | '+' | '-' | '*' | '%' | '^' | '/' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '&' | '!' | '<' | '>')
    }

    fn read_string_literal(&mut self) -> String
    {
        let mut word: String = String::new();

        loop {
            if self.next() != '"' && self.next() != '\n' && self.next() != '\0'
            {
                word.push(self.advance());
            }
            else if self.next() == '\n' || self.next() == '\0' {
                self.rep.add(GraviError::throw(crate::error::Kind::UnterminatedString)
                                                .file(&self.file)
                                                .at(self.line, self.column)
                                                .hint(format!("Try writing {} to end your monologue.", "\"".bright_blue().bold()).as_str()));

                break;
            }
            else {
                break;
            }
        }

        word
    }

    fn read_op(&mut self, next: char) -> String
    {
        let mut word: String = String::new();

        let c = self.advance();
        self.column += 1;

        if self.next() == next
        {
            word.push_str(format!("{}{}", c, self.advance()).as_str());
        }
        else {
            word.push(c);
        }

        word
    }

    fn read_atom(&mut self) -> String
    {
        let mut word: String = String::new();

        if self.next().is_alphabetic()
        {
            word.push(self.advance());

            loop {
                if !self.is_punctuation(None) && !self.next().is_whitespace() && self.next() != '\0'
                {
                    word.push(self.advance());
                }
                else {
                    break;
                }
            }
        }
        else if self.next().is_ascii_digit()
        {
            word.push(self.advance());

            loop {
                if self.next() != '\n' && !self.is_punctuation(Some('.')) && !self.next().is_whitespace() && self.next() != '\0'
                {
                    word.push(self.advance());
                }
                else {
                    break;
                }
            }
        }
        else if self.next() == '!' {
            let mut temp = String::new();

            loop {
                if !self.is_punctuation(None) && !self.next().is_whitespace() && self.next() != '\0'
                {
                    temp.push(self.advance());
                }
                else {
                    break;
                }
            }

            if matches!(temp.as_str(), "!GPU" | "!PAR")
            {
                word.push_str(&temp);
            }
            else {
                self.chars.extend(temp.chars().rev());
                self.column -= temp.len();
                word.push(self.advance());
            }
        }

        word
    }

    fn what(&self, word: &str) -> Token
    {
        match word {
            "with"   => Token::new(TokenKind::Keyword(Keyword::With),   &self.file, self.line, self.column - word.len()),
            "pub"   => Token::new(TokenKind::Keyword(Keyword::Pub),   &self.file, self.line, self.column - word.len()),
            "var"   => Token::new(TokenKind::Keyword(Keyword::Var),   &self.file, self.line, self.column - word.len()),
            "mut"   => Token::new(TokenKind::Keyword(Keyword::Mut),   &self.file, self.line, self.column - word.len()),
            "if"    => Token::new(TokenKind::Keyword(Keyword::If),    &self.file, self.line, self.column - word.len()),
            "else"  => Token::new(TokenKind::Keyword(Keyword::Else),  &self.file, self.line, self.column - word.len()),
            "loop"   => Token::new(TokenKind::Keyword(Keyword::Loop), &self.file, self.line, self.column - word.len()),
            "in"   => Token::new(TokenKind::Keyword(Keyword::In),     &self.file, self.line, self.column - word.len()),
            "stop" => Token::new(TokenKind::Keyword(Keyword::Stop),   &self.file, self.line, self.column - word.len()),
            "skip" => Token::new(TokenKind::Keyword(Keyword::Skip),   &self.file, self.line, self.column - word.len()),
            "!GPU"  => Token::new(TokenKind::Keyword(Keyword::GPU),   &self.file, self.line, self.column - word.len()),
            "!PAR"  => Token::new(TokenKind::Keyword(Keyword::PAR),   &self.file, self.line, self.column - word.len()),
            "fun"   => Token::new(TokenKind::Keyword(Keyword::Fun),   &self.file, self.line, self.column - word.len()),
            "class" => Token::new(TokenKind::Keyword(Keyword::Class), &self.file, self.line, self.column - word.len()),
            "type"  => Token::new(TokenKind::Keyword(Keyword::Type),  &self.file, self.line, self.column - word.len()),
            "ext"   => Token::new(TokenKind::Keyword(Keyword::Ext),   &self.file, self.line, self.column - word.len()),
            "ret"   => Token::new(TokenKind::Keyword(Keyword::Ret),   &self.file, self.line, self.column - word.len()),

            // Types
            "u8"     => Token::new(TokenKind::Type(Type::Numeric(Numeric::U8)),  &self.file, self.line, self.column - word.len()),
            "u16"    => Token::new(TokenKind::Type(Type::Numeric(Numeric::U16)), &self.file, self.line, self.column - word.len()),
            "u32"    => Token::new(TokenKind::Type(Type::Numeric(Numeric::U32)), &self.file, self.line, self.column - word.len()),
            "u64"    => Token::new(TokenKind::Type(Type::Numeric(Numeric::U64)), &self.file, self.line, self.column - word.len()),
            "i8"     => Token::new(TokenKind::Type(Type::Numeric(Numeric::I8)),  &self.file, self.line, self.column - word.len()),
            "i16"    => Token::new(TokenKind::Type(Type::Numeric(Numeric::I16)), &self.file, self.line, self.column - word.len()),
            "i32"    => Token::new(TokenKind::Type(Type::Numeric(Numeric::I32)), &self.file, self.line, self.column - word.len()),
            "i64"    => Token::new(TokenKind::Type(Type::Numeric(Numeric::I64)), &self.file, self.line, self.column - word.len()),
            "f16"    => Token::new(TokenKind::Type(Type::Numeric(Numeric::F16)), &self.file, self.line, self.column - word.len()),
            "f32"    => Token::new(TokenKind::Type(Type::Numeric(Numeric::F32)), &self.file, self.line, self.column - word.len()),
            "f64"    => Token::new(TokenKind::Type(Type::Numeric(Numeric::F64)), &self.file, self.line, self.column - word.len()),
            "bool"   => Token::new(TokenKind::Type(Type::Boolean),               &self.file, self.line, self.column - word.len()),
            "char"   => Token::new(TokenKind::Type(Type::Character),             &self.file, self.line, self.column - word.len()),
            "string" => Token::new(TokenKind::Type(Type::StringLiteral),         &self.file, self.line, self.column - word.len()),

            // Boolean literals
            "true"  => Token::new(TokenKind::Boolean(true),  &self.file, self.line, self.column - word.len()),
            "false" => Token::new(TokenKind::Boolean(false), &self.file, self.line, self.column - word.len()),

            // Punctuation
            "."  => Token::new(TokenKind::Punctuation(Punctuation::Dot),            &self.file, self.line, self.column - word.len()),
            ".." => Token::new(TokenKind::Punctuation(Punctuation::Spread),         &self.file, self.line, self.column - word.len()),
            ":"  => Token::new(TokenKind::Punctuation(Punctuation::Colon),          &self.file, self.line, self.column - word.len()),
            "::" => Token::new(TokenKind::Punctuation(Punctuation::RangeInclusive), &self.file, self.line, self.column - word.len()),
            ","  => Token::new(TokenKind::Punctuation(Punctuation::Comma),          &self.file, self.line, self.column - word.len()),
            ";"  => Token::new(TokenKind::Punctuation(Punctuation::SemiColon),      &self.file, self.line, self.column - word.len()),
            "="  => Token::new(TokenKind::Punctuation(Punctuation::Assignment),     &self.file, self.line, self.column - word.len()),
            "'"  => Token::new(TokenKind::Punctuation(Punctuation::SingleQuote),    &self.file, self.line, self.column - word.len()),
            "\"" => Token::new(TokenKind::Punctuation(Punctuation::Quote),          &self.file, self.line, self.column - word.len()),
            "("  => Token::new(TokenKind::Punctuation(Punctuation::LParen),         &self.file, self.line, self.column - word.len()),
            ")"  => Token::new(TokenKind::Punctuation(Punctuation::RParen),         &self.file, self.line, self.column - word.len()),
            "["  => Token::new(TokenKind::Punctuation(Punctuation::LBracket),       &self.file, self.line, self.column - word.len()),
            "]"  => Token::new(TokenKind::Punctuation(Punctuation::RBracket),       &self.file, self.line, self.column - word.len()),
            "{"  => Token::new(TokenKind::Punctuation(Punctuation::LBrace),         &self.file, self.line, self.column - word.len()),
            "}"  => Token::new(TokenKind::Punctuation(Punctuation::RBrace),         &self.file, self.line, self.column - word.len()),

            // Operators
            "+"  => Token::new(TokenKind::Operator(Operator::Add),   &self.file, self.line, self.column - word.len()),
            "-"  => Token::new(TokenKind::Operator(Operator::Sub),   &self.file, self.line, self.column - word.len()),
            "*"  => Token::new(TokenKind::Operator(Operator::Mul),   &self.file, self.line, self.column - word.len()),
            "/"  => Token::new(TokenKind::Operator(Operator::Div),   &self.file, self.line, self.column - word.len()),
            "%"  => Token::new(TokenKind::Operator(Operator::Mod),   &self.file, self.line, self.column - word.len()),
            "^"  => Token::new(TokenKind::Operator(Operator::Pow),   &self.file, self.line, self.column - word.len()),
            "!"  => Token::new(TokenKind::Operator(Operator::LNot),  &self.file, self.line, self.column - word.len()),
            "||" => Token::new(TokenKind::Operator(Operator::LOr),   &self.file, self.line, self.column - word.len()),
            "&&" => Token::new(TokenKind::Operator(Operator::LAnd),  &self.file, self.line, self.column - word.len()),
            "|"  => Token::new(TokenKind::Operator(Operator::BWOr),  &self.file, self.line, self.column - word.len()),
            "&"  => Token::new(TokenKind::Operator(Operator::BWAnd), &self.file, self.line, self.column - word.len()),
            "=="  => Token::new(TokenKind::Operator(Operator::Eq),   &self.file, self.line, self.column - word.len()),
            "!="  => Token::new(TokenKind::Operator(Operator::NEq),  &self.file, self.line, self.column - word.len()),
            ">="  => Token::new(TokenKind::Operator(Operator::GE),   &self.file, self.line, self.column - word.len()),
            "<="  => Token::new(TokenKind::Operator(Operator::LE),   &self.file, self.line, self.column - word.len()),
            ">"  => Token::new(TokenKind::Operator(Operator::G),     &self.file, self.line, self.column - word.len()),
            "<"  => Token::new(TokenKind::Operator(Operator::L),     &self.file, self.line, self.column - word.len()),

            _ => Token::new(TokenKind::Identifier(word.to_string()), &self.file, self.line, self.column - word.len())
        }
    }

    fn tokenize_next(&mut self) -> bool
    {
        if self.chars.is_empty()
        {
            return false
        }

        let mut word: String = String::new();

        self.is_special();
        self.is_comment();

        match self.next() {
            ':' => {
                let c = self.advance();

                if self.next() == ':'
                {
                    word.push_str(format!("{}{}", c, self.advance()).as_str());
                }
                else {
                    word.push(c);
                }
            },
            '.' => {
                let c = self.advance();
                self.column += 1;

                if self.next() == '.'
                {
                    word.push_str(format!("{}{}", c, self.advance()).as_str());
                }
                else {
                    word.push(c);
                }
            },
            '+' | '-' | '*' | '/' | '%' | '^' | ';' | ',' | '(' | ')' | '[' | ']' | '{' | '}' => {
                word.push(self.advance());
            },
            '"' => {
                self.advance();

                self.tokens.push(Token::new(
                    TokenKind::Punctuation(Punctuation::Quote), &self.file, self.line, self.column - 1
                ));

                if self.next() != '"'
                {
                    let s = self.read_string_literal();

                    self.tokens.push(Token::new(
                        TokenKind::Value(s.clone()), &self.file, self.line, self.column - s.len()
                    ));
                }

                if self.next() == '\0'
                {
                    self.rep.add(GraviError::throw(crate::error::Kind::UnterminatedString)
                                        .file(&self.file)
                                        .at(self.line, self.column)
                                        .hint(format!("Close with '{}'", "\"".bright_blue().bold()).as_str()));

                    self.advance();
                }
                else {
                    self.advance();

                    self.tokens.push(Token::new(
                        TokenKind::Punctuation(Punctuation::Quote), &self.file, self.line, self.column - 1
                    ));
                }
            },
            '\'' => {
                self.advance();

                if self.next() != '\''
                {
                    let c = self.advance();
                    self.tokens.push(Token::new(TokenKind::Char(c), &self.file, self.line, self.column - 1));
                }

                if self.advance() != '\''
                {
                    // error! unclosed character
                }
            },
            '|' => {
                word.push_str(&self.read_op('|'));
            },
            '&' => {
                word.push_str(&self.read_op('&'));
            },
            '!' => {
                word.push_str(&self.read_op('='));
            },
            '=' => {
                word.push_str(&self.read_op('='));
            },
            '>' => {
                word.push_str(&self.read_op('='));
            },
            '<' => {
                word.push_str(&self.read_op('='));
            },
            '#' | '@' | '$' | '_' => {
                self.rep.add(GraviError::throw(crate::error::Kind::UnknownChar(self.next()))
                                        .file(&self.file)
                                        .at(self.line, self.column)
                                        .hint("Try removing it."));

                self.advance();
            }
            _ => {
                word = self.read_atom();
            }
        }

        if let Some(c) = word.chars().next()
        {
            if c.is_ascii_digit() {
                self.tokens.push(Token::new(TokenKind::Value(word.clone()), &self.file, self.line, self.column - word.len()));
                return true
            }
        }

        if !word.is_empty()
        {
            self.tokens.push(self.what(word.as_str()));
        }

        true
    }

    fn advance(&mut self) -> char
    {
        self.column += 1;
        self.chars.pop().unwrap_or(' ')
    }

    fn next(&self) -> char
    {
        if self.chars.is_empty()
        {
            return '\0';
        }

        self.chars[self.chars.len() - 1]
    }

    pub fn process(&mut self)
    {
        loop {
            if !self.tokenize_next()
            {
                break;
            }
        }
    }

    pub fn tokens(&self) -> &Vec<Token>    { &self.tokens }
    pub fn tokens_mut(&mut self) -> &mut Vec<Token> { &mut self.tokens }
    pub fn reporter(&self) -> &Reporter    { &self.rep }
}
