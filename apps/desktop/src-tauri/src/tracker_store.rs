//! The `TrackerStore` persistence seam + the `JsonFileStore` concrete impl (item #5,
//! R-STO-1/2/3). This lives in the COMMAND layer (not in `aa-tracker`) so the cores stay
//! IO-free — only this thin adapter touches the filesystem.
//!
//! Per `doc/design/item-5-storage-decision.md`: a single on-device JSON document written
//! ATOMICALLY (serialize → temp sibling IN THE SAME PRIVATE DIR → `rename` over the target),
//! so a crash mid-write never corrupts the live file. A future `SqlCipherStore` implements
//! the SAME trait — a localized swap behind this seam, not a rewrite.
//!
//! Security (item #5 review, Finding 1): the default store lives in a PER-USER, app-scoped
//! data dir (NOT the shared world-readable temp dir), the dir is `0700` and the file `0600`
//! on Unix, and the atomic-write temp sibling is created inside that same private dir with a
//! NON-PREDICTABLE name (pid + a monotonic nanos counter) so there is no symlink-follow /
//! predictable-clobber vector. This is the only place a wall-clock is read — it is IO-layer
//! freshness for a temp filename, not core logic.

use aa_tracker::TrackerDoc;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// A persistence error from the store (R-STO-1) — typed, carried to the command layer.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("tracker store io: {0}")]
    Io(String),
    #[error("tracker store serialize/parse: {0}")]
    Serde(String),
}

/// The persistence port (R-STO-1). The cores never see this — only the command layer does.
pub trait TrackerStore {
    fn load(&self) -> Result<TrackerDoc, StoreError>;
    fn save(&self, doc: &TrackerDoc) -> Result<(), StoreError>;
}

/// One JSON document, written atomically (temp + rename), at an on-device path
/// (OS app-data dir in prod; an injected temp dir in tests).
pub struct JsonFileStore {
    path: PathBuf,
}

impl JsonFileStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        JsonFileStore { path: path.into() }
    }

    /// The PER-USER, app-scoped default tracker path (Finding 1). Deterministic, std-only
    /// (no `dirs`/`directories` dep — the storage decision forbids new deps): prefer
    /// `$XDG_DATA_HOME`, else `$HOME/.local/share`; fall back to a per-user-isolated subdir
    /// under the shared temp dir ONLY when no home is resolvable (so even the fallback is
    /// NOT a single shared world-readable file). The document is `<data_dir>/job-hunter/tracker.json`.
    pub fn default_path() -> PathBuf {
        // Read the env at the boundary, then delegate ALL branch logic to the pure
        // `resolve_default_path` so it is unit-testable without racy env mutation.
        resolve_default_path(
            std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
            std::env::var_os("HOME").map(PathBuf::from),
            std::env::temp_dir(),
            current_user_tag(),
        )
    }

    /// The directory the document (and its atomic-write temp sibling) live in.
    fn dir(&self) -> Option<&Path> {
        self.path.parent()
    }

    /// A NON-PREDICTABLE temp sibling INSIDE the store's own private dir (Finding 1): pid +
    /// a monotonic nanos counter so the name cannot be pre-created/symlinked by an attacker,
    /// and the rename is same-filesystem (the temp is never in a world-writable dir).
    fn tmp_path(&self) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();
        let name = match self.path.file_name().and_then(|n| n.to_str()) {
            Some(base) => format!(".{base}.{pid}.{nanos}.tmp"),
            None => format!(".tracker.{pid}.{nanos}.tmp"),
        };
        match self.dir() {
            Some(dir) => dir.join(name),
            None => PathBuf::from(name),
        }
    }
}

