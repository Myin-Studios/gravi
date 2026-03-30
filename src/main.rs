use std::{fs::File, io::Write, path};

use colored::Colorize;

pub mod lexer;
pub mod parser;
pub mod codegen;
pub mod backend;

use backend::Backend;
use codegen::{BackendType, BuildFlag, Generators, Target};

fn info()
{
    println!("\n{} - version: {}", "Nyon".purple().bold(), "alpha-0.1".white().bold());
    println!("Copyright © {} {}\n", "Myin Studios".bright_blue().bold(), "2026".white().bold());
    println!("Type {} to open the {} section!", "-h".blue().bold(), "help".white().bold());
}

fn help()
{
    println!("\n| {} {} |", "WELCOME TO", "NYON".purple().bold());
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
    println!("\t{}: It generates and compile code with {}.", "-llvm".bright_blue().bold(), "LLVM".white().bold());
}

fn clear(all: bool)
{
    if all {
        println!("\n{} '{}'...", "Removing".green().bold(), "out".white().bold());
        let _ = std::fs::remove_dir_all("out");
    }
    else
    {
        println!("\n{} '{}'...", "Cleaning".green().bold(), "out".white().bold());
        let _ = std::fs::remove_dir_all("out");
        let _ = std::fs::create_dir("out");
    }
}

fn build(input: String, filename: &String, ty: BackendType, target: Target, flag: BuildFlag)
{
    let mut l = lexer::Lexer::new(input.as_str());
    l.process();

    let mut p = parser::Parser::new();
    p.process(l.tokens_mut());

    let cg = match ty {
        BackendType::ZIG | BackendType::GCC | BackendType::LLVM /* to be moved in another branch!*/ => {
            Generators::C(backend::c::CGenerator::new())
        }
    };

    let output = match cg {
        Generators::C(mut g) => {
            g.process(p.output());

            let mut f = File::create("out/out.c").unwrap();
            let _ = f.write_all(g.output().as_bytes()).unwrap();

            match ty {
                BackendType::GCC => std::process::Command::new("gcc")
                                            .arg("out/out.c")
                                            .arg("-o")
                                            .arg(format!("out/{}.exe", filename))
                                            .arg("-fopenmp")
                                            .output(),
                BackendType::ZIG => std::process::Command::new("zig")
                                            .arg("cc")
                                            .arg("out/out.c")
                                            .arg("-o")
                                            .arg(format!("out/{}.exe", filename))
                                            .output(),
                BackendType::LLVM => std::process::Command::new("gcc") // it uses GCC by default until LLVM is implemented  
                                            .arg("out/out.c")
                                            .arg("-o")
                                            .arg(format!("out/{}.exe", filename))
                                            .arg("-fopenmp")
                                            .output(),
            }
        }
    };

    if target == Target::Release
    {
        if flag != BuildFlag::KeepCode
        {
            let _ = std::fs::remove_file("out/out.c");
        }
    }

    let status = match output {
        Ok(_) => "[SUCCESS]".green().bold(),
        Err(_) => "[FAILURE]".red().bold(),
    };

    println!("Compiled with status:\t\t{}", status);
}

fn run(filename: &String)
{
    let output = std::process::Command::new(format!("out/{}.exe", filename).as_str()).output();
    
    match output {
        Ok(out) =>
        {
            println!("\n{} and {} '{}'", "Building".green().bold(), "running".green().bold(), filename.white().bold());
            println!("{}", String::from_utf8(out.stdout).unwrap_or_default());
        }
        Err(err) =>
        {
            println!("\n{} and {} '{}'", "Building".green().bold(), "running".green().bold(), filename.white().bold());
            println!("{}", err.kind());
        }
    }
}

fn main()
{
    let args: Vec<String> = std::env::args().collect();
    let input = args.iter().find(|s| s.contains(".nn")).unwrap_or(&"".to_string()).to_string();
    
    let mut ty = BackendType::GCC;
    let mut target = Target::Debug;
    let mut flag = BuildFlag::RemoveCode;

    if args.contains(&"-h".to_string()) || args.contains(&"-help".to_string())
    {
        help();

        return;
    }
    else if args.contains(&"-v".to_string()) || args.contains(&"-version".to_string()) || args.len() < 2 {
        info();

        return;
    }
    else {
        let mut filename = path::Path::new(input.as_str()).file_name().unwrap_or_default().to_str().unwrap_or_default().to_string();
        let to_rem = filename.find('.').unwrap_or(filename.len());
        filename = filename.to_string().drain(..to_rem).collect();

        if args.contains(&"-zig".to_string()) {
            ty = BackendType::ZIG;
        }
        else if args.contains(&"-llvm".to_string()){
            ty = BackendType::LLVM;
        }

        if args.contains(&"-rel".to_string())
        {
            target = Target::Release;
        }

        if args.contains(&"-kc".to_string())
        {
            flag = BuildFlag::KeepCode;
        }

        if args.contains(&"-c".to_string())
        {
            clear(false);
        }
        else if args.contains(&"-ca".to_string()) {
            clear(true);
        }

        build(input, &filename, ty, target, flag);

        if args.contains(&"-r".to_string())
        {
            run(&filename);
        }
    }
}