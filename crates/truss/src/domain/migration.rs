//! Domain values for `truss migrate` (decision record D-06, D-10, D-11, D-14).
//!
//! Migration crosses the three ownership domains established by decision 0008
//! under one immutable plan, so the plan and its inventory live here: pure
//! data with no filesystem, framework, or infrastructure dependency.
//! `MigrationPlan` is owned data passed by shared reference, and the port
//! returns a report; nothing in the execution path mutates a plan.
//!
//! The path vocabulary below is structural, not textual. It names the legacy
//! roots, the accepted destinations, the canonical relative ignore rules, and
//! the one token rewriter the accepted contract allows (D-11).

use super::{ContentHash, RelativePath};
use std::collections::BTreeSet;

/// The installed tree's root directory name before decision 0008.
pub const LEGACY_CORE_ROOT: &str = ".truss-core";
/// The installed tree's root directory name from decision 0008.
pub const NEW_CORE_ROOT: &str = ".truss/core";
/// The legacy delivery evidence root.
pub const LEGACY_DELIVERY_ROOT: &str = ".delivery";
/// The legacy delivery dispatch root.
pub const LEGACY_DISPATCH_ROOT: &str = ".delivery-dispatch";
/// The project-owned authority namespace.
pub const AUTHORITY_ROOT: &str = ".truss/authority";
/// The delivery working-memory namespace.
pub const DELIVERY_NAMESPACE: &str = ".truss/delivery";
/// The retained migration backup root.
pub const BACKUP_ROOT: &str = ".truss-migration-backup";
/// The stable apply lock, outside every moving root (D-08).
pub const BACKUP_LOCK: &str = "migration.lock";
/// The per-transaction journal file name.
pub const JOURNAL_FILE: &str = "migration.json";
/// The deterministic backup path template preview prints (D-07).
pub const BACKUP_TIMESTAMP_PLACEHOLDER: &str = "<UTC-timestamp>";

/// The optional mixed entrypoint files (D-03).
pub const ENTRYPOINTS: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];
/// The managed block markers every installed entrypoint carries.
pub const TRUSS_BEGIN: &str = "<!-- TRUSS:BEGIN -->";
/// The closing managed block marker.
pub const TRUSS_END: &str = "<!-- TRUSS:END -->";

/// The three recognized legacy roots, in report order.
pub const LEGACY_ROOTS: [&str; 3] = [LEGACY_CORE_ROOT, LEGACY_DELIVERY_ROOT, LEGACY_DISPATCH_ROOT];

/// The four managed relative integration rules plus the owner-approved fifth
/// (ADR 0008 amendment, D-01). Never root-anchored.
pub const INTEGRATION_IGNORE_RULES: [&str; 5] = [
    ".truss/authority/",
    ".truss/delivery/",
    ".truss/core/bin/truss",
    ".truss/core/bin/truss.exe",
    ".truss-migration-backup/",
];

/// A local-only consumer keeps one whole-tree rule plus the retained backup
/// rule (ADR 0008 §4, amended by D-01).
pub const LOCAL_ONLY_IGNORE_RULES: [&str; 2] = [".truss/", ".truss-migration-backup/"];

/// The stable state vocabulary (BR-24, D-14). Exit 0 covers only `ready`,
/// `already_migrated`, and `migrated`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationState {
    Ready,
    AlreadyMigrated,
    Blocked,
    Migrated,
    RolledBack,
    RollbackFailed,
    RecoveryRequired,
}

impl MigrationState {
    pub const ALL: [Self; 7] = [
        Self::Ready,
        Self::AlreadyMigrated,
        Self::Blocked,
        Self::Migrated,
        Self::RolledBack,
        Self::RollbackFailed,
        Self::RecoveryRequired,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::AlreadyMigrated => "already_migrated",
            Self::Blocked => "blocked",
            Self::Migrated => "migrated",
            Self::RolledBack => "rolled_back",
            Self::RollbackFailed => "rollback_failed",
            Self::RecoveryRequired => "recovery_required",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Ready | Self::AlreadyMigrated | Self::Migrated => 0,
            Self::Blocked | Self::RolledBack | Self::RollbackFailed | Self::RecoveryRequired => 1,
        }
    }
}

