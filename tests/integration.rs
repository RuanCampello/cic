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
    assert_eq!(stdout(&output), "2657\n");
}

#[test]
fn lex_ok_program() {
    let output = cic("lex", "ok.ci");
    assert!(output.status.success());
    assert_eq!(stdout(&output).lines().next(), Some("<OpenParen, '(', 0>"));
    assert_eq!(stdout(&output).lines().count(), 15);
}

#[test]
fn parse_ok_program() {
    let output = cic("parse", "ok.ci");
    assert!(output.status.success());
    assert!(stdout(&output).lines().next().is_some_and(|root| root.trim() == "+"));
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
fn stdout(output: &process::Output) -> &str {
    str::from_utf8(&output.stdout).expect("stdout to be utf-8")
}

#[inline]
fn stderr(output: &process::Output) -> &str {
    str::from_utf8(&output.stderr).expect("stderr to be utf-8")
}
