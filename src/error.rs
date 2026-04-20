use colored::Colorize;

use crate::lexer::{Token, Type};

#[derive(Clone, Debug)]
pub enum Severity
{
    Fatal,
    Error,
    Warning,
    Info
}

#[derive(Clone, Debug)]
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
    UnterminatedStatement(usize),
    
    InvalidImport(String),
    PrivateImport(String),
    DuplicateImport(String),

    TooManyEntry,
    TypeMismatch(Type, Type),
    UndeclaredVariable(String),
    UninitializedVariable(String),
    UntypedVariable(String),
    MutatingImmutable(String),

    EntryNotFound,
    UnsupportedExpression,
    UnsupportedReturnType,
    InvalidParameter(usize),
}

#[derive(Clone, Debug)]
pub struct GraviError {
    kind:     Kind,
    severity: Severity,
    file:     Option<String>,
    line:     Option<usize>,
    col:      Option<usize>,
    hint:     Option<String>,
}

impl GraviError {
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
        eprintln!("{}! {}", severity, msg.white().bold());

        if let (Some(file), Some(line), Some(col)) = (&self.file, self.line, self.col) {
            eprintln!(" {} {}:[{}:{}]", "\tat", file.bright_blue().bold(), line, col);
        }

        if let Some(hint) = &self.hint {
            eprintln!("  {} {} {}", "|", "You should...".green().bold(), hint.green());
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
            Kind::UnterminatedStatement(line) => format!("Hey! The started line {} is not terminated!", line.to_string().bright_blue().bold()),

            Kind::InvalidImport(name) => format!("{} is a directory!", name.bright_blue().bold()),
            Kind::PrivateImport(name) => format!("Where where? I can't see it... Maybe {} is private?", name.bright_blue().bold()),
            Kind::DuplicateImport(name) => format!("Wait! I remember this... Are you trying to import {} twice?", name.bright_blue().bold()),

            Kind::TooManyEntry => format!("W-Wait wait wait! I found too many {}", "entry points".bright_blue().bold()),
            Kind::TypeMismatch(expected, found) => format!("A-A-A... You mismatched the type! Expected {}, but found {}!", expected.to_string().green().bold(), found.to_string().red().bold()),
            Kind::UndeclaredVariable(name) => format!("Hm... Where {} come from?!", name.bright_blue().bold()),
            Kind::UninitializedVariable(name) => format!("Be careful! {} is not initialized!", name.bright_blue().bold()),
            Kind::UntypedVariable(name) => format!("Waaait! {}'s type is {}!", name.bright_black().bold(), "none".red().bold()),
            Kind::MutatingImmutable(name) => format!("No! Don't touch {}... It's not mutable!", name.bright_blue().bold()),

            Kind::EntryNotFound => "Hmm... Where are you?? My dear entry point!".to_string(),
            Kind::UnsupportedExpression => "Mh... Why would you like to do this? It's not allowed here!".to_string(),
            Kind::UnsupportedReturnType => "Are you kidding me? What's that?! An unsupported return type!".to_string(),
            Kind::InvalidParameter(pos) => format!("Do whatever you want, but the parameter at the position {} is not valid!", pos),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Reporter
{
    messages: Vec<GraviError>,
}

impl Reporter {
    pub fn new() -> Self
    {
        Self
        {
            messages: Vec::new(),
        }
    }

    pub fn add(&mut self, msg: GraviError)
    {
        self.messages.push(msg);
    }

    pub fn has_errors(&self) -> bool
    {
        self.messages.iter().any(|m| matches!(m.severity, Severity::Fatal | Severity::Error))
    }

    pub fn fire_all(&self)
    {
        for msg in &self.messages {
            if matches!(msg.severity, Severity::Fatal | Severity::Error) {
                msg.fire();
            }
        }

        for msg in &self.messages {
            if matches!(msg.severity, Severity::Warning) {
                msg.fire();
            }
        }

        for msg in &self.messages {
            if matches!(msg.severity, Severity::Info) {
                msg.fire();
            }
        }
    }
}
