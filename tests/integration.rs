use cic::{codegen, frontend::parser, semantic, toolchain};
use std::{
    fs,
    process::{self, Command},
};

fn compile_and_run(name: &str, program: &str) -> String {
    let dir = std::env::temp_dir().join(format!("cic-{name}-{}", process::id()));
    let program = parser::parse(program).expect("test program to be valid");
    let symbols = semantic::analyse(&program).expect("test program to declare its variables");
    let asm = codegen::generate(&program, &symbols);

    let output = Command::new(toolchain::link(&asm, &dir)).output().expect("program to run");
    fs::remove_dir_all(&dir).expect("temp dir to be removable");

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
        ("constant", "= 333", "333"),
        ("mul", "= (6 * 7)", "42"),
        ("nested", "= (3 + (4 + (11 + 7)))", "25"),
        ("spec", "= ((427 / 7) + (11 * (231 + 5)))", "2657"),
        ("operand_order", "= (100 - (20 /3))", "94"),
        ("negative", "= (3 - 10)", "-7"),
        ("i64_max", "= 9223372036854775807", "9223372036854775807"),
        ("precedence", "= 2 + 3 * 4 - 10 / 2", "9"),
        ("left_assoc", "= 100 - 20 - 5", "75"),
        ("perimeter", "l = 30;\nc = 40;\n= l + l + c + c", "140"),
        ("chained", "x = 11 * 7;\ny = x * 3 + 5;\n= x * y", "18172"),
        ("redeclared", "x = 2;\nx = x * 21;\n= x", "42"),
        ("long_names", "Width1 = 6;\nheightZ = 7;\n= Width1 * heightZ", "42"),
    ];

    for (name, program, expected) in cases {
        assert_eq!(compile_and_run(name, program), format!("{expected}\n"), "{program}");
    }
}

#[test]
fn run_ok_program() {
    let output = cic("run", "ok.ci");
    assert!(output.status.success());
    assert_eq!(stdout(&output), ["60467"]);
}

#[test]
fn eval_ok_program() {
    let output = cic("eval", "ok.ci");
    assert!(output.status.success());
    assert_eq!(stdout(&output), ["60467"]);
}

#[test]
fn lex_ok_program() {
    let output = cic("lex", "ok.ci");
    assert!(output.status.success());

    let expected = [
        "<Ident, 'x', 0>",
        "<Equal, '=', 2>",
        "<OpenParen, '(', 4>",
        "<Integer, '7', 5>",
        "<Plus, '+', 7>",
        "<Integer, '4', 9>",
        "<CloseParen, ')', 10>",
        "<Star, '*', 12>",
        "<Integer, '12', 14>",
        "<Semicolon, ';', 16>",
        "<Ident, 'y', 18>",
        "<Equal, '=', 20>",
        "<Ident, 'x', 22>",
        "<Star, '*', 24>",
        "<Integer, '3', 26>",
        "<Plus, '+', 28>",
        "<Integer, '11', 30>",
        "<Semicolon, ';', 32>",
        "<Equal, '=', 34>",
        "<OpenParen, '(', 36>",
        "<Ident, 'x', 37>",
        "<Star, '*', 39>",
        "<Ident, 'y', 41>",
        "<CloseParen, ')', 42>",
        "<Plus, '+', 44>",
        "<OpenParen, '(', 46>",
        "<Ident, 'x', 47>",
        "<Star, '*', 49>",
        "<Integer, '11', 51>",
        "<CloseParen, ')', 53>",
        "<Plus, '+', 55>",
        "<OpenParen, '(', 57>",
        "<Ident, 'y', 58>",
        "<Star, '*', 60>",
        "<Integer, '13', 62>",
        "<CloseParen, ')', 64>",
    ];

    assert_eq!(stdout(&output), expected);
}

#[test]
fn parse_ok_program() {
    let output = cic("parse", "ok.ci");
    assert!(output.status.success());

    let expected = [
        r"  _=_",
        r" /   \",
        r"x     *",
        r"     / \",
        r"    +   12",
        r"   / \",
        r"  7   4",
        r"",
        r"  _=_",
        r" /   \",
        r"y     +",
        r"     / \",
        r"    *   11",
        r"   / \",
        r"  x   3",
        r"",
        r"       ___+___",
        r"      /       \",
        r"    _+_        *",
        r"   /   \      / \",
        r"  *     *    y   13",
        r" / \   / \",
        r"x   y x   11",
    ];
    assert_eq!(stdout(&output), expected)
}

#[test]
fn lexical_error_fails() {
    let output = cic("parse", "lex_error.ci");
    assert!(!output.status.success());
    assert!(stderr(&output).starts_with("lexical error at 4"));
}

#[test]
fn syntax_error_fails() {
    let output = cic("parse", "syntax_error.ci");
    assert!(!output.status.success());
    assert!(stderr(&output).starts_with("parsing error at 7"));
}

#[test]
fn undeclared_variable_fails() {
    for subcommand in ["eval", "run", "build"] {
        let output = cic(subcommand, "undeclared.ci");
        assert!(!output.status.success(), "{subcommand}");
        assert_eq!(stderr(&output), "semantic error at 8: use of undeclared variable `y`\n");
    }
}

#[test]
fn generated_assembly_declares_variables_in_bss() {
    let program = parser::parse("x = 11 * 7;\ny = x * 3 + 5;\n= x * y").unwrap();
    let symbols = semantic::analyse(&program).unwrap();
    let asm = codegen::generate(&program, &symbols);

    let bss = asm.find(".section .bss").expect("a bss section");
    let text = asm.find(".section .text").expect("a text section");
    let x = asm.find(".lcomm var_x, 8").expect("x to be declared");
    let y = asm.find(".lcomm var_y, 8").expect("y to be declared");
    assert!(bss < x && x < text && bss < y && y < text);
    assert!(asm.contains("mov %rax, var_x") && asm.contains("mov var_x, %rax"));
}

#[inline]
fn stdout(output: &process::Output) -> Vec<&str> {
    str::from_utf8(&output.stdout).expect("stdout to be utf-8").lines().collect()
}

#[inline]
fn stderr(output: &process::Output) -> &str {
    str::from_utf8(&output.stderr).expect("stderr to be utf-8")
}
