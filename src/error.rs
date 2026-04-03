use colored::Colorize;

use crate::lexer::Token;

pub enum Severity
{
    Fatal,
    Error,
    Warning,
    Info
}

pub enum Kind
{
    FileNotFound(String),
    UnterminatedComment,
    UnterminatedString,
    UnknownChar(char),

    UnexpectedToken(Token),
    UnexpectedEOF,
    UnclosedParenthesis,
    ExpectedIdentifier,
    ExpectedType,
    ExpectedFunctionName,
    ExpectedReturnType,
    ExpectedValue,
    UnsupportedStatement,

    UnsupportedExpression,
    UnsupportedReturnType,
    InvalidParameter(usize)
}

pub struct NyonError {
    pub kind:     Kind,
    pub severity: Severity,
    pub file:     Option<String>,
    pub line:     Option<usize>,
    pub col:      Option<usize>,
    pub hint:     Option<String>,
}

impl NyonError {
    pub fn throw(kind: Kind) -> Self
    {
        Self
        {
            kind,
            severity: Severity::Error,
            file: None,
            line: None,
            col: None,
            hint: None
        }
    }

    pub fn severity(self, sev: Severity) -> Self
    {
        Self
        {
            severity: sev,
            ..self
        }
    }

    pub fn file(self, file: &str) -> Self
    {
        Self
        {
            file: Some(String::from(file)),
            ..self
        }
    }

    pub fn at(self, l: usize, c: usize) -> Self
    {
        Self
        {
            line: Some(l),
            col: Some(c),
            ..self
        }
    }

    pub fn hint(self, hint: &str) -> Self
    {
        Self
        {
            hint: Some(String::from(hint)),
            ..self
        }
    }

    pub fn fire(&self)
    {
        let severity = match self.severity {
            Severity::Fatal   => "fatal".magenta().bold(),
            Severity::Error   => "error".bright_red().bold(),
            Severity::Warning => "warning".yellow().bold(),
            Severity::Info    => "info".cyan().bold(),
        };

        let msg = self.kind.message();
        println!("{}! {}", severity, msg.white().bold());

        if let (Some(file), Some(line), Some(col)) = (&self.file, self.line, self.col) {
            println!(" {} {}:[{}:{}]", "\tat", file.bright_blue().bold(), line, col);
        }

        if let Some(hint) = &self.hint {
            println!("  {} {} {}", "|", "You should...".green().bold(), hint.green());
        }
    }
}

impl Kind {
    pub fn message(&self) -> String
    {
        match self {
            Kind::FileNotFound(path) =>
            format!("Unable to open the file at: \"{}\"", path.white().bold()),
            Kind::UnterminatedString => "You should terminate a string literal!".to_string(),
            Kind::UnterminatedComment => "How long is this comment?!".to_string(),
            Kind::UnknownChar(c) => format!("Hm? {}?", c.to_string().bright_blue().bold()),
            
            Kind::UnexpectedToken(token) => format!("{:#?}\n^ I found this token! Are you sure it's the right one?", token),
            Kind::UnexpectedEOF => format!("Uh! I reached the \"{}\" (End Of File). Did you forget anything back?", "EOF".white().bold()),
            Kind::UnclosedParenthesis => "Where's the end of this call? You surely forgot to close the parenthesis!".to_string(),
            Kind::ExpectedIdentifier => "There's something missing here, but... What? Oh, an identifier!".to_string(),
            Kind::ExpectedType => "Ok, a null-type identifier! Right? Right?!".to_string(),
            Kind::ExpectedFunctionName => "A nice function needs a nice name!".to_string(),
            Kind::ExpectedReturnType => format!("Well... You maybe want \"{}\" as a type?\nNot after putting that ':'!", "none".bright_blue().bold()),
            Kind::ExpectedValue => "Go, go! Tell me more! Equals to...?".to_string(),
            Kind::UnsupportedStatement => "Mhmhmh! Not here, not now...".to_string(),

            Kind::UnsupportedExpression => "Mh... Why would you like to do this? It's not allowed here!".to_string(),
            Kind::UnsupportedReturnType => "Are you kidding me? What's that?! An unsupported return type!".to_string(),
            Kind::InvalidParameter(pos) => format!("Do whatever you want, but the parameter at the position {} is not valid!", pos),
        }
    }
}

pub struct Reporter
{
    err: Vec<NyonError>,
    warn: Vec<NyonError>,
    info: Vec<NyonError>
}

impl Reporter {
    pub fn new() -> Self
    {
        Self
        {
            err: Vec::new(),
            warn: Vec::new(),
            info: Vec::new(),
        }
    }

    pub fn add(&mut self, msg: NyonError)
    {
        match msg.severity {
            Severity::Fatal | Severity::Error => self.err.push(msg),
            Severity::Warning => self.warn.push(msg),
            Severity::Info => self.info.push(msg),
        }
    }

    pub fn has_errors(&self) -> bool
    {
        !self.err.is_empty()
    }
    
    pub fn fire_all(&self)
    {
        for err in &self.err
        {
            err.fire();
        }

        for warn in &self.warn
        {
            warn.fire();
        }

        for info in &self.info
        {
            info.fire();
        }
    }
}