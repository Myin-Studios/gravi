use std::{fs::File, io::Write};

pub mod lexer;
pub mod parser;
pub mod codegen;

fn main()
{
    let mut l = lexer::Lexer::new("./examples/test.nv");
    l.process();

    let mut p = parser::Parser::new();
    p.process(l.tokens_mut());

    let mut cg = codegen::Generator::new();
    cg.process(p.output());

    let _ = std::fs::create_dir("out");

    let mut f = File::create("out/out.c").unwrap();
    let _ = f.write_all(cg.output().as_bytes()).unwrap();

    let _ = std::process::Command::new("gcc").arg("out/out.c").arg("-o").arg("out/test.exe").arg("-fopenmp").output();

    let _ = std::fs::remove_file("out/out.c");
}