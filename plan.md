# Semantic Diff & Intent Guard
> A structural validation layer for vibe coders — inspired by the RTK (Rust Token Killer) architecture.

---

## 1. Overview

The **Semantic Diff & Intent Guard** is a **CLI tool** that sits between AI-generated code and your codebase. It intercepts diffs, parses them at the AST level, and runs an intent verification loop to ensure AI edits match what you actually asked for — without requiring you to manually read every changed line.

```
┌──────────────────────────────────────────────┐
│  Developer Intent (prompt / commit message)  │
└───────────────────┬──────────────────────────┘
                    │
                    ▼
         ┌──────────────────┐
         │  RTK-Diff Engine │  ← AST-level structural diff
         │  (tree-sitter)   │
         └────────┬─────────┘
                  │
                  ▼
         ┌──────────────────┐
         │  Intent Guard    │  ← Local LLM semantic match
         │  (accuracy mode) │
         └────────┬─────────┘
                  │
        ┌─────────┴──────────┐
        ▼                    ▼
   ✅ Approved           ⚠️ Warning
   (proceed)         (side-effect detected)
```

---

## 2. Core Objectives

| Goal | Description |
| :--- | :--- |
| **Reduce Cognitive Load** | Deliver 1-line "vibe checks" — no need to manually inspect every line of a diff |
| **Enhance Reliability** | Ensure high-speed AI edits match stated developer intent |
| **Token Efficiency** | RTK-inspired filtering keeps validation lightweight with `<100ms` overhead on the AST layer |

---

## 3. Language Support (Phase 1)

The following languages are supported at launch via `tree-sitter` grammars:

- **Rust** — `tree-sitter-rust`
- **TypeScript / JavaScript** — `tree-sitter-typescript`, `tree-sitter-javascript`
- **Python** — `tree-sitter-python`
- **Go** — `tree-sitter-go`

> Additional grammars (Java, C++, Ruby) can be registered as community plugins post-launch.

---

## 4. Technical Architecture

### System Flow

```mermaid
flowchart TD
    A([Developer runs AI command]) --> B[Capture LLM Prompt / Intent]
    B --> C[Generate Git Diff]
    C --> D{RTK-Diff Engine}

    D --> E[Parse AST via tree-sitter]
    E --> F{Classify Change Type}
    F --> G[Refactor\nrename / move]
    F --> H[Behavioral Change\nlogic update]
    F --> I[Boilerplate\nimports / formatting]

    I --> J[Auto-approve & skip]
    G --> K[Intent Guard]
    H --> K

    K --> L[Local LLM — Accuracy Mode\ne.g. Mistral 7B / Phi-3 medium]
    L --> M{Semantic Match?}
    M -- ✅ High confidence --> N[Approved — Proceed]
    M -- ⚠️ Low confidence --> O[Warning Report]
    O --> P{Sensitive File?}
    P -- Yes --> Q[Env Encryption Flag]
    P -- No --> R[Prompt Developer: Proceed? y/N]
```

---

### A. Semantic Analysis Layer — The RTK-Diff Engine

Instead of a standard line-based `git diff`, this engine parses the **Abstract Syntax Tree (AST)** to identify _functional_ changes only.

**Change Classification:**

| Type | Definition | Action |
| :--- | :--- | :--- |
| `REFACTOR` | Renaming, moving logic, restructuring without behavioral change | Pass to Intent Guard |
| `BEHAVIORAL` | Logic updates, conditional changes, new/removed function bodies | Pass to Intent Guard |
| `BOILERPLATE` | Import statements, formatting, whitespace, comments | Auto-approve & skip |

