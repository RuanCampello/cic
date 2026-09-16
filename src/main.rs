use clap::{Parser, Subcommand};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use cic;

#[derive(Parser)]
#[command(name = "cic", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compiles a ci file to a native x86-64 executable
    Build { path: PathBuf },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { path } => build(path),
    }
}

fn build(path: PathBuf) {
    let src = fs::read_to_string(&path).unwrap();
    let assembly = cic::compile(&src);

    let stem = path.file_stem().expect("file must have a name").to_str().unwrap();
    let dir = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    let asm = dir.join(format!("{stem}.s"));
    let obj = dir.join(format!("{stem}.o"));
    let exe = dir.join(stem);
    let runtime = dir.join("runtime.s");

    fs::write(&asm, assembly).expect("failed to write assembly file");
    fs::write(&runtime, cic::resources::RUNTIME).expect("failed to write runtime file");

    let as_status = Command::new("as")
        .args(["--64", "-o"])
        .arg(&obj)
        .arg(&asm)
        .status()
        .expect("failed to execute 'as'");
    assert!(as_status.success(), "assembler failed: {as_status}");

    let ld_status = Command::new("ld")
        .arg("--o")
        .arg(&exe)
        .arg(&obj)
        .status()
        .expect("failed to execute 'ld'");

    assert!(ld_status.success(), "linker failed: {ld_status}")
}
