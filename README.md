# CIC

CI compiler

--- 
This is a compiler's class project, which will be heavily structured after my 
other compiler language project, the [nyx compiler](https://gitlab.com/ruancampello/nyx/-/tree/feat/fmt) that's also built in Rust.

This is meant to be run only on x86-64 especially with Linux idiosyncrasies and tools.

The group is formed by me only. 

LLM usage: I used an LLM to implement the syntax tree printing (`src/frontend/tree.rs`) and to validate the generated assembly. Everything else was written by me.

---

To run the testing cases, as it is in any Rust project, you might run:

```sh
cargo test
```

To build the binary (of the compiler itself), run: 

```sh
cargo build --release
```

To run the compiler, those are the commands you might wanna use:

```sh
cargo run -- lex   [file]            # token sequence
cargo run -- parse [file]            # syntax tree
cargo run -- eval  [file]            # value of the expression
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
