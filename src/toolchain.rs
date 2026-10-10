//! Assembling and linking of the generated code

use crate::codegen;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// assembles and links `asm` inside `dir`, returning the executable's path
pub fn link(asm: &str, dir: impl AsRef<Path>) -> PathBuf {
    let dir = dir.as_ref();
    fs::create_dir_all(dir).expect("output dir to be creatable");
    fs::write(dir.join("runtime.s"), codegen::RUNTIME).expect("runtime.s to be writable");
    fs::write(dir.join("p.s"), asm).expect("p.s to be writable");

    let steps: [&[&str]; 2] = [&["as", "--64", "-o", "p.o", "p.s"], &["ld", "-o", "p", "p.o"]];

    for step in steps {
        let (cmd, args) = step.split_first().expect("step to have a command");
        let status = Command::new(cmd)
            .args(args)
            .current_dir(dir)
            .status()
            .unwrap_or_else(|err| panic!("couldn't run {cmd}: {err}"));

        assert!(status.success(), "{cmd} rejected the generated code");
    }

    dir.join("p")
}
