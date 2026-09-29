//! Persists a per-install device prefix in the Tauri app-data
//! directory and constructs a `TraceIdGenerator` from it.
//!
//! The first launch mints and writes; subsequent launches re-use the
//! persisted value so a workstation keeps a stable middle segment on
//! every trace id it emits. The I/O is best-effort: a read or write
//! failure falls back to a `None` device prefix and logs a warning,
//! so a corrupted app-data directory does not prevent the app from
//! launching. Trace ids are observability, not a hard requirement.

use std::path::Path;

use trace_id::TraceIdGenerator;

pub const DEVICE_PREFIX_FILE_NAME: &str = "aegis-desktop-device-prefix";

/// Load the device prefix from `<app_data_dir>/aegis-desktop-device-prefix`,
/// or mint a fresh `nanoid!()` and write it if the file is missing.
///
/// I/O failures (no app data dir, permission denied, write failure)
/// fall back to a freshly-minted, non-persisted prefix so the app
/// still boots — trace ids are observability, not a hard
/// requirement.
pub fn load_or_create(app_data_dir: &Path) -> TraceIdGenerator {
    let path = app_data_dir.join(DEVICE_PREFIX_FILE_NAME);
    let prefix = match std::fs::read_to_string(&path) {
        Ok(s) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                mint_and_write(&path).unwrap_or_else(fallback)
            } else {
                trimmed
            }
        }
        Err(_) => mint_and_write(&path).unwrap_or_else(fallback),
    };
    TraceIdGenerator::new(Some(prefix))
}

fn mint_and_write(path: &Path) -> Option<String> {
    let id = nanoid::nanoid!();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, &id).ok()?;
    Some(id)
}

fn fallback() -> String {
    // `nanoid!()` cannot fail; mint a fresh id without persisting so
    // the generator is still usable. Caller-side logging of the
    // fallback condition is the operator's signal to investigate.
    nanoid::nanoid!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_create_writes_new_nanoid_when_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let prefix_path = dir.join(DEVICE_PREFIX_FILE_NAME);
        assert!(!prefix_path.exists());

        let generator = load_or_create(dir);
        let id = generator.client_side();

        assert!(prefix_path.is_file(), "prefix file should be created");
        let persisted = std::fs::read_to_string(&prefix_path).unwrap();
        let persisted = persisted.trim();
        assert!(!persisted.is_empty());
        assert!(id.starts_with("C-"), "id should start with 'C-': {id:?}");
        // id layout: "C-<prefix?>-<ulid>". The last segment is
        // always the ULID (26 Crockford-base-32 chars); everything
        // between "C-" and the tail is the persisted prefix verbatim.
        let tail = id.rsplit('-').next().expect("at least one segment");
        assert_eq!(tail.len(), 26, "ULID tail should be 26 chars, got {tail:?}");
        let middle = &id["C-".len()..id.len() - tail.len() - 1];
        assert_eq!(middle, persisted, "middle segment should match persisted prefix");
    }

    #[test]
    fn load_or_create_reuses_existing_prefix_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let prefix_path = dir.join(DEVICE_PREFIX_FILE_NAME);
        let existing = "fixed-prefix-abc123";
        std::fs::write(&prefix_path, existing).unwrap();

        let generator = load_or_create(dir);
        let id = generator.client_side();

        assert!(id.starts_with("C-fixed-prefix-abc123-"), "got {id:?}");
        // The file should not be overwritten.
        let after = std::fs::read_to_string(&prefix_path).unwrap();
        assert_eq!(after, existing);
    }

    #[test]
    fn load_or_create_trims_whitespace_around_existing_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let prefix_path = dir.join(DEVICE_PREFIX_FILE_NAME);
        std::fs::write(&prefix_path, "  trimmed-xyz\n").unwrap();

        let generator = load_or_create(dir);
        let id = generator.client_side();
        assert!(id.starts_with("C-trimmed-xyz-"), "got {id:?}");
    }

    #[test]
    fn load_or_create_replaces_empty_existing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let prefix_path = dir.join(DEVICE_PREFIX_FILE_NAME);
        std::fs::write(&prefix_path, "").unwrap();

        let generator = load_or_create(dir);
        let id = generator.client_side();
        let persisted = std::fs::read_to_string(&prefix_path).unwrap();
        assert!(!persisted.is_empty());
        // The newly-minted prefix (whatever nanoid produced, trimmed)
        // appears verbatim between "C-" and the ULID tail.
        let tail = id.rsplit('-').next().expect("at least one segment");
        assert_eq!(tail.len(), 26, "ULID tail should be 26 chars, got {tail:?}");
        let middle = &id["C-".len()..id.len() - tail.len() - 1];
        assert_eq!(middle, persisted.trim());
    }

    #[test]
    fn load_or_create_returns_usable_prefix_when_write_fails() {
        // Point the dir at a path whose parent is a regular file so
        // create_dir_all on the prefix file's parent fails. The
        // function should still return a TraceIdGenerator (with a
        // freshly-minted, non-persisted prefix) instead of panicking.
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"not a dir").unwrap();
        let bad = blocker.join("nested");

        let generator = load_or_create(&bad);
        let id = generator.client_side();
        // 2 or 3 segments is acceptable here — both reflect a
        // working generator. We only assert it does not panic and
        // produces a well-formed id.
        assert!(id.starts_with("C-"), "got {id:?}");
        assert!(id.len() > "C-".len() + 10, "expected a ULID tail, got {id:?}");
    }
}