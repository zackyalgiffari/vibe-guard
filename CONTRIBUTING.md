# Contributing to vibe-guard

Thanks for your interest in improving `vibe-guard`! Contributions of all kinds are
welcome — bug reports, feature requests, docs, and code.

## Getting started

```bash
git clone https://github.com/zackyalgiffari/vibe-guard
cd vibe-guard
cargo build
cargo test
```

## Before opening a pull request

Please make sure the same checks CI runs pass locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
```

- Keep changes focused; one logical change per PR.
- Add or update tests in `tests/` for behavior changes.
- Match the existing code style (naming, comment density, error handling with `anyhow`).

## Adding a language grammar

The AST layer is driven by `src/ast/lang.rs`. To add a language:

1. Add its `tree-sitter-<lang>` crate to `Cargo.toml`.
2. Extend the `Language` enum, `from_path`, `ts_language()`, and `func_kinds()`.
3. Add classification tests in `tests/engine_tests.rs`.

## Reporting bugs

Open an issue with a minimal reproduction: the input diff (or `--diff` file), the command
you ran, and what you expected vs. what happened. Redact any real secrets.

## Code of Conduct

By participating, you agree to abide by our [Code of Conduct](./CODE_OF_CONDUCT.md).
