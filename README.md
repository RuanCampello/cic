# CIC

CI compiler

--- 
This a compiler's class project, that will be heavily structured after my 
other compiler language project, the [nyx compiler](https://gitlab.com/ruancampello/nyx/-/tree/feat/fmt) that's also built in rust.

This is meant to be ran only on x86-64 specially with Linux idiosyncrasies and tools.

---

To run the testing cases, as it is in any rust project, you might run:

```sh
cargo test
```

To actually compile some file to an executable, you may run:

```sh
cargo run -- build [file path]
```

Then, to execute, you can just access it in the current directory:

```sh
./[file path]
```
