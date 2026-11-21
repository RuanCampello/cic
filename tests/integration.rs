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
        ("constant", "{ return 333; }", "333"),
        ("mul", "{ return (6 * 7); }", "42"),
        ("nested", "{ return (3 + (4 + (11 + 7))); }", "25"),
        ("spec", "{ return ((427 / 7) + (11 * (231 + 5))); }", "2657"),
        ("operand_order", "{ return (100 - (20 /3)); }", "94"),
        ("negative", "{ return (3 - 10); }", "-7"),
        ("i64_max", "{ return 9223372036854775807; }", "9223372036854775807"),
        ("precedence", "{ return 2 + 3 * 4 - 10 / 2; }", "9"),
        ("left_assoc", "{ return 100 - 20 - 5; }", "75"),
        ("perimeter", "l = 30;\nc = 40;\n{ return l + l + c + c; }", "140"),
        ("chained", "x = 11 * 7;\ny = x * 3 + 5;\n{ return x * y; }", "18172"),
        ("redeclared", "x = 2;\nx = x * 21;\n{ return x; }", "42"),
        ("long_names", "Width1 = 6;\nheightZ = 7;\n{ return Width1 * heightZ; }", "42"),
        ("less", "{ return 3 < 5; }", "1"),
        ("not_less", "{ return 5 < 3; }", "0"),
        ("greater", "{ return 5 > 3; }", "1"),
        ("not_greater", "{ return 3 > 3; }", "0"),
        ("equal", "{ return 4 == 4; }", "1"),
        ("not_equal", "{ return 4 == 5; }", "0"),
        ("negative_less", "{ return 0 - 5 < 0; }", "1"),
        ("comparison_precedence", "{ return 1 + 2 == 9 / 3; }", "1"),
        ("assign", "x = 1;\n{ x = x + 41; return x; }", "42"),
        ("if_then", "x = 0;\n{ if 1 { x = 1; } else { x = 2; } return x; }", "1"),
        ("if_else", "x = 0;\n{ if 0 { x = 1; } else { x = 2; } return x; }", "2"),
        ("empty_arms", "x = 7;\n{ if x { } else { } return x; }", "7"),
        ("while_skipped", "x = 3;\n{ while 0 { x = 0; } return x; }", "3"),
        (
            "nested_commands",
            "i = 0;\nevens = 0;\n{ while i < 10 { if i == i / 2 * 2 { evens = evens + 1; } else { } i = i + 1; } return evens; }",
            "5",
        ),
    ];

    for (name, program, expected) in cases {
        assert_eq!(compile_and_run(name, program), format!("{expected}\n"), "{program}");
    }
}

#[test]
fn fixtures_evaluate_and_run_to_their_value() {
    let fixtures = [
        ("ok.ci", "11"),
        ("minimal.ci", "42"),
        ("delta.ci", "8"),
        ("sum.ci", "45"),
        ("remainder.ci", "2"),
        ("gcd.ci", "6"),
    ];

    for (fixture, expected) in fixtures {
        for subcommand in ["eval", "run"] {
            let output = cic(subcommand, fixture);
            assert!(output.status.success(), "{subcommand} {fixture}: {}", stderr(&output));
            assert_eq!(stdout(&output), [expected], "{subcommand} {fixture}");
        }
    }
}

#[test]
fn lex_ok_program() {
    let output = cic("lex", "ok.ci");
    assert!(output.status.success());

    let expected = [
        "<Ident, 'x', 0>",
        "<Eq, '=', 2>",
        "<Integer, '5', 4>",
        "<Semicolon, ';', 5>",
        "<OpenBrace, '{', 7>",
        "<If, 'if', 11>",
        "<Ident, 'x', 14>",
        "<EqEq, '==', 16>",
        "<Integer, '5', 19>",
        "<OpenBrace, '{', 21>",
        "<Ident, 'x', 27>",
        "<Eq, '=', 29>",
        "<Integer, '1', 31>",
        "<Semicolon, ';', 32>",
        "<CloseBrace, '}', 36>",
        "<Else, 'else', 38>",
        "<OpenBrace, '{', 43>",
        "<CloseBrace, '}', 47>",
        "<If, 'if', 51>",
        "<Ident, 'x', 54>",
        "<EqEq, '==', 56>",
        "<Integer, '0', 59>",
        "<OpenBrace, '{', 61>",
        "<Ident, 'x', 67>",
        "<Eq, '=', 69>",
        "<Integer, '9', 71>",
        "<Semicolon, ';', 72>",
        "<CloseBrace, '}', 76>",
        "<Else, 'else', 78>",
        "<OpenBrace, '{', 83>",
        "<Ident, 'x', 89>",
        "<Eq, '=', 91>",
        "<Ident, 'x', 93>",
        "<Plus, '+', 95>",
        "<Integer, '10', 97>",
        "<Semicolon, ';', 99>",
        "<CloseBrace, '}', 103>",
        "<Return, 'return', 107>",
        "<Ident, 'x', 114>",
        "<Semicolon, ';', 115>",
        "<CloseBrace, '}', 117>",
    ];

    assert_eq!(stdout(&output), expected);
}

#[test]
fn parse_ok_program() {
    let output = cic("parse", "ok.ci");
    assert!(output.status.success());

    let expected = [
        r"  =",
        r" / \",
        r"x   5",
        r"",
        r"if",
        r"    _==",
        r"   /   \",
        r"  x     5",
        r"  then",
        r"      =",
        r"     / \",
        r"    x   1",
        r"  else",
        r"",
        r"if",
        r"    _==",
        r"   /   \",
        r"  x     0",
        r"  then",
        r"      =",
        r"     / \",
        r"    x   9",
        r"  else",
        r"      =",
        r"     / \",
        r"    x   +",
        r"       / \",
        r"      x   10",
        r"",
        r"return",
        r"  x",
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
    let cases = [
        ("undeclared.ci", "semantic error at 11: use of undeclared variable `y`\n"),
        ("undeclared_rhs.ci", "semantic error at 15: use of undeclared variable `y`\n"),
    ];

    for (fixture, expected) in cases {
        for subcommand in ["eval", "run", "build"] {
            let output = cic(subcommand, fixture);
            assert!(!output.status.success(), "{subcommand} {fixture}");
            assert_eq!(stderr(&output), expected, "{subcommand} {fixture}");
        }
    }
}

#[test]
fn if_without_else_fails() {
    let output = cic("parse", "missing_else.ci");
    assert!(!output.status.success());
    assert_eq!(
        stderr(&output),
        "parsing error at 29: expected keyword `else` but found keyword `return`\n"
    );
}

#[test]
fn generated_assembly_declares_variables_in_bss() {
    let program = parser::parse("x = 11 * 7;\ny = x * 3 + 5;\n{ return x * y; }").unwrap();
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