/// Stable snake-case refusal and failure codes (D-14). Human detail lives in
/// `evidence`, never in the code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MigrationReason {
    NotInstalled,
    UnknownDocument,
    AmbiguousRunKey,
    InvalidMarkers,
    PendingSession,
    UnsafeSymlink,
    Collision,
    UnsupportedApplyPlatform,
    RecoveryConflict,
    MalformedJournal,
    MultipleJournals,
    RepoMismatch,
    InvalidState,
    UnknownSchema,
    NonUtf8Document,
    StaleInventory,
    UnknownLegacyEntry,
    UnsafePath,
}

impl MigrationReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotInstalled => "not_installed",
            Self::UnknownDocument => "unknown_document",
            Self::AmbiguousRunKey => "ambiguous_run_key",
            Self::InvalidMarkers => "invalid_markers",
            Self::PendingSession => "pending_session",
            Self::UnsafeSymlink => "unsafe_symlink",
            Self::Collision => "collision",
            Self::UnsupportedApplyPlatform => "unsupported_apply_platform",
            Self::RecoveryConflict => "recovery_conflict",
            Self::MalformedJournal => "malformed_journal",
            Self::MultipleJournals => "multiple_journals",
            Self::RepoMismatch => "repository_mismatch",
            Self::InvalidState => "invalid_state",
            Self::UnknownSchema => "unknown_schema",
            Self::NonUtf8Document => "non_utf8_document",
            Self::StaleInventory => "stale_inventory",
            Self::UnknownLegacyEntry => "unknown_legacy_entry",
            Self::UnsafePath => "unsafe_path",
        }
    }
}

/// What the migration does at one destination path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationKind {
    /// The destination was absent; the migration creates it.
    Create,
    /// The destination existed with the intended bytes; the migration writes
    /// nothing and the file is idempotent.
    Preserve,
    /// The destination was absent or held different bytes; the migration
    /// writes the intended bytes.
    Replace,
}

impl OperationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Preserve => "preserve",
            Self::Replace => "replace",
        }
    }
}

/// One planned file at one destination.
///
/// `source` names the legacy file the bytes come from when the content is a
/// byte-for-byte move; `content` carries the bytes when the plan synthesizes
/// them (a state-record path rewrite, an authority-document token rewrite, or
/// an entrypoint block replacement).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationOperation {
    pub source: Option<String>,
    pub destination: String,
    pub kind: OperationKind,
    pub source_hash: Option<ContentHash>,
    pub before_hash: Option<ContentHash>,
    pub after_hash: ContentHash,
    pub content: Option<Vec<u8>>,
}

/// An integration original the migration rewrites outside the legacy roots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegrationKind {
    Entrypoint,
    Ignore,
}

impl IntegrationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Entrypoint => "entrypoint",
            Self::Ignore => "ignore",
        }
    }
}

/// One planned integration write (entrypoint block or ignore repair).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntegrationOperation {
    pub path: String,
    pub kind: IntegrationKind,
    pub before_hash: Option<ContentHash>,
    pub after_hash: ContentHash,
    pub content: Vec<u8>,
}

/// Whether an inventoried legacy path is a file or a directory, so rollback
/// can restore an empty directory as faithfully as a file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventoryKind {
    File,
    Directory,
}

impl InventoryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
        }
    }
}

/// One inventoried legacy path with its size and content hash.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InventoryEntry {
    pub path: String,
    pub kind: InventoryKind,
    pub size: u64,
    pub hash: Option<ContentHash>,
}

/// The immutable migration plan: everything the transaction needs, and
/// nothing it may change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationPlan {
    pub repository: String,
    pub backup_template: String,
    pub run_key: Option<String>,
    pub legacy_roots: Vec<String>,
    pub inventory: Vec<InventoryEntry>,
    pub operations: Vec<MigrationOperation>,
    pub integration: Vec<IntegrationOperation>,
    pub evidence: Vec<String>,
}