**Implementation Details:**
- Parser: [`tree-sitter`](https://tree-sitter.github.io/tree-sitter/) with per-language grammars
- Filtering: Whitespace-only and comment-only diffs are discarded before reaching the LLM
- Output: Structured diff object — `{ file, changeType, affectedSymbols[], before_ast, after_ast }`

---

### B. The Intent Guard — Verification Loop

This module compares the **developer's stated intent** against the **structural code change**.

```mermaid
sequenceDiagram
    participant Dev as Developer
    participant CLI as semantic-guard CLI
    participant RTK as RTK-Diff Engine
    participant LLM as Local LLM (Accuracy Mode)
    participant Idx as Indexing LLM Read
    participant Enc as Env Encryption Layer

    Dev->>CLI: Run command with intent prompt
    CLI->>RTK: Pass diff + AST
    RTK->>CLI: Return classified change list
    CLI->>Idx: Query: has LLM seen latest file versions?
    Idx-->>CLI: Context coverage report
    CLI->>LLM: [Intent + Structural Diff + Context Coverage]
    LLM-->>CLI: Confidence score + detected side effects
    CLI->>Enc: Check if changed files touch secrets/env vars
    Enc-->>CLI: Sensitive file flag (if any)
    CLI->>Dev: ✅ Approved / ⚠️ Warning with report
```

**Input:**
- LLM prompt (the developer's original intent)
- Structured diff from RTK-Diff Engine
- Context coverage report from the Indexing LLM Read tool

**Local LLM — Accuracy Mode:**
- Default model: `Mistral-7B-Instruct` or `Phi-3-medium`
- Latency: acceptable (accuracy is prioritized over speed)
- Runs fully local — no data leaves the machine
- Model is configurable via `~/.semantic-guard/config.toml`

**Output:**
```
confidence: 0.87
intent_match: true
side_effects:
  - "Password validation logic removed in auth.ts (line 42)"
sensitive_files: []
```

---

## 5. Implementation Phases

```mermaid
gantt
    title Semantic Diff & Intent Guard — Roadmap
    dateFormat  YYYY-MM-DD
    section Phase 1 — AST Integration
    Setup tree-sitter multi-lang parsers     :p1a, 2025-08-01, 14d
    Build structural diff engine             :p1b, after p1a, 10d
    Function signature change detection      :p1c, after p1b, 7d

    section Phase 2 — Vibe Summary
    Heuristic change classifier              :p2a, after p1c, 7d
    1-line report generator                  :p2b, after p2a, 5d
    CLI output formatting                    :p2c, after p2b, 5d

    section Phase 3 — Intent Bridge
    Indexing LLM Read integration            :p3a, after p2c, 10d
    Local LLM semantic match pipeline        :p3b, after p3a, 10d
    Confidence scoring & side-effect output  :p3c, after p3b, 7d

    section Phase 4 — Safety Guard
    Sensitive file detection                 :p4a, after p3c, 7d
    Env Encryption layer integration         :p4b, after p4a, 7d
    End-to-end testing & tuning              :p4c, after p4b, 10d
```

### Phase Details

| Phase | Name | Key Deliverable |
| :---: | :--- | :--- |
| **1** | **AST Integration** | `tree-sitter`-powered diff engine for Rust, TS/JS, Python, Go. Detects modified function signatures. |
| **2** | **Vibe Summary** | Heuristic reporter that classifies changes into a single human-readable line, e.g. `"Logic Update in 2 functions, 1 Refactor"`. |
| **3** | **Intent Bridge** | Integration with the Indexing LLM Read tool. Verifies the AI had sufficient context before making changes. Local LLM pipeline with confidence scoring. |
| **4** | **Safety Guard** | Sensitive file warnings via the Env Encryption layer. Flags any changes touching code that interacts with encrypted env vars or secrets. |

---

## 6. Example Workflow

```mermaid
flowchart LR
    A["🧑 Developer Prompt\n'Refactor the login handler\nto use the new database connector'"]
    B["🤖 AI Action\nGenerates diff → auth.ts + db.ts"]
    C["🔍 RTK-Diff Engine\nParses AST of both files"]
    D["⚠️ Side Effect Detected\nPassword validation check\nremoved in auth.ts line 42"]
    E["🛡️ Intent Guard Report\nIntent: 'Database Refactor'\nActual: Logic deleted in auth.ts"]
    F["❓ Developer Prompt\nProceed? y/N"]

    A --> B --> C --> D --> E --> F
```

**Terminal Output:**
```
$ semantic-guard check --intent "Refactor the login handler to use the new database connector"

[RTK-Diff]  auth.ts      → BEHAVIORAL  (2 functions modified)
[RTK-Diff]  db.ts        → REFACTOR    (connector swap)
[Indexer]   Context coverage: 100% (both files indexed)

[Intent Guard] Confidence: 0.71 ⚠️
  ↳ Intent matched: Database Refactor ✅
  ↳ Side effect:    Password validation logic DELETED in auth.ts:42 ❌

⚠️  Warning: Logic for 'Password Validation' was removed. This was NOT in your stated intent.

Proceed? [y/N]:
```

---

## 7. Integration with Existing Stack

### Indexing LLM Read
The Intent Guard queries the indexer before each validation to ensure the LLM has seen the **latest committed version** of every file it is modifying. If a file is stale in the index, the guard downgrades its confidence score and emits a context warning.

### Env Encryption Layer
Any diff touching a code block that reads, writes, or references encrypted environment variables or secrets will automatically be flagged as a **Sensitive File**. The tool will escalate the warning and require explicit confirmation before approving.

```
[Safety Guard] ⛔ SENSITIVE FILE DETECTED
  ↳ db.ts references: process.env.DB_SECRET_KEY (encrypted)
  ↳ Require explicit approval: type 'CONFIRM' to proceed
```

---

## 8. CLI Interface

```
Usage:
  semantic-guard check [flags]
  semantic-guard config
  semantic-guard index sync

Flags:
  --intent   <string>   The developer's intent / original prompt
  --diff     <file>     Path to a pre-generated diff file (default: git diff HEAD)
  --model    <string>   Override local LLM model (default: from config)
  --lang     <string>   Comma-separated language filter (rust,ts,py,go)
  --no-llm              Skip Intent Guard, run AST analysis only
  --yes                 Auto-approve non-sensitive, high-confidence changes
  --json                Output report as JSON

Config file: ~/.semantic-guard/config.toml
```

---

## 9. Non-Goals (v1)

- No remote/cloud LLM calls — all inference runs locally
- No UI or IDE plugin (CLI only in v1; VS Code extension is post-launch)
- No auto-revert — the guard warns but never automatically undoes a change
- No support for binary file diffs