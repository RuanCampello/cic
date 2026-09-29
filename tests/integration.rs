use std::process;

fn cic(subcommand: &str, program: &str) -> process::Output {
    process::Command::new(env!("CARGO_BIN_EXE_cic"))
        .args([subcommand, &format!("tests/fixtures/{program}")])
        .output()
        .expect("cic binary to be built")
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