/// The operator-visible outcome of one invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationReport {
    pub state: MigrationState,
    pub reason: Option<MigrationReason>,
    pub applied: bool,
    pub repository: String,
    pub backup_path: Option<String>,
    pub backup_path_template: String,
    pub transaction_id: Option<String>,
    pub run_key: Option<String>,
    pub legacy_roots: Vec<String>,
    pub operations: Vec<MigrationOperation>,
    pub conflicts: Vec<String>,
    pub evidence: Vec<String>,
}

/// One incomplete journal discovered during preflight (D-08, D-12).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncompleteJournal {
    pub transaction_id: String,
    pub phase: String,
    pub path: String,
}

/// The defect a malformed entrypoint presents.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntrypointDefect {
    MissingBegin,
    MissingEnd,
    DuplicateBegin,
    DuplicateEnd,
    OutOfOrder,
}

impl EntrypointDefect {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MissingBegin => "missing TRUSS:BEGIN",
            Self::MissingEnd => "missing TRUSS:END",
            Self::DuplicateBegin => "duplicate TRUSS:BEGIN",
            Self::DuplicateEnd => "duplicate TRUSS:END",
            Self::OutOfOrder => "TRUSS:END before TRUSS:BEGIN",
        }
    }
}

/// The deterministic preview backup path (D-07): a template, no timestamp.
pub fn backup_template(repository: &str) -> String {
    format!("{repository}/{BACKUP_ROOT}/{BACKUP_TIMESTAMP_PLACEHOLDER}/")
}

/// The `.truss/core` destination for a path inside the legacy core root.
///
/// The transform is structural: every `.truss-core/` path segment becomes
/// `.truss/core/`, so `base/.truss-core/docs/x` becomes
/// `base/.truss/core/docs/x` (D-10, BR-07).
pub fn core_destination(legacy: &str) -> String {
    legacy.replace(
        &format!("{LEGACY_CORE_ROOT}/"),
        &format!("{NEW_CORE_ROOT}/"),
    )
}

/// The `.truss/authority` destination for a recognized unmanaged document, or
/// `None` when the path is not a recognized authority document (BR-09).
///
/// Directory-name recognition alone is not classification: the caller checks
/// manifest membership first, so a managed README or template under
/// `.truss-core/docs/decisions/` stays under `.truss/core` (BR-10, REQ-015).
pub fn authority_destination(legacy: &str) -> Option<String> {
    let root = LEGACY_CORE_ROOT;
    if legacy == format!("{root}/TRUSS.md") {
        return Some(format!("{AUTHORITY_ROOT}/TRUSS.md"));
    }
    if legacy == format!("{root}/ARCHITECTURE.md") {
        return Some(format!("{AUTHORITY_ROOT}/ARCHITECTURE.md"));
    }
    let maps = [
        (
            format!("{root}/docs/decisions/"),
            format!("{AUTHORITY_ROOT}/decisions/"),
        ),
        (
            format!("{root}/docs/plans/active/"),
            format!("{AUTHORITY_ROOT}/plans/active/"),
        ),
        (
            format!("{root}/docs/plans/completed/"),
            format!("{AUTHORITY_ROOT}/plans/completed/"),
        ),
        (
            format!("{root}/docs/product/"),
            format!("{AUTHORITY_ROOT}/product/"),
        ),
    ];
    for (from, to) in maps {
        if let Some(rest) = legacy.strip_prefix(&from) {
            if !rest.is_empty() {
                return Some(format!("{to}{rest}"));
            }
        }
    }
    None
}

/// The evidence destination for a file under a legacy delivery root (D-02).
///
/// The run key must match the recognized key exactly, so a file under a
/// different first-level directory is never filed under a plausible but false
/// key.
pub fn delivery_destination(legacy_root: &str, run_key: &str, relative: &str) -> Option<String> {
    let rest = relative.strip_prefix(&format!("{legacy_root}/{run_key}/"))?;
    if rest.is_empty() {
        return None;
    }
    let channel = if legacy_root == LEGACY_DISPATCH_ROOT {
        "dispatch"
    } else {
        "evidence"
    };
    Some(format!(
        "{DELIVERY_NAMESPACE}/runs/{run_key}/{channel}/{rest}"
    ))
}

