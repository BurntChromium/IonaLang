//! Command line interface for the compiler

use std::error::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Build,
    Check,
    Test,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    StdLib,
    Entrypoint(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flags {
    SingleFile,
    Verbose,
}

pub struct Command {
    pub mode: Mode,
    pub target: Target,
    pub flags: Vec<Flags>,
}

pub fn parse_args(args: &Vec<String>) -> Result<Command, Box<dyn Error>> {
    if args.len() < 2 {
        return Err("you must pass at least 1 argument to the compiler".into());
    }
    let mode: Mode;
    match args[1].as_str() {
        "build" => mode = Mode::Build,
        "check" => mode = Mode::Check,
        "test" => mode = Mode::Test,
        _ => unreachable!("compiler must be invoked in 'build', 'check', or 'test' mode"),
    }
    if args.len() >= 2 {
        let mut flags: Vec<Flags> = Vec::new();
        let mut maybe_target: Option<Target> = None;
        for arg in args.iter().skip(1) {
            if arg.starts_with("-") {
                flags.push(match arg.as_str() {
                    "-v" => Flags::Verbose,
                    "--verbose" => Flags::Verbose,
                    "-f" => Flags::SingleFile,
                    "--file" => Flags::SingleFile,
                    _ => unreachable!("the only supported compiler flags are -v and -f"),
                });
            } else {
                if arg.ends_with(".iona") {
                    maybe_target = Some(Target::Entrypoint(arg.clone()));
                } else if arg == "stdlib" {
                    maybe_target = Some(Target::StdLib);
                }
            }
        }
        return Ok(Command {
            mode,
            target: maybe_target.unwrap_or(Target::Entrypoint("main.iona".to_string())),
            flags,
        });
    } else {
        let target: Target = Target::Entrypoint("main.iona".to_string());
        return Ok(Command {
            mode,
            target,
            flags: Vec::new(),
        });
    }
}
