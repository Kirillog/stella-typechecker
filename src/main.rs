use std::io::Read;
use stella_typechecker::{parser, typechecker};

fn main() {
    let src = match std::env::args().nth(1).as_deref() {
        None | Some("-") => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .unwrap_or_else(|e| {
                    eprintln!("Cannot read stdin: {e}");
                    std::process::exit(1);
                });
            buf
        }
        Some(path) => std::fs::read_to_string(path).unwrap_or_else(|e| {
            eprintln!("Cannot read {path}: {e}");
            std::process::exit(1);
        }),
    };

    let program = match parser::ProgramParser::new().parse(&src) {
        Ok(program) => program,
        Err(e) => {
            eprintln!("Parse error: {e}");
            std::process::exit(1);
        }
    };

    let errors = typechecker::TypeChecker::new().check_program(&program, &src);
    if errors.is_empty() {
        println!("Type OK");
    } else {
        eprintln!("{}", errors[0]);
        std::process::exit(2);
    }
}
