# AGENTS.md

## Overview

Rust interactive CLI (package name `tui`, edition 2024). REPL for file operations, HTTP requests, AI chat, and voice (STT/TTS) via local models. No tests, no CI.

## Commands

```
cargo build
cargo run              # launches interactive REPL
cargo clippy           # defaults, no custom config
cargo fmt              # defaults, no custom config
```

## Build quirks

- **Rust 2024 edition** — requires rustc 1.85+.
- **Static CRT on Windows** — `.cargo/config.toml` sets `+crt-static` and `CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded`. This is required for `sherpa-onnx` and `llama-cpp-2` to link correctly.
- **Heavy native deps** — `sherpa-onnx`, `llama-cpp-2`, `cpal`, `rodio` need C/C++ toolchains (CMake, MSVC). Builds are slow.
- **Local model files** — `models/` contains GGUF, ONNX, and safetensors files used at runtime by voice and LLM features. These are not checked in via normal means (large files). If missing, voice/LLM commands will fail.

## Structure

```
src/
  main.rs              # async REPL loop, hardcoded voice/agent commands + registry dispatch
  registry.rs          # command registry (sync + async handlers), build_registry()
  error.rs             # anyhow re-exports
  file_ops/            # CRUD + gzip/zip compression
  commands/            # system commands (e.g. system info)
  http/                # HTTP GET requests
  download/            # download manager (stub)
  ai/                  # AI chat via rig-core (local LLM)
  voice/               # STT, TTS, voice agent, streaming agent, local record/play
    agent.rs           # rig-core voice agent
    streaming_agent.rs # streaming variant
    llm.rs             # local LLM via llama-cpp-2
    stt.rs             # speech-to-text (sherpa-onnx)
    tts.rs             # text-to-speech (sherpa-onnx)
    local.rs           # record and playback (cpal/hound)
models/                # ONNX + GGUF model files (runtime, not in normal git)
examples/              # standalone experiments, not part of the app
```

## Conventions

- **Error handling migration in progress** — `error.rs` provides `anyhow::Result` and `Context`. Existing code still uses `match`/`unwrap` — migrate to `?` with `anyhow::Result` as functions are touched.
- **Examples use `[dev-dependencies]`** — `actix-web`, `csv`, `error-chain`, `rhai`, `select`, `tempfile` are dev-only. Don't move them to `[dependencies]`.
- **Rhai scripts** — `src/add.rhai` and `src/multiply.rhai` are only used by `examples/actix_server.rs`.
- **Command dispatch** — some commands (`agent`, `voice`, `llm`, `stream`, `tts`, `stt`) are hardcoded in `main.rs` outside the registry. All others go through `registry.rs`.
