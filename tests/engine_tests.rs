use vibe_guard::config::Config;
use vibe_guard::engine::classify::{build_from_patch, classify_sources};
use vibe_guard::engine::{ChangeType, StructuredDiff, SymbolChange, SymbolKind};
use vibe_guard::git::FileStatus;
use vibe_guard::intent::heuristic::HeuristicProvider;
use vibe_guard::intent::provider::LlmProvider;
use vibe_guard::summary;

// ---- AST classification ---------------------------------------------------

#[test]
fn body_change_is_behavioral() {
    let before = "function add(a, b) {\n  return a + b;\n}\n";
    let after = "function add(a, b) {\n  return a - b;\n}\n";
    let d = classify_sources("math.ts", FileStatus::Modified, before, after).unwrap();
    assert_eq!(d.change_type, ChangeType::Behavioral);
    assert!(d
        .symbols
        .iter()
        .any(|s| s.name == "add" && s.kind == SymbolKind::Modified));
}

#[test]
fn comment_and_formatting_only_is_boilerplate() {
    let before = "function add(a, b) {\n  return a + b;\n}\n";
    let after = "// adds two numbers\nfunction add(a,   b) {\n    return a + b;\n}\n";
    let d = classify_sources("math.ts", FileStatus::Modified, before, after).unwrap();
    assert_eq!(d.change_type, ChangeType::Boilerplate);
}

#[test]
fn rename_with_same_body_is_refactor() {
    let before = "function add(a, b) {\n  return a + b;\n}\n";
    let after = "function sum(a, b) {\n  return a + b;\n}\n";
    let d = classify_sources("math.ts", FileStatus::Modified, before, after).unwrap();
    assert_eq!(d.change_type, ChangeType::Refactor);
    assert!(d.symbols.iter().all(|s| s.kind == SymbolKind::Renamed));
}

#[test]
fn python_body_change_is_behavioral() {
    let before = "def greet(name):\n    return 'hi ' + name\n";
    let after = "def greet(name):\n    return 'hello ' + name\n";
    let d = classify_sources("app.py", FileStatus::Modified, before, after).unwrap();
    assert_eq!(d.change_type, ChangeType::Behavioral);
}

#[test]
fn rust_added_function_is_behavioral() {
    let before = "fn a() -> i32 { 1 }\n";
    let after = "fn a() -> i32 { 1 }\nfn b() -> i32 { 2 }\n";
    let d = classify_sources("lib.rs", FileStatus::Modified, before, after).unwrap();
    assert_eq!(d.change_type, ChangeType::Behavioral);
    assert!(d
        .symbols
        .iter()
        .any(|s| s.name == "b" && s.kind == SymbolKind::Added));
}

#[test]
fn go_method_body_change_is_behavioral() {
    let before = "package main\nfunc (s *S) Run() int { return 1 }\n";
    let after = "package main\nfunc (s *S) Run() int { return 2 }\n";
    let d = classify_sources("main.go", FileStatus::Modified, before, after).unwrap();
    assert_eq!(d.change_type, ChangeType::Behavioral);
}

#[test]
fn unknown_language_falls_back_to_text_diff() {
    let d = classify_sources("notes.txt", FileStatus::Modified, "a\n", "a\nb\n").unwrap();
    assert_eq!(d.change_type, ChangeType::Behavioral);
    assert!(d.language.is_none());
}

// ---- Patch mode -----------------------------------------------------------

#[test]
fn patch_mode_detects_behavioral_vs_boilerplate() {
    let patch = "\
diff --git a/a.ts b/a.ts
--- a/a.ts
+++ b/a.ts
@@ -1,1 +1,1 @@
-  return a + b;
+  return a - b;
diff --git a/b.ts b/b.ts
--- a/b.ts
+++ b/b.ts
@@ -1,0 +1,1 @@
+// just a comment
";
    let d = build_from_patch(patch, &[]).unwrap();
    let a = d.files.iter().find(|f| f.path == "a.ts").unwrap();
    let b = d.files.iter().find(|f| f.path == "b.ts").unwrap();
    assert_eq!(a.change_type, ChangeType::Behavioral);
    assert_eq!(b.change_type, ChangeType::Boilerplate);
}

// ---- Heuristic Intent Guard ----------------------------------------------

fn diff_with_removed(name: &str) -> StructuredDiff {
    use vibe_guard::engine::FileDiff;
    StructuredDiff {
        files: vec![FileDiff {
            path: "auth.ts".to_string(),
            language: None,
            status: FileStatus::Modified,
            change_type: ChangeType::Behavioral,
            symbols: vec![SymbolChange {
                name: name.to_string(),
                kind: SymbolKind::Removed,
                line: Some(42),
            }],
            detail: "1 symbol removed".to_string(),
        }],
    }
}

#[test]
fn heuristic_flags_removed_security_symbol() {
    let cfg = Config::default();
    let provider = HeuristicProvider::new(&cfg);
    let diff = diff_with_removed("validatePassword");
    let report = provider
        .evaluate(
            "Refactor the login handler to use the new database connector",
            &diff,
        )
        .unwrap();
    assert!(
        !report.side_effects.is_empty(),
        "expected a side-effect warning"
    );
    assert!(
        !report.intent_match,
        "removal of validation should not match intent"
    );
}

#[test]
fn heuristic_no_false_alarm_on_benign_change() {
    let cfg = Config::default();
    let provider = HeuristicProvider::new(&cfg);
    let diff = diff_with_removed("formatCurrency");
    let report = provider
        .evaluate("update currency formatting helper", &diff)
        .unwrap();
    assert!(report.side_effects.is_empty());
}

// ---- Summary --------------------------------------------------------------

#[test]
fn summary_reports_logic_update() {
    let before = "function add(a, b) {\n  return a + b;\n}\n";
    let after = "function add(a, b) {\n  return a * b;\n}\n";
    let d = classify_sources("math.ts", FileStatus::Modified, before, after).unwrap();
    let diff = StructuredDiff { files: vec![d] };
    let line = summary::vibe_line(&diff);
    assert!(line.contains("Logic Update"), "got: {line}");
}
