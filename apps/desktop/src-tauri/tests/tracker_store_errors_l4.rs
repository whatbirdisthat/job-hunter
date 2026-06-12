//! L4 — the reachable `StoreError` arms of the tracker persistence adapter (R-STO-1).
//! The load-of-corrupt-file (Serde) and the save-to-an-unwritable-path (IO) arms ARE
//! observable error VALUES a caller can hit, so they are exercised here (mirroring the
//! existing items' "every observable error value is tested" discipline). The infallible
//! serialize arm and the defensive write/rename I/O arms are the documented P-COV-1/P-COV-2
//! classes (doc/COVERAGE.md).

use aa_desktop::tracker_store::{JsonFileStore, StoreError, TrackerStore};
use aa_tracker::TrackerDoc;

fn temp_path(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "aa-tracker-storeerr-{tag}-{}-{}.json",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ))
}

#[test]
fn load_of_corrupt_file_is_a_serde_error() {
    // R-STO-1 — a corrupt on-disk document surfaces as StoreError::Serde, not a panic.
    let path = temp_path("corrupt");
    std::fs::write(&path, b"{ this is not valid tracker json").unwrap();
    let store = JsonFileStore::new(&path);
    let err = store.load().unwrap_err();
    assert!(matches!(err, StoreError::Serde(_)));
    assert!(err.to_string().contains("serialize/parse"));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn save_to_unwritable_path_is_an_io_error() {
    // R-STO-1 — saving under a path whose parent is a FILE (not a dir) cannot write the temp
    // sibling → StoreError::Io, surfaced (not a panic).
    let file_as_dir = temp_path("notadir");
    std::fs::write(&file_as_dir, b"i am a file").unwrap();
    // Use the file as if it were a directory: <file>/doc.json — the write must fail.
    let bad = file_as_dir.join("doc.json");
    let store = JsonFileStore::new(&bad);
    let doc = TrackerDoc {
        applications: vec![],
        contacts: vec![],
    };
    let err = store.save(&doc).unwrap_err();
    assert!(matches!(err, StoreError::Io(_)));
    assert!(err.to_string().contains("io"));
    let _ = std::fs::remove_file(&file_as_dir);
}

#[test]
fn missing_file_loads_an_empty_doc() {
    // First run: a missing file is an empty document, not an error (R-STO-1).
    let path = temp_path("missing");
    let store = JsonFileStore::new(&path);
    let doc = store.load().unwrap();
    assert!(doc.applications.is_empty() && doc.contacts.is_empty());
}

/// A scratch private dir for the permission/temp-location tests (Finding 1). Mirrors
/// `temp_path` but yields a fresh DIRECTORY the store creates `<dir>/tracker.json` under.
fn temp_dir(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "aa-tracker-permtest-{tag}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ))
}

#[cfg(unix)]
#[test]
fn saved_file_is_owner_only_0600_and_dir_0700() {
    // Finding 1 (a): the plaintext-PII tracker file is owner-only `0600` and its private dir
    // `0700` after a save — never the world-readable shared-temp posture.
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir("perms");
    let path = dir.join("tracker.json");
    let store = JsonFileStore::new(&path);
    let doc = TrackerDoc {
        applications: vec![],
        contacts: vec![],
    };
    store.save(&doc).unwrap();

    let file_mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(file_mode, 0o600, "tracker file must be owner-only 0600");
    let dir_mode = std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(dir_mode, 0o700, "tracker dir must be owner-only 0700");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn temp_sibling_lives_in_the_store_dir_not_shared_temp() {
    // Finding 1 (b): the atomic-write temp sibling is created INSIDE the store's own private
    // dir (so the rename is same-filesystem and the temp is never in a world-writable dir).
    // We prove it by counting the entries left in the dir during/after a save: the store dir
    // is otherwise empty, so any temp must appear there — and the shared temp dir gains no
    // `aa-tracker.json`-style sibling.
    let dir = temp_dir("tmploc");
    let path = dir.join("tracker.json");
    let store = JsonFileStore::new(&path);
    let doc = TrackerDoc {
        applications: vec![],
        contacts: vec![],
    };
    store.save(&doc).unwrap();

    // After the rename, the ONLY entry in the private dir is the live tracker.json — the temp
    // sibling was created here (not in the shared temp dir) and renamed over the target.
    let entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries, vec![std::ffi::OsString::from("tracker.json")]);

    // The legacy world-readable shared path is NOT created by a save into a private dir.
    assert!(!std::env::temp_dir().join("aa-tracker.json").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn default_path_is_per_user_app_scoped_not_shared_temp_file() {
    // Finding 1: the DEFAULT store path is a per-user, app-scoped `job-hunter/tracker.json`
    // under the data dir — never the old single shared `<tmp>/aa-tracker.json`.
    let p = JsonFileStore::default_path();
    assert_eq!(p.file_name().unwrap(), "tracker.json");
    assert_eq!(
        p.parent().unwrap().file_name().unwrap(),
        "job-hunter",
        "the file lives under an app-scoped job-hunter/ subdir"
    );
    assert_ne!(
        p,
        std::env::temp_dir().join("aa-tracker.json"),
        "must not be the legacy shared world-readable temp file"
    );
}
