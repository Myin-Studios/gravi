pub mod lexer;
pub mod parser;

fn main()
{
    let mut l = lexer::Lexer::new("./examples/test.nv");
    l.process();

    let mut p = parser::Parser::new();
    p.process(l.tokens_mut());
}