use std::{collections::HashSet, fs::File, io::Write, path};

use colored::Colorize;

pub mod ast;
pub mod lexer;
pub mod parser;
pub mod symbol;
pub mod resolver;
pub mod typechecker;
pub mod codegen;
pub mod backend;
pub mod error;

use backend::Backend;
use codegen::{BackendType, BuildFlag, Target};

use crate::symbol::SymbolTable;

fn info()
{
    println!("\n{} - version: {}", "Aion".purple().bold(), "alpha-0.1".white().bold());
    println!("Copyright © {} {}\n", "Myin Studios".bright_blue().bold(), "2026".white().bold());
    println!("Type {} to open the {} section!", "-h".bright_blue().bold(), "help".white().bold());
}

fn help()
{
    println!("\n| {} {} |", "WELCOME TO", "AION".purple().bold());
    println!("{}", "-------------------".bright_black());
    println!("You are in the {} section!\n", "help".white().bold());
    println!("    {}", "Build and Run".white().bold());
    println!("    If none of those commands are specified, it's considered a {} {} by default!\n", "debug".white().bold(), "build".white().bold());
    println!("\t{}: It {} and {} the program after build", "-r".bright_blue().bold(), "Builds".white().bold(), "runs".white().bold());

    print!("\n");
    println!("    {}", "Target".white().bold());
    println!("    As said above, Nyon compiles for a {} target by default.\n    Note that, in the {} compilation, the generated C code is automatically removed, unlike other target.\n", "debug".white().bold(), "release".white().bold());
    println!("\t{}: The program is for a {} target", "-rel".bright_blue().bold(), "release".white().bold());
    println!("\t{}: It keeps the generated C code (generally in the release target to check the generated code)", "-kc".bright_blue().bold());

    print!("\n");
    println!("    {}", "Clean".white().bold());
    println!("    Do you want to clean the output folder? You are in the right place!\n");
    println!("\t{}: It completely removes all the files in the '{}' folder", "-c".bright_blue().bold(), "out".white().bold());
    println!("\t{}: It stands for \"Clear All\". It also removes the '{}' folder", "-ca".bright_blue().bold(), "out".white().bold());

    print!("\n");
    println!("    {}", "Back-end".white().bold());
    println!("    Those commands are used to choose the back-end to use to generate and compile the code (GCC by default).\n");
    println!("\t{}: It generates {} code and uses {} to compile it", "-zig".bright_blue().bold(), "C".white().bold(), "Zig".white().bold());
    // println!("\t{}: It generates and compile code with {}.", "-llvm".bright_blue().bold(), "LLVM".white().bold());
}

fn clear(all: bool)
{
    if all {
        println!("\n{} '{}'...", "Removing".green().bold(), "out".white().bold());
        let _ = std::fs::remove_dir_all("out");
    }
    else {
        println!("\n{} '{}'...", "Cleaning".green().bold(), "out".white().bold());
        let _ = std::fs::remove_dir_all("out");
        let _ = std::fs::create_dir("out");
    }
}

pub fn lex(input: &str) -> lexer::Lexer
{
    let mut l = lexer::Lexer::new(input);
    l.process();
    l
}

pub fn parse(tokens: &mut Vec<lexer::Token>) -> parser::Parser
{
    let mut p = parser::Parser::new();
    p.process(tokens);
    p
}

fn resolve(prog: &ast::Program, dirname: &str) -> resolver::Resolver
{
    let mut r = resolver::Resolver::new();
    r.process(prog, &dirname);
    r
}

fn typecheck(prog: &mut ast::Program, mut symbols: &mut SymbolTable) -> typechecker::Checker
{
    let mut tc = typechecker::Checker::new();
    tc.process(prog, &mut symbols);
    tc
}

fn compile(c_src: &str, filename: &str, ty: &BackendType) -> std::io::Result<std::process::Output>
{
    match ty {
        BackendType::GCC => std::process::Command::new("gcc")
                                .arg(c_src).arg("-o")
                                .arg(format!("out/{}.exe", filename))
                                .arg("-fopenmp")
                                .output(),
        BackendType::ZIG => std::process::Command::new("zig")
                                .arg("cc").arg(c_src).arg("-o")
                                .arg(format!("out/{}.exe", filename))
                                .output(),
        BackendType::LLVM => { // this doesn't work: not yet implemented!
            eprintln!("warning: LLVM backend is not yet implemented, falling back to GCC");
            std::process::Command::new("gcc")
                .arg(c_src).arg("-o")
                .arg(format!("out/{}.exe", filename))
                .arg("-fopenmp")
                .output()
        }
    }
}

