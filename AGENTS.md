# AGENTS.md

## Overview

Rust file manager + download manager CLI (package name `filemanager`, edition 2024). Interactive REPL for file operations. Download manager and AI integration planned.

## Commands

```
cargo build
cargo run              # launches interactive CLI (create/write/read/delete/compress/decompress)
cargo clippy           # no custom config — defaults apply
cargo fmt              # no custom config — defaults apply
```

No tests, no CI, no task runner.

## Structure

```
src/
  main.rs              # REPL loop, dispatches to modules
  error.rs             # anyhow re-exports for unified error handling
  file_ops/            # file operations (CRUD, compression)
    mod.rs
    create.rs          # create_file, create_directory
    read.rs            # read_file, get_file_type, read_entries
    write.rs           # write_file
    delete.rs          # delete_file
    compress.rs        # compress_file (gzip), decompress_file (zip)
  download/            # download manager (stub — not yet implemented)
    mod.rs
    http.rs
examples/              # standalone experiments, not part of the app
```

## Quirks

- **Rust 2024 edition** — requires rustc 1.85+.
- **Error handling migration in progress** — `error.rs` provides `anyhow::Result` and `Context`. Existing file_ops still use `match`/`unwrap` — migrate to `?` with `anyhow::Result` as functions are touched.
- **Examples use `[dev-dependencies]`** — `actix-web`, `csv`, `error-chain`, `rhai`, `select`, `tempfile` are dev-only. Don't move them back to `[dependencies]`.
- **Rhai scripts** — `src/add.rhai` and `src/multiply.rhai` are only used by `examples/actix_server.rs`.
