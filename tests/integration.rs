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
        ("constant", "main { return 333; }", "333"),
        ("mul", "main { return (6 * 7); }", "42"),
        ("nested", "main { return (3 + (4 + (11 + 7))); }", "25"),
        ("spec", "main { return ((427 / 7) + (11 * (231 + 5))); }", "2657"),
        ("operand_order", "main { return (100 - (20 /3)); }", "94"),
        ("negative", "main { return (3 - 10); }", "-7"),
        ("i64_max", "main { return 9223372036854775807; }", "9223372036854775807"),
        ("precedence", "main { return 2 + 3 * 4 - 10 / 2; }", "9"),
        ("left_assoc", "main { return 100 - 20 - 5; }", "75"),
        ("perimeter", "var l = 30;\nvar c = 40;\nmain { return l + l + c + c; }", "140"),
        (
            "chained",
            "var x = 11 * 7;\nvar y = x * 3 + 5;\nmain { return x * y; }",
            "18172",
        ),
        ("redeclared", "var x = 2;\nvar x = x * 21;\nmain { return x; }", "42"),
        (
            "long_names",
            "var Width1 = 6;\nvar heightZ = 7;\nmain { return Width1 * heightZ; }",
            "42",
        ),
        ("less", "main { return 3 < 5; }", "1"),
        ("not_less", "main { return 5 < 3; }", "0"),
        ("greater", "main { return 5 > 3; }", "1"),
        ("not_greater", "main { return 3 > 3; }", "0"),
        ("equal", "main { return 4 == 4; }", "1"),
        ("not_equal", "main { return 4 == 5; }", "0"),
        ("negative_less", "main { return 0 - 5 < 0; }", "1"),
        ("comparison_precedence", "main { return 1 + 2 == 9 / 3; }", "1"),
        ("assign", "var x = 1;\nmain { x = x + 41; return x; }", "42"),
        ("if_then", "var x = 0;\nmain { if 1 { x = 1; } else { x = 2; } return x; }", "1"),
        ("if_else", "var x = 0;\nmain { if 0 { x = 1; } else { x = 2; } return x; }", "2"),
        ("empty_arms", "var x = 7;\nmain { if x { } else { } return x; }", "7"),
        ("while_skipped", "var x = 3;\nmain { while 0 { x = 0; } return x; }", "3"),
        (
            "nested_commands",
            "var i = 0;\nvar evens = 0;\nmain { while i < 10 { if i == i / 2 * 2 { evens = evens + 1; } else { } i = i + 1; } return evens; }",
            "5",
        ),
        ("no_params", "fun seven() { return 7; }\nmain { return seven() * 6; }", "42"),
        (
            "no_locals",
            "fun add(x, y) { return x + y; }\nmain { return add(40, 2); }",
            "42",
        ),
        (
            "param_order",
            "fun sub(x, y) { return x - y; }\nmain { return sub(50, 8); }",
            "42",
        ),
        (
            "assign_param",
            "fun f(x) { x = x + 1; return x; }\nmain { return f(41); }",
            "42",
        ),
        (
            "param_shadows",
            "var x = 1;\nfun f(x) { return x * 2; }\nmain { return f(21) + x; }",
            "43",
        ),
        (
            "local_shadows",
            "var y = 5;\nfun f() { var y = 40; return y; }\nmain { return f() + y; }",
            "45",
        ),
        (
            "global_in_init",
            "var y = 5;\nfun f() { var y = y + 1; return y; }\nmain { return f(); }",
            "6",
        ),
        (
            "assign_global",
            "var n = 0;\nvar t = 0;\nfun bump(k) { n = n + k; return n; }\nmain { t = bump(40); t = bump(2); return n; }",
            "42",
        ),
        (
            "nested_calls",
            "fun inc(x) { return x + 1; }\nfun twice(x) { return inc(inc(x)); }\nmain { return twice(twice(38)); }",
            "42",
        ),
        (
            "factorial",
            "fun fact(n) { var r = 1; if n > 1 { r = n * fact(n - 1); } else { } return r; }\nmain { return fact(10); }",
            "3628800",
        ),
        (
            "calls_in_binary",
            "fun one() { return 1; }\nfun two() { return 2; }\nmain { return two() * 10 + one(); }",
            "21",
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
        ("fib.ci", "89"),
        ("abs.ci", "8"),
        ("calls.ci", "168"),
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
        "<Var, 'var', 0>",
        "<Ident, 'x', 4>",
        "<Eq, '=', 6>",
        "<Integer, '5', 8>",
        "<Semicolon, ';', 9>",
        "<Main, 'main', 11>",
        "<OpenBrace, '{', 16>",
        "<If, 'if', 20>",
        "<Ident, 'x', 23>",
        "<EqEq, '==', 25>",
        "<Integer, '5', 28>",
        "<OpenBrace, '{', 30>",
        "<Ident, 'x', 36>",
        "<Eq, '=', 38>",
        "<Integer, '1', 40>",
        "<Semicolon, ';', 41>",
        "<CloseBrace, '}', 45>",
        "<Else, 'else', 47>",
        "<OpenBrace, '{', 52>",
        "<CloseBrace, '}', 56>",
        "<If, 'if', 60>",
        "<Ident, 'x', 63>",
        "<EqEq, '==', 65>",
        "<Integer, '0', 68>",
        "<OpenBrace, '{', 70>",
        "<Ident, 'x', 76>",
        "<Eq, '=', 78>",
        "<Integer, '9', 80>",
        "<Semicolon, ';', 81>",
        "<CloseBrace, '}', 85>",
        "<Else, 'else', 87>",
        "<OpenBrace, '{', 92>",
        "<Ident, 'x', 98>",
        "<Eq, '=', 100>",
        "<Ident, 'x', 102>",
        "<Plus, '+', 104>",
        "<Integer, '10', 106>",
        "<Semicolon, ';', 108>",
        "<CloseBrace, '}', 112>",
        "<Return, 'return', 116>",
        "<Ident, 'x', 123>",
        "<Semicolon, ';', 124>",
        "<CloseBrace, '}', 126>",
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
        r"main",
        r"  if",
        r"      _==",
        r"     /   \",
        r"    x     5",
        r"    then",
        r"        =",
        r"       / \",
        r"      x   1",
        r"    else",
        r"",
        r"  if",
        r"      _==",
        r"     /   \",
        r"    x     0",
        r"    then",
        r"        =",
        r"       / \",
        r"      x   9",
        r"    else",
        r"        =",
        r"       / \",
        r"      x   +",
        r"         / \",
        r"        x   10",
        r"",
        r"  return",
        r"    x",
    ];
    assert_eq!(stdout(&output), expected)
}

