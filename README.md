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

To actually compile some file to an executable, you may run:

```sh
cargo run -- build [file path]
```

Then, to execute, you can just run it from the current directory:

```sh
./[file path]
```
