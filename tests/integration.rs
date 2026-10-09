use std::{
    fs,
    process::{self, Command},
};

fn compile_and_run(name: &str, program: &str) -> String {
    let dir = std::env::temp_dir().join(format!("cic-{name}-{}", process::id()));

    fs::create_dir_all(&dir).expect("temp dir to be creatable");
    fs::copy("asm/runtime.s", dir.join("runtime.s")).expect("asm/runtime to exist");
    fs::write(dir.join("p.ci"), program).expect("program to be executable");

    let steps: [&[&str]; _] = [
        &[env!("CARGO_BIN_EXE_cic"), "build", "p.ci"],
        &["as", "--64", "-o", "p.o", "p.s"],
        &["ld", "-o", "p", "p.o"],
    ];

    for step in steps {
        let (cmd, args) = step.split_first().expect("step to have a cmd");

        let status = Command::new(cmd)
            .args(args)
            .current_dir(&dir)
            .status()
            .unwrap_or_else(|err| panic!("couldn't run {cmd}: {err}"));

        assert!(status.success(), "{cmd} failed for {program}");
    }

    let output = Command::new(dir.join("p")).output().expect("compiled program to run");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("stdout to be utf8")
}

fn cic(subcommand: &str, program: &str) -> process::Output {
    Command::new(env!("CARGO_BIN_EXE_cic"))
        .args([subcommand, &format!("tests/fixtures/{program}")])
        .output()
        .expect("cic binary to be built")
}

#[test]
fn compiled_programs_print_their_value() {
    let cases = [
        ("constant", "333", "333"),
        ("mul", "(6 * 7)", "42"),
        ("nested", "(3 + (4 + (11 + 7)))", "25"),
        ("spec", "((427 / 7) + (11 * (231 + 5)))", "2657"),
        ("operand_order", "(100 - (20 /3))", "94"),
        ("negative", "(3 - 10)", "-7"),
        ("i64_max", "9223372036854775807", "9223372036854775807"),
    ];

    for (name, program, expected) in cases {
        assert_eq!(compile_and_run(name, program), format!("{expected}\n"), "{program}");
    }
}

#[test]
fn eval_ok_program() {
    let output = cic("eval", "ok.ci");
    assert!(output.status.success());
    assert_eq!(stdout(&output), ["2657"]);
}

#[test]
fn lex_ok_program() {
    let output = cic("lex", "ok.ci");
    assert!(output.status.success());

    let expected = [
        "<OpenParen, '(', 0>",
        "<OpenParen, '(', 1>",
        "<Integer, '427', 2>",
        "<Slash, '/', 6>",
        "<Integer, '7', 8>",
        "<CloseParen, ')', 9>",
        "<Plus, '+', 11>",
        "<OpenParen, '(', 13>",
        "<Integer, '11', 14>",
        "<Star, '*', 17>",
        "<OpenParen, '(', 19>",
        "<Integer, '231', 20>",
        "<Plus, '+', 24>",
        "<Integer, '5', 26>",
        "<CloseParen, ')', 27>",
        "<CloseParen, ')', 28>",
        "<CloseParen, ')', 29>",
    ];

    assert_eq!(stdout(&output), expected);
}

#[test]
fn parse_ok_program() {
    let output = cic("parse", "ok.ci");
    assert!(output.status.success());

    let expected = [
        r"     __+__",
        r"    /     \",
        r"   /      _*_",
        r"  / \    /   \",
        r"427  7  11    +",
        r"             / \",
        r"           231  5",
    ];
    assert_eq!(stdout(&output), expected)
}

#[test]
fn lexical_error_fails() {
    let output = cic("parse", "lex_error.ci");
    assert!(!output.status.success());
    assert!(stderr(&output).starts_with("lexical error at 5"));
}

#[test]
fn syntax_error_fails() {
    let output = cic("parse", "syntax_error.ci");
    assert!(!output.status.success());
    assert!(stderr(&output).starts_with("parsing error at 3"));
}

#[inline]
fn stdout(output: &process::Output) -> Vec<&str> {
    str::from_utf8(&output.stdout).expect("stdout to be utf-8").lines().collect()
}

#[inline]
fn stderr(output: &process::Output) -> &str {
    str::from_utf8(&output.stderr).expect("stderr to be utf-8")
}
