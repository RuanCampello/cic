# CIC

CI compiler

--- 
This is a compiler's class project, which will be heavily structured after my 
other compiler language project, the [nyx compiler](https://gitlab.com/ruancampello/nyx/-/tree/feat/fmt) that's also built in Rust.

This is meant to be run only on x86-64 especially with Linux idiosyncrasies and tools.

Group: Ruan Campello - 20240101758 (solo)

LLM usage: I used an LLM to implement the syntax tree printing (`src/frontend/tree.rs`), to validate the generated assembly, and for help with the assembly generation of comparisons and commands (`if`/`while`) in `src/codegen.rs`. Everything else was written by me.

---

The compiler implements the Cmd language with the syntax from the specification, no variation:

```
<programa> ::= <decl>* '{' <cmd>* 'return' <exp> ';' '}'
<decl>     ::= <id> '=' <exp> ';'
<cmd>      ::= <if> | <while> | <atrib>
<if>       ::= 'if' <exp> '{' <cmd>* '}' 'else' '{' <cmd>* '}'
<while>    ::= 'while' <exp> '{' <cmd>* '}'
<atrib>    ::= <id> '=' <exp> ';'
<exp>      ::= <exp_a> (('<' | '>' | '==') <exp_a>)*
<exp_a>    ::= <exp_m> (('+' | '-') <exp_m>)*
<exp_m>    ::= <prim> (('*' | '/') <prim>)*
<prim>     ::= <num> | <id> | '(' <exp> ')'
```

```
n = 1;
m = 10;
sum = 0;
{
  while n < m {
    sum = sum + n;
    n = n + 1;
  }
  return sum;
}
```

Comparisons result in 1 (true) or 0 (false), and any value other than 0 is true in `if` and
`while`. Using or assigning a variable that was not declared before the body is a semantic
error, reported before any code is generated. Variables are 64-bit integers placed in the
`.bss` section.

---

To run the testing cases, as it is in any Rust project, you might run:

```sh
cargo test
```

Unit tests cover the lexer, parser, semantic analysis and interpreter. Integration tests
use the programs in `tests/fixtures/`. The spec examples (`minimal`, `delta`, `sum`,
`remainder`, `gcd`) and `ok.ci` are run with both `eval` and `run`, checking the printed
value. The other fixtures check the errors: a lexical error, a syntax error, an `if`
without `else`, and undeclared variables on each side of an assignment. Several smaller
programs with comparisons, `if` and `while` are also compiled, assembled, linked and run.

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
