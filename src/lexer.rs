use std::{fs::File, io::Read};

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

        drop(content);
        
        Self
        {
            chars,
            line: 1,
            column: 1,
            tokens: Vec::new(),
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
                loop {
                    self.is_special(); // multiline comments

                    c = self.advance();

                    if c == '*'
                    {
                        if self.next() == '/'
                        {
                            let _ = self.advance();
                            
                            break;
                        }
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
        matches!(c, '.' | ',' | ':' | ';' | '=' | '+' | '-' | '*' | '/' | '(' | ')' | '[' | ']' | '{' | '}')
    }

    fn read_string_literal(&mut self) -> String
    {
        let mut word: String = String::new();

        loop {
            if self.next() != '"'
            {
                word.push(self.advance());
            }
            else {
                break;
            }
        }

        word
    }
    
    fn read_word(&mut self) -> String
    {
        let mut word: String = String::new();
        
        if self.next().is_alphabetic() || self.next() == '!'
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

        word
    }

    fn what(&self, word: &str) -> Token
    {
        match word {
            "pub" => Token::new(TokenKind::Keyword(Keyword::Pub), self.line, self.column - word.len()),
            "var" => Token::new(TokenKind::Keyword(Keyword::Var), self.line, self.column - word.len()),
            "mut" => Token::new(TokenKind::Keyword(Keyword::Mut), self.line, self.column - word.len()),
            "if" => Token::new(TokenKind::Keyword(Keyword::If), self.line, self.column - word.len()),
            "else" => Token::new(TokenKind::Keyword(Keyword::Else), self.line, self.column - word.len()),
            "for" => Token::new(TokenKind::Keyword(Keyword::For), self.line, self.column - word.len()),
            "while" => Token::new(TokenKind::Keyword(Keyword::While), self.line, self.column - word.len()),
            "!GPU" => Token::new(TokenKind::Keyword(Keyword::GPU), self.line, self.column - word.len()),
            "!PAR" => Token::new(TokenKind::Keyword(Keyword::PAR), self.line, self.column - word.len()),
            "fun" => Token::new(TokenKind::Keyword(Keyword::Fun), self.line, self.column - word.len()),
            "class" => Token::new(TokenKind::Keyword(Keyword::Class), self.line, self.column - word.len()),
            "type" => Token::new(TokenKind::Keyword(Keyword::Type), self.line, self.column - word.len()),
            "ext" => Token::new(TokenKind::Keyword(Keyword::Ext), self.line, self.column - word.len()),
            "ret" => Token::new(TokenKind::Keyword(Keyword::Ret), self.line, self.column - word.len()),

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
            "bool" => Token::new(TokenKind::Type(Type::Boolean), self.line, self.column - word.len()),
            "char" => Token::new(TokenKind::Type(Type::Character), self.line, self.column - word.len()),
            "string" => Token::new(TokenKind::Type(Type::StringLiteral), self.line, self.column - word.len()),
            
            // Punctuation
            "." => Token::new(TokenKind::Punctuation(Punctuation::Dot), self.line, self.column - word.len()),
            ".." => Token::new(TokenKind::Punctuation(Punctuation::Spread), self.line, self.column - word.len()),
            ":" => Token::new(TokenKind::Punctuation(Punctuation::Colon), self.line, self.column - word.len()),
            "::" => Token::new(TokenKind::Punctuation(Punctuation::RangeInclusive), self.line, self.column - word.len()),
            "," => Token::new(TokenKind::Punctuation(Punctuation::Comma), self.line, self.column - word.len()),
            ";" => Token::new(TokenKind::Punctuation(Punctuation::SemiColon), self.line, self.column - word.len()),
            "=" => Token::new(TokenKind::Punctuation(Punctuation::Assignment), self.line, self.column - word.len()),
            "'" => Token::new(TokenKind::Punctuation(Punctuation::SingleQuote), self.line, self.column - word.len()),
            "\"" => Token::new(TokenKind::Punctuation(Punctuation::Quote), self.line, self.column - word.len()),
            "(" => Token::new(TokenKind::Punctuation(Punctuation::LParen), self.line, self.column - word.len()),
            ")" => Token::new(TokenKind::Punctuation(Punctuation::RParen), self.line, self.column - word.len()),
            "[" => Token::new(TokenKind::Punctuation(Punctuation::LBracket), self.line, self.column - word.len()),
            "]" => Token::new(TokenKind::Punctuation(Punctuation::RBracket), self.line, self.column - word.len()),
            "{" => Token::new(TokenKind::Punctuation(Punctuation::LBrace), self.line, self.column - word.len()),
            "}" => Token::new(TokenKind::Punctuation(Punctuation::RBrace), self.line, self.column - word.len()),

            // Operator
            "+" => Token::new(TokenKind::Operator(Operator::Add), self.line, self.column - word.len()),
            "-" => Token::new(TokenKind::Operator(Operator::Sub), self.line, self.column - word.len()),
            "*" => Token::new(TokenKind::Operator(Operator::Mul), self.line, self.column - word.len()),
            "/" => Token::new(TokenKind::Operator(Operator::Div), self.line, self.column - word.len()),

            _ => Token::new(TokenKind::Identifier(word.to_string()), self.line, self.column - word.len())
        }
    }

    fn has(&mut self) -> bool
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
            '=' | '+' | '-' | '*' | '/' | ';' | ',' | '(' | ')' | '[' | ']' | '{' | '}' | '\'' => {
                word.push(self.advance());
            },
            '"' => {
                self.advance();

                self.tokens.push(Token::new(
                    TokenKind::Punctuation(Punctuation::Quote), self.line, self.column - 1
                ));

                if self.next() != '"'
                {
                    let s = self.read_string_literal();

                    self.tokens.push(Token::new(
                        TokenKind::Value(s.clone()), self.line, self.column - s.len()
                    ));
                }
                
                self.advance();

                self.tokens.push(Token::new(
                    TokenKind::Punctuation(Punctuation::Quote), self.line, self.column - 1
                ));
            }
            _ => {
                word = self.read_word();
            }
        }

        if let Some(c) = word.chars().next()
        {
            if c.is_ascii_digit() {
                self.tokens.push(Token::new(TokenKind::Value(word.clone()), self.line, self.column - word.len()));
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

        println!("\n{:#?}", self.tokens);
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