/// Pure resolver for [`JsonFileStore::default_path`] (Finding 1) — env values in, path out, no
/// I/O, so every branch is unit-testable without racy env mutation. Prefer a non-empty
/// `$XDG_DATA_HOME`, else a non-empty `$HOME/.local/share`, else a per-user-isolated subdir
/// under `temp_dir` (never a single shared file). The document is `<data_dir>/job-hunter/tracker.json`.
fn resolve_default_path(
    xdg_data_home: Option<PathBuf>,
    home: Option<PathBuf>,
    temp_dir: PathBuf,
    user_tag: String,
) -> PathBuf {
    let data_dir = xdg_data_home
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| {
            home.filter(|p| !p.as_os_str().is_empty())
                .map(|h| h.join(".local").join("share"))
        })
        .unwrap_or_else(|| temp_dir.join(format!("aa-tracker-{user_tag}")));
    data_dir.join("job-hunter").join("tracker.json")
}

/// A per-user tag for the no-home temp fallback (Finding 1): the `USER`/`USERNAME` env, else
/// `"shared"`. Std-only — no `dirs`/`users`/`libc` dep, no `unsafe`. This isolates the
/// fallback per-user so it is never a single shared world-readable file; the `0700` dir mode
/// set on Unix is the real confidentiality control.
fn current_user_tag() -> String {
    user_tag_from(std::env::var("USER").ok(), std::env::var("USERNAME").ok())
}

/// Pure resolver for [`current_user_tag`] — the first non-empty of `USER`/`USERNAME`, else
/// `"shared"`. Split out so every branch is unit-testable without env mutation.
fn user_tag_from(user: Option<String>, username: Option<String>) -> String {
    user.filter(|u| !u.is_empty())
        .or(username.filter(|u| !u.is_empty()))
        .unwrap_or_else(|| "shared".to_string())
}

impl TrackerStore for JsonFileStore {
    fn load(&self) -> Result<TrackerDoc, StoreError> {
        // A missing file is an empty document (first run), not an error — the tracker
        // starts blank and the first save creates the file.
        if !Path::new(&self.path).exists() {
            return Ok(TrackerDoc {
                applications: vec![],
                contacts: vec![],
            });
        }
        let text =
            std::fs::read_to_string(&self.path).map_err(|e| StoreError::Io(e.to_string()))?;
        serde_json::from_str(&text).map_err(|e| StoreError::Serde(e.to_string()))
    }

    fn save(&self, doc: &TrackerDoc) -> Result<(), StoreError> {
        // R-STO-2 atomic write: serialize → write a NON-PREDICTABLE temp sibling inside the
        // store's own PRIVATE dir → rename over the target. A crash before the rename leaves
        // the prior good file intact, and the rename is same-filesystem (Finding 1).
        let json =
            serde_json::to_string_pretty(doc).map_err(|e| StoreError::Serde(e.to_string()))?;

        // Ensure the parent dir exists. When WE create it, lock it to owner-only `0700` on
        // Unix before writing plaintext PII. We do NOT re-chmod a pre-existing dir: an injected
        // store dir (tests) or a shared parent is the host's to own — tightening it could fail
        // (EPERM) or clobber a deliberate mode. The default path's `job-hunter/` subdir is one
        // WE create, so it gets the `0700` treatment.
        if let Some(dir) = self.dir() {
            if !dir.as_os_str().is_empty() && !dir.exists() {
                std::fs::create_dir_all(dir).map_err(|e| StoreError::Io(e.to_string()))?;
                set_owner_only_dir(dir)?;
            }
        }

        let tmp = self.tmp_path();
        std::fs::write(&tmp, json.as_bytes()).map_err(|e| StoreError::Io(e.to_string()))?;
        // On Unix, lock the temp file to `0600` BEFORE the rename so the live file is never
        // momentarily group/other-readable (the rename preserves the source mode).
        set_owner_only_file(&tmp)?;
        std::fs::rename(&tmp, &self.path).map_err(|e| StoreError::Io(e.to_string()))?;
        Ok(())
    }
}

/// Set a directory to owner-only `0700` (Unix). A no-op on non-Unix (no std mode model).
#[cfg(unix)]
fn set_owner_only_dir(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| StoreError::Io(e.to_string()))
}

