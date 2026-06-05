---
name: rust-modern-modules
description: Enforce modern Rust module file layout. Use when Codex creates, splits, moves, or refactors Rust modules, especially when deciding between `mod.rs` and `foo.rs` plus `foo/bar.rs` layouts.
---

# Rust Modern Modules

## Rule

Prefer Rust 2018+ module layout:

```text
src/foo.rs
src/foo/bar.rs
```

Use this from `src/foo.rs`:

```rust
mod bar;
```

Do not create `src/foo/mod.rs` for new modules unless the repository already consistently uses `mod.rs` in that area or the user explicitly asks for it.

## Workflow

1. Inspect nearby Rust modules before creating files.
2. If both layouts are viable, choose `foo.rs` as the parent module and `foo/*.rs` for children.
3. If refactoring an existing `mod.rs`, avoid moving it unless the user asked for a layout cleanup or the move is necessary for the task.
4. Keep module declarations private by default: use `mod child;` first, then widen to `pub mod child;` only when external access is needed.
5. Run `cargo fmt` after moving or creating Rust module files.

## Examples

For a logging module with a daily appender, prefer:

```text
src/logging.rs
src/logging/daily_file_appender.rs
```

Inside `src/logging.rs`:

```rust
mod daily_file_appender;

use daily_file_appender::DailyFileAppender;
```
