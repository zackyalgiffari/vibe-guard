# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- `main.rs` now invokes the CLI and propagates its exit code (it previously only
  printed a placeholder message and never ran `check`/`config`/`index`).

### Added
- Open-source docs: full `README.md`, `SECURITY.md`, PR template, and issue templates.

## [0.1.0] - 2026-05-29

### Added
- **RTK-Diff engine** — AST-level structural diff via tree-sitter, classifying each
  changed file as `BEHAVIORAL`, `REFACTOR`, or `BOILERPLATE`.
- Language support for Rust, TypeScript, JavaScript (incl. JSX/TSX), Python, and Go.
- **Vibe summary** — a one-line human-readable change report.
- **Intent Guard** — semantic match of stated intent vs. structural change, with a
  pluggable LLM provider (local Ollama) and a built-in heuristic fallback that runs with
  no model installed.
- **Indexer** — local file-freshness / context-coverage tracking (`index sync`).
- **Safety guard** — detection of secret access, env-var usage, and sensitive filenames,
  requiring explicit confirmation.
- CLI (`check`, `config`, `index sync`) with `--intent`, `--diff`, `--model`, `--lang`,
  `--no-llm`, `--yes`, and `--json` flags.

[Unreleased]: https://github.com/zackyalgiffari/vibe-guard/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/zackyalgiffari/vibe-guard/releases/tag/v0.1.0
