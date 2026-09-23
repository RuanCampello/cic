use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
            Self::Parse { .. } => Ok(todo!()),
            Self::Eval { .. } => Ok(format!("{}", interpreter::evaluate(&parser::parse(src)?)?)),
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { path } => unreachable!("build was not done for this phase"),
        Command::Parse { path } => todo!(),
        Command::Eval { path } => todo!(),
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
