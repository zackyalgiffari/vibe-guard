pub mod classify;

use crate::ast::Language;
use crate::git::FileStatus;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    /// Renaming/moving/restructuring with no behavioral change.
    Refactor,
    /// Logic updates: changed bodies, signatures, added/removed functions.
    Behavioral,
    /// Imports, formatting, whitespace, comments only.
    Boilerplate,
}

impl ChangeType {
    pub fn label(self) -> &'static str {
        match self {
            ChangeType::Refactor => "REFACTOR",
            ChangeType::Behavioral => "BEHAVIORAL",
            ChangeType::Boilerplate => "BOILERPLATE",
        }
    }
}

/// How a single named symbol changed between before/after.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SymbolKind {
    Added,
    Removed,
    Modified,
    Renamed,
}

#[derive(Debug, Clone, Serialize)]
pub struct SymbolChange {
    pub name: String,
    pub kind: SymbolKind,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileDiff {
    pub path: String,
    #[serde(serialize_with = "ser_lang")]
    pub language: Option<Language>,
    pub status: FileStatus,
    pub change_type: ChangeType,
    pub symbols: Vec<SymbolChange>,
    /// Short human note (e.g. "2 functions modified").
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructuredDiff {
    pub files: Vec<FileDiff>,
}

impl StructuredDiff {
    pub fn functional_files(&self) -> impl Iterator<Item = &FileDiff> {
        self.files
            .iter()
            .filter(|f| f.change_type != ChangeType::Boilerplate)
    }
}

fn ser_lang<S>(lang: &Option<Language>, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match lang {
        Some(l) => s.serialize_str(l.code()),
        None => s.serialize_none(),
    }
}
