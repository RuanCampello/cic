# CIC

CI compiler

--- 
This is a compiler's class project, which will be heavily structured after my 
other compiler language project, the [nyx compiler](https://gitlab.com/ruancampello/nyx/-/tree/feat/fmt) that's also built in Rust.

This is meant to be run only on x86-64 especially with Linux idiosyncrasies and tools.

The group is formed by me only. I didn't use any LLM at this stage of the project.

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
cargo run -- lex   [file]   # token sequence
cargo run -- parse [file]   # syntax tree
cargo run -- eval  [file]   # value of the expression
```

You can also use `target/release/cic`, of course, to run it manually after building :D
