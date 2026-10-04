//! The on-disk queue of batches that failed to submit.
//!
//! One file per batch, named for the batch id. Because the batch id
//! ends in a ULID, a reverse lexicographic listing of the directory
//! is exactly newest-first — the retry order the spec asks for,
//! with no mtime reads and no clock involved.
//!
//! The on-disk format is a JSON array of strings, not one-entry-per-
//! line. A formatted log event is a single line of JSON, but that
//! is a property of the *caller*, not a guarantee we make to the
//! file: an entry containing a literal newline would split into two
//! lines and silently become two entries on reload. JSON escapes
//! newlines, so the round trip is exact.

use std::fs;
use std::path::PathBuf;

use super::sender::SubmitterError;

/// The queue of batches awaiting a successful submit.
pub(crate) struct PendingStore {
    dir: PathBuf,
    max_files: usize,
}

impl PendingStore {
    pub(crate) fn new(dir: PathBuf, max_files: usize) -> Self {
        Self { dir, max_files }
    }

    /// Create the pending directory. Idempotent.
    pub(crate) fn create_dir(&self) -> Result<(), SubmitterError> {
        fs::create_dir_all(&self.dir).map_err(|source| SubmitterError::CreateDir {
            dir: self.dir.clone(),
            source,
        })
    }

    /// Write `entries` under `batch_id`, then enforce the file cap.
    pub(crate) fn persist(
        &self,
        batch_id: &str,
        entries: &[String],
    ) -> Result<(), SubmitterError> {
        // `serde_json::Error` is not an `io::Error`, so the codec
        // failure is boxed into one to keep `SubmitterError::Persist`
        // carrying a single inner type.
        let encoded = serde_json::to_string(entries).map_err(|e| SubmitterError::Persist {
            batch_id: batch_id.to_string(),
            source: std::io::Error::other(e),
        })?;
        fs::write(self.dir.join(batch_id), encoded).map_err(|source| SubmitterError::Persist {
            batch_id: batch_id.to_string(),
            source,
        })?;
        self.enforce_cap();
        Ok(())
    }

    /// Batch ids, newest first.
    ///
    /// Only extension-less files count as batches. A batch id is
    /// `{device_id}-{ULID}`, and `mint_batch_id` folds the device id
    /// to `[A-Za-z0-9_-]` before appending a Crockford-base32 ULID —
    /// neither can contain a dot, so "no extension" is a reliable
    /// test. It keeps a stray file (an editor backup, a leftover
    /// temp) from being offered to the drain or counted against the
    /// file cap.
    pub(crate) fn list_newest_first(&self) -> Vec<String> {
        let Ok(dir) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut ids: Vec<String> = dir
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| !name.contains('.'))
            .collect();
        ids.sort();
        ids.reverse();
        ids
    }

    /// Read back a persisted batch.
    pub(crate) fn load(&self, batch_id: &str) -> Result<Vec<String>, SubmitterError> {
        let path = self.dir.join(batch_id);
        let raw = fs::read_to_string(&path).map_err(|source| SubmitterError::Persist {
            batch_id: batch_id.to_string(),
            source,
        })?;
        serde_json::from_str(&raw).map_err(|e| SubmitterError::Persist {
            batch_id: batch_id.to_string(),
            source: std::io::Error::other(e),
        })
    }

    /// Delete a persisted batch. Missing files are not an error — the
    /// caller's intent ("this batch is no longer pending") already
    /// holds.
    pub(crate) fn remove(&self, batch_id: &str) {
        let _ = fs::remove_file(self.dir.join(batch_id));
    }

    /// A second handle to the same directory. The store is just a
    /// path and a cap, so this is a plain clone.
    #[cfg(test)]
    pub(crate) fn clone_store(&self) -> PendingStore {
        PendingStore {
            dir: self.dir.clone(),
            max_files: self.max_files,
        }
    }

    /// Bound the backlog. Past `max_files`, the oldest ids are
    /// evicted, so a long outage degrades into "keep the most recent
    /// batches" instead of filling the disk.
    fn enforce_cap(&self) {
        let ids = self.list_newest_first();
        for stale in ids.into_iter().skip(self.max_files) {
            self.remove(&stale);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store(dir: &std::path::Path, max_files: usize) -> PendingStore {
        let s = PendingStore::new(dir.to_path_buf(), max_files);
        s.create_dir().unwrap();
        s
    }

    fn entries(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn create_dir_makes_the_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("pending");
        let s = PendingStore::new(dir.clone(), 4);
        s.create_dir().unwrap();
        assert!(dir.is_dir());
    }

    #[test]
    fn create_dir_reports_failure_for_a_blocked_path() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"not a dir").unwrap();
        let s = PendingStore::new(blocker.join("nested"), 4);
        let err = s.create_dir().unwrap_err();
        assert!(matches!(err, SubmitterError::CreateDir { .. }));
    }

    #[test]
    fn persist_names_the_file_after_the_batch_id() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 4);
        s.persist("dev-A01HQ", &entries(&["a", "b"])).unwrap();
        assert!(tmp.path().join("dev-A01HQ").is_file());
    }

    #[test]
    fn load_round_trips_entries_with_embedded_newlines() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 4);
        let tricky = entries(&["{\"level\":\"info\"}\n", "tail\r\nmore", "quo\"te"]);
        s.persist("dev-newlines", &tricky).unwrap();
        assert_eq!(s.load("dev-newlines").unwrap(), tricky);
    }

    #[test]
    fn list_newest_first_is_reverse_lexicographic() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 10);
        for id in ["dev-A01HQ", "dev-A05HQ", "dev-A03HQ"] {
            s.persist(id, &entries(&["x"])).unwrap();
        }
        assert_eq!(
            s.list_newest_first(),
            vec!["dev-A05HQ".to_string(), "dev-A03HQ".to_string(), "dev-A01HQ".to_string()]
        );
    }

    #[test]
    fn list_ignores_non_batch_files() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 10);
        s.persist("dev-A01HQ", &entries(&["x"])).unwrap();
        std::fs::write(tmp.path().join("notes.txt"), b"ignore me").unwrap();
        assert_eq!(s.list_newest_first(), vec!["dev-A01HQ".to_string()]);
    }

    #[test]
    fn remove_deletes_and_is_a_noop_when_absent() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 4);
        s.persist("dev-A01HQ", &entries(&["x"])).unwrap();
        s.remove("dev-A01HQ");
        assert!(!tmp.path().join("dev-A01HQ").exists());
        s.remove("dev-A01HQ");
    }

    #[test]
    fn persist_past_the_cap_evicts_the_oldest() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 2);
        for id in ["dev-A01HQ", "dev-A02HQ", "dev-A03HQ"] {
            s.persist(id, &entries(&["x"])).unwrap();
        }
        let left = s.list_newest_first();
        assert_eq!(left.len(), 2, "cap enforced, got {left:?}");
        assert_eq!(left, vec!["dev-A03HQ".to_string(), "dev-A02HQ".to_string()]);
    }

    #[test]
    fn load_reports_failure_for_a_missing_batch() {
        let tmp = tempfile::tempdir().unwrap();
        let s = store(tmp.path(), 4);
        assert!(s.load("dev-nope").is_err());
    }

    #[test]
    fn persist_into_a_blocked_path_returns_persist_error() {
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"not a dir").unwrap();
        let s = PendingStore::new(blocker, 4);
        let err = s.persist("dev-A01HQ", &entries(&["x"])).unwrap_err();
        assert!(
            matches!(err, SubmitterError::Persist { ref batch_id, .. } if batch_id == "dev-A01HQ"),
            "got {err:?}"
        );
    }
}