#[test]
fn lexical_error_fails() {
    let output = cic("parse", "lex_error.ci");
    assert!(!output.status.success());
    assert!(stderr(&output).starts_with("lexical error at 8"));
}

#[test]
fn syntax_error_fails() {
    let output = cic("parse", "syntax_error.ci");
    assert!(!output.status.success());
    assert!(stderr(&output).starts_with("parsing error at 11"));
}

#[test]
fn semantic_errors_fail() {
    let cases = [
        ("undeclared.ci", "semantic error at 20: use of undeclared variable `y`\n"),
        ("undeclared_rhs.ci", "semantic error at 24: use of undeclared variable `y`\n"),
        ("local_scope.ci", "semantic error at 66: use of undeclared variable `y`\n"),
        (
            "undeclared_function.ci",
            "semantic error at 20: call to undeclared function `g`\n",
        ),
        ("arity.ci", "semantic error at 51: `add` takes 2 argument(s) but 1 were given\n"),
        ("not_a_function.ci", "semantic error at 28: `f` is a variable, not a function\n"),
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
        "parsing error at 38: expected keyword `else` but found keyword `return`\n"
    );
}

#[test]
fn generated_assembly_declares_variables_in_bss() {
    let program =
        parser::parse("var x = 11 * 7;\nvar y = x * 3 + 5;\nmain { return x * y; }").unwrap();
    let symbols = semantic::analyse(&program).unwrap();
    let asm = codegen::generate(&program, &symbols);

    let bss = asm.find(".section .bss").expect("a bss section");
    let text = asm.find(".section .text").expect("a text section");
    let x = asm.find(".lcomm var_x, 8").expect("x to be declared");
    let y = asm.find(".lcomm var_y, 8").expect("y to be declared");
    assert!(bss < x && x < text && bss < y && y < text);
    assert!(asm.contains("mov %rax, var_x") && asm.contains("mov var_x, %rax"));
}

#[test]
fn functions_follow_the_calling_convention() {
    let src = "fun f(x) {\n  var y = x * x + 7;\n  var z = y * x - 9;\n  return y * z;\n}\nmain { return f(42); }";
    let program = parser::parse(src).unwrap();
    let symbols = semantic::analyse(&program).unwrap();
    let asm = codegen::generate(&program, &symbols);

    let call = ["  mov $42, %rax", "  push %rax", "  call fun_f", "  add $8, %rsp"].join("\n");
    let prologue = ["fun_f:", "  push %rbp", "  sub $16, %rsp", "  mov %rsp, %rbp"].join("\n");
    let epilogue = ["  add $16, %rsp", "  pop %rbp", "  ret"].join("\n");
    assert!(asm.contains(&call), "{asm}");
    assert!(asm.contains(&prologue) && asm.contains(&epilogue), "{asm}");
    assert!(asm.contains("mov %rax, 0(%rbp)") && asm.contains("mov %rax, 8(%rbp)"));
    assert!(
        asm.contains("mov 32(%rbp), %rax"),
        "x lives past the locals, rbp and return address"
    );

    let sair = asm.find("call sair").expect("main to exit");
    assert!(sair < asm.find("fun_f:").unwrap(), "functions come after the main program");
}

#[test]
fn parse_function_program() {
    let output = cic("parse", "abs.ci");
    assert!(output.status.success(), "{}", stderr(&output));

    let lines = stdout(&output);
    for header in ["fun abs(x)", "fun delta(a, b, c)", "main"] {
        assert!(lines.contains(&header), "{header} missing from {lines:#?}");
    }
    let main = [r"main", r"  return", r"    delta()", r"     / | \", r"    a  b  c"];
    assert!(lines.ends_with(&main), "{lines:#?}");
}

#[inline]
fn stdout(output: &process::Output) -> Vec<&str> {
    str::from_utf8(&output.stdout).expect("stdout to be utf-8").lines().collect()
}

#[inline]
fn stderr(output: &process::Output) -> &str {
    str::from_utf8(&output.stderr).expect("stderr to be utf-8")
}
