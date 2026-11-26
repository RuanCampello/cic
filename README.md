# CIC

CI compiler

--- 
This is a compiler's class project, which will be heavily structured after my 
other compiler language project, the [nyx compiler](https://gitlab.com/ruancampello/nyx/-/tree/feat/fmt) that's also built in Rust.

This is meant to be run only on x86-64 especially with Linux idiosyncrasies and tools.

Group: Ruan Campello - 20240101758 (solo)

LLM usage: I used an LLM to implement the syntax tree printing (`src/frontend/tree.rs`), to validate the generated assembly, and for help with the assembly generation of comparisons and commands (`if`/`while`) in `src/codegen.rs`. Everything else was written by me.

---

The compiler implements the Fun language with the syntax from the specification:

```
<programa> ::= <decl>* 'main' '{' <cmd>* 'return' <exp> ';' '}'
<decl>     ::= <vardecl> | <fundecl>
<fundecl>  ::= 'fun' <id> '(' <arglist>? ')' '{' <vardecl>* <cmd>* 'return' <exp> ';' '}'
<arglist>  ::= <id> | <id> ',' <arglist>
<vardecl>  ::= 'var' <id> '=' <exp> ';'
<cmd>      ::= <if> | <while> | <atrib>
<if>       ::= 'if' <exp> '{' <cmd>* '}' 'else' '{' <cmd>* '}'
<while>    ::= 'while' <exp> '{' <cmd>* '}'
<atrib>    ::= <id> '=' <exp> ';'
<exp>      ::= <exp_a> (('<' | '>' | '==') <exp_a>)*
<exp_a>    ::= <exp_m> (('+' | '-') <exp_m>)*
<exp_m>    ::= <prim> (('*' | '/') <prim>)*
<prim>     ::= <num> | <id> | '(' <exp> ')' | <fun>
<fun>      ::= <id> '(' <params>? ')'
<params>   ::= <exp> | <exp> ',' <params>
```

```
fun fib(n) {
  var res = 0;
  if n < 2 {
    res = 1;
  } else {
    res = fib(n - 1) + fib(n - 2);
  }
  return res;
}

main {
  return fib(10);
}
```

Comparisons result in 1 (true) or 0 (false), and any value other than 0 is true in `if` and
`while`. Global variables are 64-bit integers placed in the `.bss` section. Parameters and
locals live in the function's activation record on the stack, following the calling
convention from the specification (arguments pushed last to first, `push %rbp`,
`sub`, `mov %rsp, %rbp`).

Declarations are checked in order, so a function can call itself and earlier functions, but
not functions declared after it (no mutual recursion). Locals and parameters hide globals of
the same name. All semantic errors are reported before any code is generated.

Small differences from the specification:

- function labels are prefixed with `fun_` (and globals with `var_`), so names like `sair`
  don't clash with the runtime
- a trailing comma in a parameter or argument list is a syntax error
- a function name can't be reused by a variable or another function, and a function can't
  repeat a name among its parameters and locals (globals can still be redeclared)
- operands and arguments are evaluated right to left, in `eval` as well as in the compiled
  code, so functions that change globals give the same result in both

---

To run the testing cases, as it is in any Rust project, you might run:

```sh
cargo test
```

Unit tests cover the lexer, parser, semantic analysis and interpreter. Integration tests
use the programs in `tests/fixtures/`. The programs `minimal`, `delta`, `sum`,
`remainder`, `gcd`, `ok`, `fib` (recursion), `abs` (a function calling another one) and
`calls` (functions with and without parameters or locals, changing a global) are run with
both `eval` and `run`, checking the printed value. The other fixtures check the errors: a
lexical error, a syntax error, an `if` without `else`, undeclared variables, a call to an
undeclared function, a wrong number of arguments, calling a variable and using a local
outside its function. Several smaller programs (comparisons, `if`, `while`, calls,
shadowing, parameter order, factorial) are also compiled, assembled, linked and run.

To build the binary (of the compiler itself), run: 

```sh
cargo build --release
```

To run the compiler, those are the commands you might wanna use:

```sh
cargo run -- lex   [file]            # token sequence
cargo run -- parse [file]            # syntax tree
cargo run -- eval  [file]            # value of the program
cargo run -- build [file] [-o out.s] # assembly (defaults to [file].s)
cargo run -- run   [file]            # compiles and runs natively
```

To turn the generated assembly into an executable, `runtime.s` (in `asm/`) must be
in the same directory as the `.s` file:

```sh
cp asm/runtime.s .
as --64 -o p1.o p1.s
ld -o p1 p1.o
./p1
```

You can also use `target/release/cic`, of course, to run it manually after building :D
