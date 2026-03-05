pub mod lexer;

fn main()
{
    let mut l = lexer::Lexer::new("./examples/test.nv");
    l.process();
}