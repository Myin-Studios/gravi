use std::{fs::File, io::Write, path};

use colored::Colorize;

pub mod lexer;
pub mod parser;
pub mod codegen;

fn main()
{
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"-h".to_string()) || args.contains(&"-help".to_string())
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

        return;
    }
    else if args.contains(&"-v".to_string()) || args.contains(&"-version".to_string()) || args.len() < 2 {
        println!("\n{} - version: {}", "Nyon".purple().bold(), "alpha-0.1".white().bold());
        println!("Copyright © {} {}\n", "Myin Studios".bright_blue().bold(), "2026".white().bold());
        println!("Type {} to open the {} section!", "-h".blue().bold(), "help".white().bold());

        return;
    }
    else {
        if args.contains(&"-c".to_string())
        {
            println!("\n{} '{}'...", "Cleaning".green().bold(), "out".white().bold());
            let _ = std::fs::remove_dir_all("out");
            let _ = std::fs::create_dir("out");
        }
        else if args.contains(&"-ca".to_string()) {
            println!("\n{} '{}'...", "Removing".green().bold(), "out".white().bold());
            let _ = std::fs::remove_dir_all("out");
        }

        let input = args.iter().find(|s| s.contains(".nn")).unwrap_or(&"".to_string()).to_string();

        let mut l = lexer::Lexer::new(input.as_str());
        l.process();

        let mut p = parser::Parser::new();
        p.process(l.tokens_mut());

        let mut cg = codegen::Generator::new();
        cg.process(p.output());

        let _ = std::fs::create_dir("out");

        let mut f = File::create("out/out.c").unwrap();
        let _ = f.write_all(cg.output().as_bytes()).unwrap();

        let mut filename = path::Path::new(input.as_str()).file_name().unwrap_or_default().to_str().unwrap_or_default().to_string();
        let to_rem = filename.find('.').unwrap_or(filename.len());
        filename = filename.to_string().drain(..to_rem).collect();

        if args.contains(&"-rel".to_string())
        {
            let _ = std::process::Command::new("gcc").arg("out/out.c").arg("-o").arg(format!("out/{}.exe", filename)).arg("-fopenmp").arg("-O2").output();
            
            if !args.contains(&"-kc".to_string())
            {
                let _ = std::fs::remove_file("out/out.c");
            }
        }
        else {
            let _ = std::process::Command::new("gcc").arg("out/out.c").arg("-o").arg("-fopenmp").arg(format!("out/{}.exe", filename)).output();
        }

        if args.contains(&"-r".to_string())
        {
            println!("\n{} and {} '{}'", "Building".green().bold(), "running".green().bold(), filename.white().bold());
            let _ = std::process::Command::new("out/test.exe");
        }
    }
}