#[cfg(not(unix))]
fn set_owner_only_dir(_path: &Path) -> Result<(), StoreError> {
    Ok(())
}

/// Set a file to owner-only `0600` (Unix). A no-op on non-Unix (no std mode model).
#[cfg(unix)]
fn set_owner_only_file(path: &Path) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| StoreError::Io(e.to_string()))
}

#[cfg(not(unix))]
fn set_owner_only_file(_path: &Path) -> Result<(), StoreError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── resolve_default_path: every branch, no env mutation (Finding 1) ───────────────
    #[test]
    fn resolve_prefers_xdg_data_home() {
        let p = resolve_default_path(
            Some(PathBuf::from("/data/xdg")),
            Some(PathBuf::from("/home/u")),
            PathBuf::from("/tmp"),
            "u".into(),
        );
        assert_eq!(p, PathBuf::from("/data/xdg/job-hunter/tracker.json"));
    }

    #[test]
    fn resolve_falls_back_to_home_local_share_when_xdg_absent_or_empty() {
        // XDG absent → HOME/.local/share.
        let p = resolve_default_path(
            None,
            Some(PathBuf::from("/home/u")),
            PathBuf::from("/tmp"),
            "u".into(),
        );
        assert_eq!(
            p,
            PathBuf::from("/home/u/.local/share/job-hunter/tracker.json")
        );
        // XDG present but EMPTY → still HOME (the empty-string filter arm).
        let p2 = resolve_default_path(
            Some(PathBuf::from("")),
            Some(PathBuf::from("/home/u")),
            PathBuf::from("/tmp"),
            "u".into(),
        );
        assert_eq!(p2, p);
    }

    #[test]
    fn resolve_falls_back_to_per_user_temp_when_no_home() {
        // Neither XDG nor HOME → per-user-isolated temp subdir (never a single shared file).
        let p = resolve_default_path(None, None, PathBuf::from("/tmp"), "alice".into());
        assert_eq!(
            p,
            PathBuf::from("/tmp/aa-tracker-alice/job-hunter/tracker.json")
        );
        // An empty HOME is treated the same as absent (the empty-string filter arm).
        let p2 = resolve_default_path(
            None,
            Some(PathBuf::from("")),
            PathBuf::from("/tmp"),
            "alice".into(),
        );
        assert_eq!(p2, p);
    }

    // ── user_tag_from: every branch, no env mutation ──────────────────────────────────
    #[test]
    fn user_tag_prefers_user_then_username_then_shared() {
        assert_eq!(
            user_tag_from(Some("alice".into()), Some("bob".into())),
            "alice"
        );
        // USER empty → USERNAME.
        assert_eq!(user_tag_from(Some("".into()), Some("bob".into())), "bob");
        // both absent → "shared".
        assert_eq!(user_tag_from(None, None), "shared");
        // USER absent, USERNAME empty → "shared".
        assert_eq!(user_tag_from(None, Some("".into())), "shared");
    }

    // ── tmp_path edge arms: a path with no filename / no parent (Finding 1) ───────────
    #[test]
    fn tmp_path_handles_path_without_a_filename() {
        // "/" has no file_name() → the synthetic ".tracker.*" base; parent is Some("/").
        let store = JsonFileStore::new("/");
        let tmp = store.tmp_path();
        let name = tmp.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with(".tracker.") && name.ends_with(".tmp"));
    }

    #[test]
    fn tmp_path_handles_empty_path_without_a_parent() {
        // An empty path has neither a file_name() nor a parent() → both None arms.
        let store = JsonFileStore::new("");
        let tmp = store.tmp_path();
        let name = tmp.to_str().unwrap();
        assert!(name.starts_with(".tracker.") && name.ends_with(".tmp"));
        // No parent component was prepended.
        assert!(tmp.parent().unwrap().as_os_str().is_empty());
    }

    // ── the current_user_tag wrapper itself (so the boundary read is exercised) ───────
    #[test]
    fn current_user_tag_is_non_empty() {
        assert!(!current_user_tag().is_empty());
    }
}