/// The delivery path rewrite tokens, longest prefix first so
/// `.delivery-dispatch/` never matches as `.delivery/` and a known run key
/// consumes its own segment before the generic prefix is considered.
fn rewrite_tokens_for(run_key: Option<&str>) -> Vec<(String, String)> {
    let mut tokens = vec![
        (
            ".truss/delivery-runs/".to_owned(),
            format!("{DELIVERY_NAMESPACE}/runs/"),
        ),
        (
            ".truss/authority/approvals/".to_owned(),
            format!("{DELIVERY_NAMESPACE}/approvals/"),
        ),
    ];
    if let Some(key) = run_key {
        tokens.push((
            format!("{LEGACY_DISPATCH_ROOT}/{key}/"),
            format!("{DELIVERY_NAMESPACE}/runs/{key}/dispatch/"),
        ));
        tokens.push((
            format!("{LEGACY_DELIVERY_ROOT}/{key}/"),
            format!("{DELIVERY_NAMESPACE}/runs/{key}/evidence/"),
        ));
    }
    tokens.push((
        format!("{LEGACY_DISPATCH_ROOT}/"),
        format!("{DELIVERY_NAMESPACE}/runs/dispatch/"),
    ));
    tokens.push((
        format!("{LEGACY_DELIVERY_ROOT}/"),
        format!("{DELIVERY_NAMESPACE}/runs/evidence/"),
    ));
    tokens.push((format!("{LEGACY_CORE_ROOT}/"), format!("{NEW_CORE_ROOT}/")));
    tokens
}

/// Rewrite only path-shaped structural tokens inside one migrated document
/// (D-11, BR-16). A token is rewritten only when the prefix starts at a token
/// boundary, so the substring inside a word is left alone, and only the
/// recognized prefixes are changed; there is no repository-wide search and
/// replace.
pub fn rewrite_document_tokens(text: &str, run_key: Option<&str>) -> String {
    let tokens = rewrite_tokens_for(run_key);
    let bytes = text.as_bytes();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        let matched = tokens
            .iter()
            .find(|(from, _)| bytes[index..].starts_with(from.as_bytes()));
        let Some((from, to)) = matched else {
            let width = next_char_width(text, index);
            output.push_str(&text[index..index + width]);
            index += width;
            continue;
        };
        if !at_token_boundary(text, index) {
            let width = next_char_width(text, index);
            output.push_str(&text[index..index + width]);
            index += width;
            continue;
        }
        output.push_str(to);
        index += from.len();
    }
    output
}

fn next_char_width(text: &str, index: usize) -> usize {
    text[index..]
        .chars()
        .next()
        .map(char::len_utf8)
        .unwrap_or(1)
}

fn at_token_boundary(text: &str, index: usize) -> bool {
    match text[..index].chars().next_back() {
        None => true,
        Some(previous) => {
            !(previous.is_ascii_alphanumeric() || matches!(previous, '.' | '-' | '_' | '/'))
        }
    }
}

/// Split one entrypoint file around its single ordered managed block.
///
/// Returns the bytes before the block and the bytes after it, so the caller
/// composes `before + canonical + after` and every byte outside the markers is
/// preserved exactly (REQ-021). Zero, duplicate, or out-of-order markers are a
/// defect, never a repair (D-03, REQ-022).
pub fn split_entrypoint(bytes: &[u8]) -> Result<(Vec<u8>, Vec<u8>), EntrypointDefect> {
    let begins = find_all(bytes, TRUSS_BEGIN.as_bytes());
    let ends = find_all(bytes, TRUSS_END.as_bytes());
    match (begins.len(), ends.len()) {
        (0, _) => return Err(EntrypointDefect::MissingBegin),
        (_, 0) => return Err(EntrypointDefect::MissingEnd),
        (1, 1) => {}
        (n, _) if n > 1 => return Err(EntrypointDefect::DuplicateBegin),
        (_, n) if n > 1 => return Err(EntrypointDefect::DuplicateEnd),
        _ => unreachable!(),
    }
    let begin = begins[0];
    let mut end = ends[0] + TRUSS_END.len();
    // The canonical block file ends with a newline after the closing marker,
    // so the managed region extends to the end of that marker's line. Without
    // this, every rewrite would append one extra newline.
    if bytes[end..].starts_with(b"\r\n") {
        end += 2;
    } else if bytes[end..].starts_with(b"\n") {
        end += 1;
    }
    if end <= begin {
        return Err(EntrypointDefect::OutOfOrder);
    }
    Ok((bytes[..begin].to_vec(), bytes[end..].to_vec()))
}

fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut found = Vec::new();
    if needle.is_empty() || haystack.len() < needle.len() {
        return found;
    }
    let mut index = 0;
    while index + needle.len() <= haystack.len() {
        if &haystack[index..index + needle.len()] == needle {
            found.push(index);
            index += needle.len();
        } else {
            index += 1;
        }
    }
    found
}

/// Compose the final bytes of one entrypoint from its preserved surroundings
/// and the compile-time canonical block.
pub fn compose_entrypoint(original: &[u8], canonical: &[u8]) -> Result<Vec<u8>, EntrypointDefect> {
    let (before, after) = split_entrypoint(original)?;
    let mut composed = before;
    composed.extend_from_slice(canonical);
    composed.extend_from_slice(&after);
    Ok(composed)
}

/// The `.truss/core` path a manifest path becomes, parsed as a domain path.
pub fn rewritten_path(path: &str) -> Result<RelativePath, String> {
    RelativePath::parse(core_destination(path)).map_err(|error| error.to_string())
}

/// The ordered, de-duplicated set of manifest member paths.
pub fn member_set(paths: impl IntoIterator<Item = String>) -> BTreeSet<String> {
    paths.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_destination_rewrites_every_structural_segment() {
        assert_eq!(
            core_destination(".truss-core/manifest.json"),
            ".truss/core/manifest.json"
        );
        assert_eq!(
            core_destination(".truss-core/base/.truss-core/docs/WORKFLOW.md"),
            ".truss/core/base/.truss/core/docs/WORKFLOW.md"
        );
        // A path that merely mentions the legacy name inside a filename is
        // still a structural segment, because the separator is part of the
        // prefix.
        assert_eq!(
            core_destination(".truss-core/docs/README.md"),
            ".truss/core/docs/README.md"
        );
    }

    #[test]
    fn authority_destination_is_allowlisted_and_scope_limited() {
        assert_eq!(
            authority_destination(".truss-core/TRUSS.md").as_deref(),
            Some(".truss/authority/TRUSS.md")
        );
        assert_eq!(
            authority_destination(".truss-core/ARCHITECTURE.md").as_deref(),
            Some(".truss/authority/ARCHITECTURE.md")
        );
        assert_eq!(
            authority_destination(".truss-core/docs/decisions/0001-x.md").as_deref(),
            Some(".truss/authority/decisions/0001-x.md")
        );
        assert_eq!(
            authority_destination(".truss-core/docs/plans/active/plan.md").as_deref(),
            Some(".truss/authority/plans/active/plan.md")
        );
        assert_eq!(
            authority_destination(".truss-core/docs/templates/custom.md"),
            None
        );
        assert_eq!(authority_destination(".truss-core/docs/stray.md"), None);
        // The recognized directory itself is not a document.
        assert_eq!(authority_destination(".truss-core/docs/decisions/"), None);
    }

    #[test]
    fn delivery_destination_files_evidence_and_dispatch_under_one_run() {
        assert_eq!(
            delivery_destination(LEGACY_DELIVERY_ROOT, "run-a", ".delivery/run-a/plan.md")
                .as_deref(),
            Some(".truss/delivery/runs/run-a/evidence/plan.md")
        );
        assert_eq!(
            delivery_destination(
                LEGACY_DISPATCH_ROOT,
                "run-a",
                ".delivery-dispatch/run-a/p.md"
            )
            .as_deref(),
            Some(".truss/delivery/runs/run-a/dispatch/p.md")
        );
        assert_eq!(
            delivery_destination(LEGACY_DELIVERY_ROOT, "run-a", ".delivery/other/plan.md"),
            None
        );
    }

    #[test]
    fn token_rewrite_skips_substrings_inside_words_and_unknown_paths() {
        let text = "see `.truss-core/docs/WORKFLOW.md` and x.truss-core/docs/y and plain prose";
        let rewritten = rewrite_document_tokens(text, None);
        assert!(rewritten.contains("`.truss/core/docs/WORKFLOW.md`"));
        assert!(
            rewritten.contains("x.truss-core/docs/y"),
            "a token inside a word is not path-shaped: {rewritten}"
        );
    }

    #[test]
    fn token_rewrite_maps_legacy_delivery_roots_and_pre0008_paths() {
        let text = ".delivery/run-a/plan.md .delivery-dispatch/run-a/p .truss/delivery-runs/r/x .truss/authority/approvals/r.md";
        let rewritten = rewrite_document_tokens(text, Some("run-a"));
        assert_eq!(
            rewritten,
            ".truss/delivery/runs/run-a/evidence/plan.md .truss/delivery/runs/run-a/dispatch/p .truss/delivery/runs/r/x .truss/delivery/approvals/r.md"
        );
    }

    #[test]
    fn entrypoint_split_preserves_every_outside_byte() {
        let original = b"# Mine\n\n<!-- TRUSS:BEGIN -->\nold\n<!-- TRUSS:END -->\ntail\n";
        let (before, after) = split_entrypoint(original).unwrap();
        assert_eq!(before, b"# Mine\n\n");
        assert_eq!(after, b"tail\n");
        let canonical = b"<!-- TRUSS:BEGIN -->\nnew\n<!-- TRUSS:END -->\n";
        let composed = compose_entrypoint(original, canonical).unwrap();
        assert_eq!(
            composed,
            b"# Mine\n\n<!-- TRUSS:BEGIN -->\nnew\n<!-- TRUSS:END -->\ntail\n"
        );
    }

    /// The composed canonical block is byte-identical to itself, so a second
    /// migration is idempotent and an equal block is never rewritten.
    #[test]
    fn entrypoint_composes_to_itself_for_the_canonical_bytes() {
        let canonical = b"<!-- TRUSS:BEGIN -->\nbody\n<!-- TRUSS:END -->\n";
        let original = b"# Agent Instructions\n\n<!-- TRUSS:BEGIN -->\nbody\n<!-- TRUSS:END -->\n";
        assert_eq!(compose_entrypoint(original, canonical).unwrap(), original);
    }

    #[test]
    fn entrypoint_marker_defects_are_named() {
        assert_eq!(
            split_entrypoint(b"no markers").unwrap_err(),
            EntrypointDefect::MissingBegin
        );
        assert_eq!(
            split_entrypoint(b"<!-- TRUSS:BEGIN -->\nno end").unwrap_err(),
            EntrypointDefect::MissingEnd
        );
        assert_eq!(
            split_entrypoint(b"<!-- TRUSS:BEGIN -->\n<!-- TRUSS:BEGIN -->\n<!-- TRUSS:END -->")
                .unwrap_err(),
            EntrypointDefect::DuplicateBegin
        );
        assert_eq!(
            split_entrypoint(b"<!-- TRUSS:BEGIN -->\n<!-- TRUSS:END -->\n<!-- TRUSS:END -->")
                .unwrap_err(),
            EntrypointDefect::DuplicateEnd
        );
        assert_eq!(
            split_entrypoint(b"<!-- TRUSS:END -->\n<!-- TRUSS:BEGIN -->").unwrap_err(),
            EntrypointDefect::OutOfOrder
        );
    }

    #[test]
    fn exit_contract_covers_only_the_three_success_states() {
        assert_eq!(MigrationState::Ready.exit_code(), 0);
        assert_eq!(MigrationState::AlreadyMigrated.exit_code(), 0);
        assert_eq!(MigrationState::Migrated.exit_code(), 0);
        for state in [
            MigrationState::Blocked,
            MigrationState::RolledBack,
            MigrationState::RollbackFailed,
            MigrationState::RecoveryRequired,
        ] {
            assert_eq!(state.exit_code(), 1, "{state:?} must be non-zero");
        }
    }
}
