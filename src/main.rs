use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf};

use cic::{
    self,
    frontend::{
        lexer::LexError,
        parser::{self, ParseError},
    },
    interpreter::{self, EvalError},
};

#[derive(Parser)]
#[command(name = "cic", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compiles a ci file to a native x86-64 executable
    Build { path: PathBuf },
    /// Parses and print the syntax tree for an ci file
    Parse { path: PathBuf },
    /// Evaluate and print a ci program
    Eval { path: PathBuf },
}

#[derive(Debug)]
enum Error<'err> {
    Lex(LexError<'err>),
    Parse(ParseError<'err>),
    Eval(EvalError),
}

impl Command {
    const fn path(&self) -> &PathBuf {
        match self {
            Self::Build { path } | Self::Parse { path } | Self::Eval { path } => path,
        }
    }

    fn run<'src>(&self, src: &'src str) -> Result<String, Error<'src>> {
        match self {
            Self::Build { .. } => unreachable!("build was not implemented for this delivery"),
            Self::Parse { .. } => Ok(parser::parse(src)?.tree().to_string()),
            Self::Eval { .. } => Ok(format!("{}", interpreter::evaluate(&parser::parse(src)?)?)),
        }
    }
}

fn main() {
    let command = Cli::parse().command;
    let path = command.path();

    let Ok(src) = fs::read_to_string(path) else {
        eprintln!("couldn't read {}", path.display());
        std::process::exit(1)
    };

    match command.run(&src) {
        Ok(output) => {
            println!("{output}");
        },
        Err(error) => {
            eprintln!("{error}")
        },
    }
}

impl std::fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lex(lex) => write!(f, "{lex}"),
            Self::Parse(parse) => write!(f, "{parse}"),
            Self::Eval(eval) => write!(f, "{eval}"),
        }
    }
}

impl<'src> From<LexError<'src>> for Error<'src> {
    fn from(error: LexError<'src>) -> Self {
        Self::Lex(error)
    }
}

impl<'src> From<ParseError<'src>> for Error<'src> {
    fn from(error: ParseError<'src>) -> Self {
        Self::Parse(error)
    }
}

impl From<EvalError> for Error<'_> {
    fn from(error: EvalError) -> Self {
        Self::Eval(error)
    }
}