fn build(input: String, filename: &str, dirname: &str, ty: BackendType, target: Target, flag: BuildFlag)
{
    let mut l = lex(input.as_str());
    l.reporter().fire_all();
    if l.reporter().has_errors() { std::process::exit(1); }

    let mut p = parse(l.tokens_mut());
    p.reporter().fire_all();
    if p.reporter().has_errors() { std::process::exit(1); }

    let mut r = resolve(p.output(), &dirname);
    r.reporter().fire_all();
    if r.reporter().has_errors() { std::process::exit(1); }

    let mut tc = typecheck(p.output_mut(), r.output());
    tc.reporter().fire_all();
    if tc.reporter().has_errors() { std::process::exit(1); }

    let mut cg = backend::c::CGenerator::new();
    cg.process(p.output(), r.output());
    cg.reporter().fire_all();
    if cg.reporter().has_errors() { std::process::exit(1); }

    let _ = std::fs::create_dir("out");
    let mut f = match File::create("out/out.c") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: Could not create output file: {}", e);
            std::process::exit(1);
        }
    };
    let _ = f.write_all(cg.output().as_bytes());

    let output = compile("out/out.c", filename, &ty);

    if target == Target::Release && flag != BuildFlag::KeepCode
    {
        let _ = std::fs::remove_file("out/out.c");
    }

    let status = match output {
        Ok(_)  => "[SUCCESS]".green().bold(),
        Err(_) => "[FAILURE]".red().bold(),
    };

    println!("Compiled with status:\t\t{}", status);
}

fn run(filename: &str)
{
    let output = std::process::Command::new(format!("out/{}.exe", filename)).output();

    match output {
        Ok(out) => {
            println!("\n{} and {} '{}'", "Building".green().bold(), "running".green().bold(), filename.white().bold());
            println!("{}", String::from_utf8(out.stdout).unwrap_or_default());
        }
        Err(err) => {
            println!("\n{} and {} '{}'", "Building".green().bold(), "running".green().bold(), filename.white().bold());
            println!("{}", err.kind());
        }
    }
}

fn main()
{
    let args: Vec<String> = std::env::args().collect();
    let arg_set: HashSet<&str> = args.iter().map(String::as_str).collect();

    let input = args.iter().find(|s| s.contains(".nn")).cloned().unwrap_or_default();

    let mut ty     = BackendType::GCC;
    let mut target = Target::Debug;
    let mut flag   = BuildFlag::RemoveCode;

    if arg_set.contains("-h") || arg_set.contains("-help")
    {
        help();
        return;
    }
    else if arg_set.contains("-v") || arg_set.contains("-version") || args.len() < 2
    {
        info();
        return;
    }
    else {
        let mut filename = path::Path::new(input.as_str())
            .file_name().unwrap_or_default()
            .to_str().unwrap_or_default()
            .to_string();
        let dirname = {
            let p = path::Path::new(input.as_str())
                .parent()
                .expect("Unable to retreive the dir from input!")
                .to_str()
                .unwrap_or_default();
            if p.is_empty() { "." } else { p }
        };
        let to_rem = filename.find('.').unwrap_or(filename.len());
        filename = filename.drain(..to_rem).collect();

        if arg_set.contains("-zig") {
            ty = BackendType::ZIG;
        }
        // else if arg_set.contains("-llvm") {
        //     ty = BackendType::LLVM;
        // }

        if arg_set.contains("-rel")  { target = Target::Release; }
        if arg_set.contains("-kc")   { flag   = BuildFlag::KeepCode; }

        if arg_set.contains("-ca") {
            clear(true);
        } else if arg_set.contains("-c") {
            clear(false);
        }

        build(input.clone(), &filename, &dirname, ty, target, flag);

        if arg_set.contains("-r") {
            run(&filename);
        }
    }
}
