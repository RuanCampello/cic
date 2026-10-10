use clap::{Parser, Subcommand};
use std::{fs, os::unix::process::ExitStatusExt, path::PathBuf, process};

use cic::{
    self, codegen,
    frontend::{
        lexer::{self, LexError, Spanned},
        parser::{self, ParseError, ParseErrorKind},
    },
    interpreter::{self, EvalError},
    toolchain,
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
    Build {
        path: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Compiles a ci file and runs it natively
    Run { path: PathBuf },
    /// Parses and print the syntax tree for a ci file
    Parse { path: PathBuf },
    /// Evaluate and print a ci program
    Eval { path: PathBuf },
    /// Lexes and prints the token sequence of a ci file
    Lex { path: PathBuf },
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
            Self::Build { path, .. }
            | Self::Parse { path }
            | Self::Eval { path }
            | Self::Run { path }
            | Self::Lex { path } => path,
        }
    }

    fn run<'src>(&self, src: &'src str) -> Result<String, Error<'src>> {
        match self {
            Self::Build { .. } | Self::Run { .. } => Ok(codegen::generate(&parser::parse(src)?)),
            Self::Parse { .. } => Ok(parser::parse(src)?.tree().to_string()),
            Self::Eval { .. } => Ok(format!("{}\n", interpreter::evaluate(&parser::parse(src)?)?)),
            Self::Lex { .. } => {
                Ok(lexer::tokenise(src)?.iter().map(|token| format!("{token}\n")).collect())
            },
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

    let result = command.run(&src).unwrap_or_else(|err| {
        eprintln!("{err}");
        std::process::exit(1);
    });

    match command {
        Command::Build { path, output } => {
            let dest = output.clone().unwrap_or_else(|| path.with_extension("s"));
            if let Err(error) = fs::write(&dest, result) {
                eprintln!("couldn't write {}: {error}", dest.display());
                std::process::exit(1);
            }
        },
        Command::Run { .. } => process::exit(execute(&result)),
        _ => print!("{result}"),
    }
}

fn execute(asm: &str) -> i32 {
    let dir = std::env::temp_dir().join(format!("cic-run-{}", process::id()));
    let binary = toolchain::link(asm, &dir);

    let status = process::Command::new(binary).status().expect("linked program to run");
    fs::remove_dir_all(dir).expect("temp dir to be removable");

    match (status.code(), status.signal()) {
        (Some(code), _) => code,
        (_, Some(signal)) => {
            eprintln!("program killed by signal: {signal}");
            128 + signal
        },
        _ => unreachable!("a process must end with a exit code or signal"),
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
        match error.kind {
            ParseErrorKind::Lex(kind) => Self::Lex(Spanned::new(kind, error.span)),
            _ => Self::Parse(error),
        }
    }
}

impl From<EvalError> for Error<'_> {
    fn from(error: EvalError) -> Self {
        Self::Eval(error)
    }
}
