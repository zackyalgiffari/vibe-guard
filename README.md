# vibe-guard

> A structural validation layer for vibe coders — catch what AI edits *actually* changed before you commit.

[![CI](https://github.com/zackyalgiffari/vibe-guard/actions/workflows/ci.yml/badge.svg)](https://github.com/zackyalgiffari/vibe-guard/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)

`vibe-guard` sits between AI-generated code and your codebase. Instead of a line-based
`git diff`, it parses your changes at the **AST level**, classifies what each file
actually did, and verifies the change against the intent you stated — so you get a
one-line "vibe check" instead of manually reading every modified line.

Everything runs **fully local**. No code leaves your machine. The optional LLM step
uses a local [Ollama](https://ollama.com) model, and if no model is available it falls
back to a built-in heuristic that needs nothing installed.

```
$ vibe-guard check --intent "Refactor the login handler to use the new database connector"

[RTK-Diff]  src/auth.ts              → BEHAVIORAL  (2 functions modified)
[RTK-Diff]  src/db.ts                → REFACTOR    (connector swap)

[Vibe]      Logic Update in 2 functions, 1 Refactor
[Indexer]   Context coverage: 100% (2/2 fresh)

[Intent Guard] Confidence: 0.71 ⚠️  (via heuristic)
  ↳ Intent NOT clearly matched ❌
  ↳ Side effect: Password validation logic removed in src/auth.ts

Proceed? [y/N]:
```

## Features

- **RTK-Diff engine** — AST-level structural diff via [tree-sitter](https://tree-sitter.github.io/tree-sitter/),
  classifying every changed file as:
  - `BEHAVIORAL` — logic updates, changed/added/removed function bodies or signatures
  - `REFACTOR` — renames, moves, restructuring with no behavioral change
  - `BOILERPLATE` — imports, formatting, whitespace, comments (auto-skipped)
- **Vibe summary** — rolls the whole diff into a single human-readable line, e.g.
  `"Logic Update in 2 functions, 1 Refactor, 3 boilerplate skipped"`.
- **Intent Guard** — compares your stated intent against the structural change and
  returns a confidence score plus detected side effects. Pluggable provider: a local
  Ollama model when available, with an always-on heuristic fallback.
- **Indexer / context coverage** — tracks whether the files you're changing were seen at
  their latest committed version; stale or uncovered files lower the confidence score.
- **Safety guard** — flags any change that touches secrets, environment variables, or
  sensitive filenames (`.env`, `.pem`, `*secret*`, …) and requires an explicit typed
  confirmation before approving.

## Supported languages

| Language | Extensions |
| :--- | :--- |
| Rust | `.rs` |
| TypeScript | `.ts` `.mts` `.cts` `.tsx` |
| JavaScript | `.js` `.jsx` `.mjs` `.cjs` |
| Python | `.py` `.pyi` |
| Go | `.go` |

Files in other languages are still listed but not analyzed at the AST level.

## Install

With Cargo, straight from the repo:

```bash
cargo install --git https://github.com/zackyalgiffari/vibe-guard
```

Or build from source:

```bash
git clone https://github.com/zackyalgiffari/vibe-guard
cd vibe-guard
cargo build --release
# binary at target/release/vibe-guard
```

## Usage

```
vibe-guard <COMMAND>

Commands:
  check        Analyze the current diff and verify it against your stated intent
  config       Print the resolved config (creating a default file if none exists)
  index sync   Rebuild the local file-freshness index from the current HEAD
```

### `check`

By default `check` analyzes `git diff HEAD` in the current repository.

```bash
# Vibe-check the current working changes against your intent
vibe-guard check --intent "add retry logic to the http client"

# AST analysis only — skip the Intent Guard entirely
vibe-guard check --no-llm

# Analyze a pre-generated patch file instead of the git working tree
vibe-guard check --diff my-change.patch --intent "rename User to Account"
```

| Flag | Description |
| :--- | :--- |
| `--intent <string>` | The developer's intent / original prompt. |
| `--diff <file>` | Path to a pre-generated unified diff (default: `git diff HEAD`). |
| `--model <string>` | Override the local LLM model (default: from config). |
| `--lang <list>` | Comma-separated language filter, e.g. `rust,ts,py,go`. |
| `--no-llm` | Skip the Intent Guard; run AST analysis only. |
| `--yes` | Auto-approve non-sensitive, high-confidence changes (no prompt). |
| `--json` | Output the report as JSON. |

> When analyzing a `--diff` file (rather than the live working tree), the context-coverage
> and safety scans are skipped, since both need the actual repository contents.

`check` exits non-zero when you decline a prompt or abort a sensitive change, so it
composes well in scripts and pre-commit hooks.

## Configuration

Config lives at `~/.vibe-guard/config.toml`. Run `vibe-guard config` to create it with
defaults and print the resolved values.

| Key | Default | Description |
| :--- | :--- | :--- |
| `model` | `mistral:7b-instruct` | Local LLM model name passed to the provider (Ollama tag). |
| `ollama_url` | `http://localhost:11434` | Base URL of the Ollama HTTP API. |
| `confidence_threshold` | `0.75` | Reports below this are treated as low-confidence warnings. |
| `auto_approve_boilerplate` | `true` | Auto-approve boilerplate-only changes without prompting. |
| `sensitive_patterns` | `.env`, `.pem`, `.key`, `secrets`, `credentials`, `id_rsa` | Filename markers that flag a file as sensitive. |
| `secret_identifiers` | `secret`, `password`, `token`, `api_key`, … | Identifier substrings that indicate secret access. |

### Local LLM (Ollama)

The Intent Guard is optional. If you have [Ollama](https://ollama.com) running with the
configured model, `vibe-guard` uses it for semantic intent matching:

```bash
ollama pull mistral:7b-instruct
ollama serve
```

If Ollama is unreachable, `vibe-guard` prints a note and falls back to a built-in
heuristic Intent Guard — so the tool works out of the box with nothing installed. Use
`--no-llm` to skip the Intent Guard entirely and run only the AST analysis.

## Non-goals (v1)

- No remote/cloud LLM calls — all inference runs locally.
- No UI or IDE plugin (CLI only in v1).
- No auto-revert — the guard warns, but never undoes a change for you.
- No binary-file diffs.

## Contributing

Contributions of all kinds are welcome. See [CONTRIBUTING.md](./CONTRIBUTING.md) for the
local checks CI runs and how to add a new language grammar. By participating you agree to
the [Code of Conduct](./CODE_OF_CONDUCT.md). Security issues: see [SECURITY.md](./SECURITY.md).

## License

[MIT](./LICENSE) © zackyalgiffari
