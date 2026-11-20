//! The update policy: when to check, what to download, and what to tell
//! the frontend about it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use chrono::Utc;

use super::error::UpdateError;
use super::notifier::UpdateNotifier;
use super::source::{PendingRelease, UpdateSource};
use super::status::{UpdateStatus, UpdateStatusDto};
use super::version::is_newer;

/// How often the worker checks for a new release.
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    pub interval: Duration,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(3600),
        }
    }
}

/// A release that is downloaded and verified, waiting to be installed.
struct ReadyRelease {
    version: String,
    notes: Option<String>,
    date: Option<String>,
    bytes: Vec<u8>,
    release: Arc<dyn PendingRelease>,
}

struct ServiceState {
    status: UpdateStatus,
    last_checked_at: Option<String>,
    /// The message of the most recent failed check.
    ///
    /// It is separate from the status because a check that fails while a
    /// release is already held leaves the status at `Ready` — the
    /// download is still installable, and the failure is reported next
    /// to it rather than in place of it.
    last_error: Option<String>,
    ready: Option<ReadyRelease>,
}

struct Inner {
    current_version: String,
    config: UpdateConfig,
    source: Arc<dyn UpdateSource>,
    notifier: Arc<dyn UpdateNotifier>,
    data_dir: PathBuf,
    state: Mutex<ServiceState>,
    /// Guards against two checks running at once, so a manual check
    /// during a download cannot start a second download.
    checking: AtomicBool,
}

/// The update service. Cheap to clone: the state is shared.
#[derive(Clone)]
pub struct UpdateService {
    inner: Arc<Inner>,
}

/// Where the artifact for `version` is cached.
///
/// `<data_dir>/updates/<version>/artifact.bin`. The bytes are verified
/// against the current manifest every time they are read back, so a
/// truncated or tampered file is never installed.
pub fn artifact_cache_path(data_dir: &Path, version: &str) -> PathBuf {
    data_dir.join("updates").join(version).join("artifact.bin")
}

/// True when `version` is safe to use as a directory name.
///
/// A version that is not cacheable is still downloaded and installed, but
/// never written to disk.
fn is_cacheable_version(version: &str) -> bool {
    !version.is_empty()
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Clears the in-flight flag when the check ends, however it ends.
struct CheckGuard<'a>(&'a AtomicBool);

impl Drop for CheckGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl UpdateService {
    pub fn new(
        current_version: String,
        config: UpdateConfig,
        source: Arc<dyn UpdateSource>,
        notifier: Arc<dyn UpdateNotifier>,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                current_version,
                config,
                source,
                notifier,
                data_dir,
                state: Mutex::new(ServiceState {
                    status: UpdateStatus::Idle,
                    last_checked_at: None,
                    last_error: None,
                    ready: None,
                }),
                checking: AtomicBool::new(false),
            }),
        }
    }

    /// The state mutex, recovered from poisoning.
    ///
    /// A panic in one of the updater's own paths poisons the mutex. The
    /// frontend reads the status on a timer, so a poisoned mutex must not
    /// turn every read into an error — the state it holds is a phase, a
    /// timestamp and a downloaded artifact, and none of them is left
    /// half-written by a panic.
    fn lock(&self) -> MutexGuard<'_, ServiceState> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The current status, as the frontend reads it.
    pub fn status(&self) -> UpdateStatusDto {
        let state = self.lock();
        UpdateStatusDto {
            status: state.status.clone(),
            current_version: self.inner.current_version.clone(),
            last_checked_at: state.last_checked_at.clone(),
            last_error: state.last_error.clone(),
        }
    }

    /// Runs a check now.
    ///
    /// A check already in flight wins: the caller gets the current status
    /// rather than starting a second download of the same bytes.
    pub async fn check_now(&self) -> UpdateStatusDto {
        if self.inner.checking.swap(true, Ordering::SeqCst) {
            return self.status();
        }
        let _guard = CheckGuard(&self.inner.checking);
        self.check_once().await
    }

    /// The worker loop: check, then wait an interval, forever.
    ///
    /// Each check runs in its own task, so a panic in one of them cannot
    /// take the loop down with it. An updater that stopped checking would
    /// silently never update again, and nothing would say so.
    pub async fn run(self) {
        loop {
            let service = self.clone();
            if let Err(e) = tokio::spawn(async move { service.check_now().await }).await {
                tracing::warn!(error = %e, "update check panicked");
            }
            tokio::time::sleep(self.inner.config.interval).await;
        }
    }

    /// Installs the downloaded release.
    ///
    /// On Windows the plugin launches the installer and exits the process,
    /// so this never returns there.
    pub async fn install_ready(&self) -> Result<(), UpdateError> {
        let ready = {
            let mut state = self.lock();
            state.ready.take()
        };
        let Some(ready) = ready else {
            return Err(UpdateError::NotReady);
        };

        let version = ready.version.clone();
        self.set_status(UpdateStatus::Installing {
            version: version.clone(),
        });

        match ready.release.install(&ready.bytes) {
            Ok(()) => Ok(()),
            Err(e) => {
                // The artifact is still good — put it back so the update
                // stays installable rather than disappearing behind a
                // failure.
                let notes = ready.notes.clone();
                let date = ready.date.clone();
                {
                    let mut state = self.lock();
                    state.ready = Some(ready);
                }
                self.set_status(UpdateStatus::Ready {
                    version,
                    notes,
                    date,
                });
                Err(e)
            }
        }
    }

    async fn check_once(&self) -> UpdateStatusDto {
        self.set_status(UpdateStatus::Checking);

        let release = match self.inner.source.check().await {
            Ok(Some(release)) => release,
            Ok(None) => {
                self.finish_check(UpdateStatus::Idle);
                return self.status();
            }
            Err(e) => {
                tracing::warn!(error = %e, "update check failed");
                self.fail_check(e.to_string());
                return self.status();
            }
        };

        let version = release.version().to_string();

        // Already holding this release (or a newer one): nothing to do.
        // This is what keeps the hourly loop from re-fetching tens of
        // megabytes, and what stops a server-side rollback from
        // replacing a newer release with an older one.
        let held = {
            let state = self.lock();
            state
                .ready
                .as_ref()
                .map(|r| (r.version.clone(), r.notes.clone(), r.date.clone()))
        };
        if let Some((held_version, held_notes, held_date)) = held {
            if !is_newer(&version, &held_version) {
                // The check changed nothing, so the phase goes back to
                // the one the held release is in — a check that finds no
                // news must not strand the status in `Checking`.
                self.finish_check(UpdateStatus::Ready {
                    version: held_version,
                    notes: held_notes,
                    date: held_date,
                });
                return self.status();
            }
        }

        // The bytes: from the cache when they still verify, otherwise
        // freshly downloaded.
        let (bytes, from_cache) = match self.read_cached_artifact(&version, release.as_ref()) {
            Some(bytes) => (bytes, true),
            None => {
                self.set_status(UpdateStatus::Downloading {
                    version: version.clone(),
                    downloaded: 0,
                    total: None,
                });
                match self.download(release.as_ref(), &version).await {
                    Ok(bytes) => (bytes, false),
                    Err(e) => {
                        tracing::warn!(error = %e, "update download failed");
                        self.fail_check(e.to_string());
                        return self.status();
                    }
                }
            }
        };

        // Verify before the release is ever announced. The plugin
        // verifies the bytes it downloads as well, but the artifact can
        // also come back from the disk cache — bytes that were verified
        // against an earlier manifest and are checked against this one
        // here. A tampered artifact never reaches `Ready`.
        if let Err(e) = release.verify(&bytes) {
            tracing::warn!(error = %e, "update verification failed");
            self.fail_check(e.to_string());
            return self.status();
        }

        // Only write what was just downloaded. An artifact that came
        // from the cache is already on disk, and rewriting it on every
        // launch would be pointless I/O.
        if !from_cache {
            self.write_cached_artifact(&version, &bytes);
        }
        self.prune_cache(&version);

        let notes = release.notes().map(|n| n.to_string());
        let date = release.date().map(|d| d.to_string());
        {
            let mut state = self.lock();
            state.ready = Some(ReadyRelease {
                version: version.clone(),
                notes: notes.clone(),
                date: date.clone(),
                bytes,
                release: release.clone(),
            });
        }
        self.finish_check(UpdateStatus::Ready {
            version: version.clone(),
            notes: notes.clone(),
            date: date.clone(),
        });
        self.status()
    }

    /// Downloads the package, keeping the status's progress current.
    async fn download(
        &self,
        release: &dyn PendingRelease,
        version: &str,
    ) -> Result<Vec<u8>, UpdateError> {
        let downloaded = Arc::new(AtomicU64::new(0));
        let total = Arc::new(AtomicU64::new(0));
        let counter = downloaded.clone();
        let total_slot = total.clone();
        let progress_version = version.to_string();

        let on_progress = move |chunk_len: u64, content_length: Option<u64>| {
            let seen = counter.fetch_add(chunk_len, Ordering::SeqCst) + chunk_len;
            if let Some(len) = content_length {
                total_slot.store(len, Ordering::SeqCst);
            }
            let known_total = total_slot.load(Ordering::SeqCst);
            self.set_status(UpdateStatus::Downloading {
                version: progress_version.clone(),
                downloaded: seen,
                total: (known_total > 0).then_some(known_total),
            });
        };

        let bytes = release.download(&on_progress).await?;
        tracing::info!(version, bytes = bytes.len(), "update artifact downloaded");
        Ok(bytes)
    }

    /// Reads the cached artifact for `version` and verifies it against the
    /// current release. `None` means "not cached, or not usable".
    fn read_cached_artifact(&self, version: &str, release: &dyn PendingRelease) -> Option<Vec<u8>> {
        if !is_cacheable_version(version) {
            return None;
        }
        let path = artifact_cache_path(&self.inner.data_dir, version);
        let bytes = std::fs::read(&path).ok()?;
        if release.verify(&bytes).is_ok() {
            Some(bytes)
        } else {
            // A truncated or tampered artifact is never installed.
            let _ = std::fs::remove_file(&path);
            None
        }
    }

    /// Writes the verified artifact to the cache.
    ///
    /// The write goes to a temporary name and is renamed into place, so an
    /// interrupted write never leaves a half-written artifact where the
    /// real one is.
    fn write_cached_artifact(&self, version: &str, bytes: &[u8]) {
        if !is_cacheable_version(version) {
            return;
        }
        let path = artifact_cache_path(&self.inner.data_dir, version);
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                return;
            }
        }
        let partial = path.with_extension("bin.partial");
        if std::fs::write(&partial, bytes).is_ok() {
            let _ = std::fs::rename(&partial, &path);
        }
    }

    /// Removes the cache directories of versions other than `keep`.
    fn prune_cache(&self, keep: &str) {
        let root = self.inner.data_dir.join("updates");
        let Ok(entries) = std::fs::read_dir(&root) else {
            return;
        };
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy() != keep {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }

    /// Stores a new phase and tells the frontend when the phase moved.
    ///
    /// The frontend reads the status through the `update_status` command;
    /// this only says "something moved, ask again". It fires on a phase
    /// change and not on every progress update, so a download does not
    /// flood the channel.
    fn set_status(&self, status: UpdateStatus) {
        let phase = status.phase();
        let changed = {
            let mut state = self.lock();
            let changed = state.status.phase() != phase;
            state.status = status;
            changed
        };
        if changed {
            self.inner.notifier.notify_status_changed(phase);
        }
    }

    /// Records the completed check and the phase it ended in.
    fn finish_check(&self, status: UpdateStatus) {
        let phase = status.phase();
        let changed = {
            let mut state = self.lock();
            let changed = state.status.phase() != phase;
            state.status = status;
            state.last_checked_at = Some(Utc::now().to_rfc3339());
            state.last_error = None;
            changed
        };
        if changed {
            self.inner.notifier.notify_status_changed(phase);
        }
    }

    /// Records a failed check.
    ///
    /// A release that is already downloaded stays installable: the
    /// failure is reported next to it, not in place of it. Replacing a
    /// `Ready` status with a `Failed` one would hide the update the user
    /// has not installed yet — and the artifact is still on disk, so
    /// `install_ready` would still work while the UI claimed there was
    /// nothing to install.
    fn fail_check(&self, message: String) {
        let phase = {
            let mut state = self.lock();
            state.last_checked_at = Some(Utc::now().to_rfc3339());
            state.last_error = Some(message.clone());
            let status = match &state.ready {
                Some(ready) => UpdateStatus::Ready {
                    version: ready.version.clone(),
                    notes: ready.notes.clone(),
                    date: ready.date.clone(),
                },
                None => UpdateStatus::Failed {
                    message: message.clone(),
                },
            };
            let phase = status.phase();
            state.status = status;
            phase
        };
        // The phase may not have moved — a failure while a release is
        // held leaves it at `Ready` — but the failure is new, so the
        // frontend is told either way.
        self.inner.notifier.notify_status_changed(phase);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use tempfile::TempDir;

    use super::*;
    use crate::updater::notifier::UpdateNotifier;
    use crate::updater::source::{PendingRelease, UpdateSource};

    /// A release with fixed bytes, counting the calls the service makes.
    struct FakeRelease {
        version: String,
        bytes: Vec<u8>,
        verify_fails: bool,
        downloads: Arc<AtomicUsize>,
        installs: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl PendingRelease for FakeRelease {
        fn version(&self) -> &str {
            &self.version
        }

        fn notes(&self) -> Option<&str> {
            Some("release notes")
        }

        fn date(&self) -> Option<&str> {
            Some("2026-11-18T06:05:10Z")
        }

        async fn download(
            &self,
            on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
        ) -> Result<Vec<u8>, UpdateError> {
            self.downloads.fetch_add(1, Ordering::SeqCst);
            on_progress(self.bytes.len() as u64, Some(self.bytes.len() as u64));
            Ok(self.bytes.clone())
        }

        fn verify(&self, bytes: &[u8]) -> Result<(), UpdateError> {
            if self.verify_fails {
                return Err(UpdateError::Verification {
                    message: "bad signature".into(),
                });
            }
            if bytes != self.bytes.as_slice() {
                return Err(UpdateError::Verification {
                    message: "wrong bytes".into(),
                });
            }
            Ok(())
        }

        fn install(&self, _bytes: &[u8]) -> Result<(), UpdateError> {
            self.installs.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    /// A source whose announced release and failure mode can change between
    /// checks.
    struct FakeSource {
        announced: Mutex<Option<Arc<dyn PendingRelease>>>,
        fail_with: Mutex<Option<String>>,
        checks: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl UpdateSource for FakeSource {
        async fn check(&self) -> Result<Option<Arc<dyn PendingRelease>>, UpdateError> {
            self.checks.fetch_add(1, Ordering::SeqCst);
            if let Some(message) = self.fail_with.lock().unwrap().clone() {
                return Err(UpdateError::Network { message });
            }
            Ok(self.announced.lock().unwrap().clone())
        }
    }

    /// A release whose download fails.
    struct FailingDownloadRelease {
        version: String,
    }

    #[async_trait::async_trait]
    impl PendingRelease for FailingDownloadRelease {
        fn version(&self) -> &str {
            &self.version
        }

        fn notes(&self) -> Option<&str> {
            None
        }

        fn date(&self) -> Option<&str> {
            None
        }

        async fn download(
            &self,
            _on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
        ) -> Result<Vec<u8>, UpdateError> {
            Err(UpdateError::Network {
                message: "connection reset".into(),
            })
        }

        fn verify(&self, _bytes: &[u8]) -> Result<(), UpdateError> {
            Ok(())
        }

        fn install(&self, _bytes: &[u8]) -> Result<(), UpdateError> {
            Ok(())
        }
    }

    /// A source whose check takes a while, so a second check can start
    /// while the first is still running.
    struct SlowSource {
        delay: Duration,
        checks: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl UpdateSource for SlowSource {
        async fn check(&self) -> Result<Option<Arc<dyn PendingRelease>>, UpdateError> {
            self.checks.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.delay).await;
            Ok(None)
        }
    }

    /// A source whose first check panics.
    struct PanickingSource {
        checks: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl UpdateSource for PanickingSource {
        async fn check(&self) -> Result<Option<Arc<dyn PendingRelease>>, UpdateError> {
            let attempt = self.checks.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                panic!("the release server exploded");
            }
            Ok(None)
        }
    }

    /// Records the notifications the service emits.
    struct RecordingNotifier {
        phases: Arc<Mutex<Vec<String>>>,
    }

    impl UpdateNotifier for RecordingNotifier {
        fn notify_status_changed(&self, phase: &str) {
            self.phases.lock().unwrap().push(phase.to_string());
        }
    }

    /// The phases the notifier was told about.
    fn phases(f: &Fixture) -> Vec<String> {
        f.notifier.phases.lock().unwrap().clone()
    }

    /// A service over a temp data directory, plus the fakes behind it.
    struct Fixture {
        service: UpdateService,
        source: Arc<FakeSource>,
        notifier: Arc<RecordingNotifier>,
        downloads: Arc<AtomicUsize>,
        installs: Arc<AtomicUsize>,
        _dir: TempDir,
    }

    /// Build a fixture whose source announces `version` (or nothing), and
    /// whose release fails verification when `verify_fails` is set.
    fn fixture(version: Option<&str>, verify_fails: bool) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let downloads = Arc::new(AtomicUsize::new(0));
        let installs = Arc::new(AtomicUsize::new(0));
        let announced = version.map(|v| {
            Arc::new(FakeRelease {
                version: v.to_string(),
                bytes: b"package".to_vec(),
                verify_fails,
                downloads: downloads.clone(),
                installs: installs.clone(),
            }) as Arc<dyn PendingRelease>
        });
        let source = Arc::new(FakeSource {
            announced: Mutex::new(announced),
            fail_with: Mutex::new(None),
            checks: Arc::new(AtomicUsize::new(0)),
        });
        let notifier = Arc::new(RecordingNotifier {
            phases: Arc::new(Mutex::new(Vec::new())),
        });
        let service = UpdateService::new(
            "0.1.0".into(),
            UpdateConfig::default(),
            source.clone(),
            notifier.clone(),
            dir.path().to_path_buf(),
        );
        Fixture {
            service,
            source,
            notifier,
            downloads,
            installs,
            _dir: dir,
        }
    }

    #[tokio::test]
    async fn a_check_that_finds_nothing_leaves_the_status_idle() {
        let f = fixture(None, false);
        let dto = f.service.check_now().await;
        assert_eq!(dto.status, UpdateStatus::Idle);
        assert!(
            dto.last_checked_at.is_some(),
            "a completed check is timestamped"
        );
        assert_eq!(dto.current_version, "0.1.0");
    }

    #[tokio::test]
    async fn a_released_version_is_downloaded_verified_and_announced() {
        let f = fixture(Some("0.1.1"), false);
        let dto = f.service.check_now().await;
        assert_eq!(
            dto.status,
            UpdateStatus::Ready {
                version: "0.1.1".into(),
                notes: Some("release notes".into()),
                date: Some("2026-11-18T06:05:10Z".into()),
            }
        );
        assert_eq!(
            phases(&f),
            vec![
                "checking".to_string(),
                "downloading".to_string(),
                "ready".to_string()
            ]
        );
        assert_eq!(f.downloads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn the_same_version_is_not_downloaded_twice() {
        let f = fixture(Some("0.1.1"), false);
        f.service.check_now().await;
        f.service.check_now().await;
        assert_eq!(f.downloads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn an_older_announced_version_does_not_replace_the_ready_one() {
        let f = fixture(Some("0.1.1"), false);
        f.service.check_now().await;
        // The server now announces an older version.
        let older = Arc::new(FakeRelease {
            version: "0.1.0".into(),
            bytes: b"package".to_vec(),
            verify_fails: false,
            downloads: f.downloads.clone(),
            installs: f.installs.clone(),
        }) as Arc<dyn PendingRelease>;
        *f.source.announced.lock().unwrap() = Some(older);
        let dto = f.service.check_now().await;
        assert_eq!(
            dto.status,
            UpdateStatus::Ready {
                version: "0.1.1".into(),
                notes: Some("release notes".into()),
                date: Some("2026-11-18T06:05:10Z".into()),
            }
        );
        assert_eq!(f.downloads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn a_failed_verification_does_not_become_ready() {
        let f = fixture(Some("0.1.1"), true);
        let dto = f.service.check_now().await;
        assert!(
            matches!(dto.status, UpdateStatus::Failed { .. }),
            "got {:?}",
            dto.status
        );
        assert!(
            !phases(&f).iter().any(|p| p == "ready"),
            "the release was never announced: {:?}",
            phases(&f)
        );
    }

    #[tokio::test]
    async fn an_unreachable_release_server_fails_without_panicking() {
        let f = fixture(Some("0.1.1"), false);
        *f.source.fail_with.lock().unwrap() = Some("offline".into());
        let dto = f.service.check_now().await;
        assert!(
            matches!(dto.status, UpdateStatus::Failed { .. }),
            "got {:?}",
            dto.status
        );
        assert!(
            !phases(&f).iter().any(|p| p == "ready"),
            "the release was never announced: {:?}",
            phases(&f)
        );
    }

    #[tokio::test]
    async fn a_cached_artifact_is_reused_instead_of_downloaded_again() {
        let dir = tempfile::tempdir().unwrap();
        let downloads = Arc::new(AtomicUsize::new(0));
        let installs = Arc::new(AtomicUsize::new(0));

        let build = || {
            let announced = Arc::new(FakeRelease {
                version: "0.1.1".into(),
                bytes: b"package".to_vec(),
                verify_fails: false,
                downloads: downloads.clone(),
                installs: installs.clone(),
            }) as Arc<dyn PendingRelease>;
            let source = Arc::new(FakeSource {
                announced: Mutex::new(Some(announced)),
                fail_with: Mutex::new(None),
                checks: Arc::new(AtomicUsize::new(0)),
            });
            UpdateService::new(
                "0.1.0".into(),
                UpdateConfig::default(),
                source,
                Arc::new(RecordingNotifier {
                    phases: Arc::new(Mutex::new(Vec::new())),
                }),
                dir.path().to_path_buf(),
            )
        };

        let first = build();
        first.check_now().await;
        assert_eq!(downloads.load(Ordering::SeqCst), 1);

        // A second service over the same data directory reuses the artifact.
        let second = build();
        let dto = second.check_now().await;
        assert_eq!(
            downloads.load(Ordering::SeqCst),
            1,
            "the cached artifact is reused"
        );
        assert!(matches!(dto.status, UpdateStatus::Ready { .. }));
    }

    /// An artifact that came from the cache is already on disk. Writing
    /// it back would be pointless I/O on every launch.
    #[tokio::test]
    async fn a_cached_artifact_is_not_rewritten() {
        let dir = tempfile::tempdir().unwrap();
        let downloads = Arc::new(AtomicUsize::new(0));
        let installs = Arc::new(AtomicUsize::new(0));

        let build = || {
            let announced = Arc::new(FakeRelease {
                version: "0.1.1".into(),
                bytes: b"package".to_vec(),
                verify_fails: false,
                downloads: downloads.clone(),
                installs: installs.clone(),
            }) as Arc<dyn PendingRelease>;
            let source = Arc::new(FakeSource {
                announced: Mutex::new(Some(announced)),
                fail_with: Mutex::new(None),
                checks: Arc::new(AtomicUsize::new(0)),
            });
            UpdateService::new(
                "0.1.0".into(),
                UpdateConfig::default(),
                source,
                Arc::new(RecordingNotifier {
                    phases: Arc::new(Mutex::new(Vec::new())),
                }),
                dir.path().to_path_buf(),
            )
        };

        let first = build();
        first.check_now().await;

        let cached = artifact_cache_path(dir.path(), "0.1.1");
        let written_at = std::fs::metadata(&cached).unwrap().modified().unwrap();

        let second = build();
        second.check_now().await;

        let still_at = std::fs::metadata(&cached).unwrap().modified().unwrap();
        assert_eq!(written_at, still_at, "the cached artifact was rewritten");
    }

    #[tokio::test]
    async fn a_corrupt_cached_artifact_is_discarded_and_downloaded_again() {
        let dir = tempfile::tempdir().unwrap();
        let downloads = Arc::new(AtomicUsize::new(0));
        let installs = Arc::new(AtomicUsize::new(0));

        let build = || {
            let announced = Arc::new(FakeRelease {
                version: "0.1.1".into(),
                bytes: b"package".to_vec(),
                verify_fails: false,
                downloads: downloads.clone(),
                installs: installs.clone(),
            }) as Arc<dyn PendingRelease>;
            let source = Arc::new(FakeSource {
                announced: Mutex::new(Some(announced)),
                fail_with: Mutex::new(None),
                checks: Arc::new(AtomicUsize::new(0)),
            });
            UpdateService::new(
                "0.1.0".into(),
                UpdateConfig::default(),
                source,
                Arc::new(RecordingNotifier {
                    phases: Arc::new(Mutex::new(Vec::new())),
                }),
                dir.path().to_path_buf(),
            )
        };

        let first = build();
        first.check_now().await;
        assert_eq!(downloads.load(Ordering::SeqCst), 1);

        // Corrupt the cached artifact, as an interrupted download would.
        let cached = artifact_cache_path(dir.path(), "0.1.1");
        std::fs::write(&cached, b"truncated").unwrap();

        let second = build();
        let dto = second.check_now().await;
        assert_eq!(
            downloads.load(Ordering::SeqCst),
            2,
            "the corrupt artifact is replaced"
        );
        assert!(matches!(dto.status, UpdateStatus::Ready { .. }));
    }

    #[tokio::test]
    async fn install_ready_installs_the_downloaded_bytes() {
        let f = fixture(Some("0.1.1"), false);
        f.service.check_now().await;
        f.service.install_ready().await.unwrap();
        assert_eq!(f.installs.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn install_ready_without_a_ready_release_is_an_error() {
        let f = fixture(None, false);
        f.service.check_now().await;
        assert!(matches!(
            f.service.install_ready().await,
            Err(UpdateError::NotReady)
        ));
    }

    /// The frontend reads the status through the `update_status` command,
    /// and the command is only called again when something tells it to.
    /// A check that failed is something.
    #[tokio::test]
    async fn a_failed_check_tells_the_frontend_to_read_the_status_again() {
        let f = fixture(Some("0.1.1"), false);
        *f.source.fail_with.lock().unwrap() = Some("offline".into());
        f.service.check_now().await;
        assert!(
            phases(&f).iter().any(|p| p == "failed"),
            "the failure was not announced: {:?}",
            phases(&f)
        );
    }

    /// A download that fails is a failure, not a release.
    #[tokio::test]
    async fn a_failed_download_leaves_the_status_failed() {
        let f = fixture(Some("0.1.1"), false);
        *f.source.announced.lock().unwrap() = Some(Arc::new(FailingDownloadRelease {
            version: "0.1.1".into(),
        }));
        let dto = f.service.check_now().await;
        assert!(
            matches!(dto.status, UpdateStatus::Failed { .. }),
            "got {:?}",
            dto.status
        );
    }

    /// A release that is already downloaded stays installable when the
    /// next check fails. The artifact is on disk and verified; replacing
    /// the status with a failure would hide the update the user has not
    /// installed yet, and the row's install button with it.
    #[tokio::test]
    async fn a_failed_check_keeps_a_held_release_installable() {
        let f = fixture(Some("0.1.1"), false);
        f.service.check_now().await;
        *f.source.fail_with.lock().unwrap() = Some("offline".into());
        let dto = f.service.check_now().await;
        assert_eq!(
            dto.status,
            UpdateStatus::Ready {
                version: "0.1.1".into(),
                notes: Some("release notes".into()),
                date: Some("2026-11-18T06:05:10Z".into()),
            },
            "the downloaded release stays installable"
        );
        assert_eq!(
            dto.last_error.as_deref(),
            Some("update network failure: offline"),
            "the failure is reported next to the release"
        );
    }

    /// A manual check during a download must not start a second one.
    #[tokio::test]
    async fn a_second_check_while_one_is_in_flight_is_a_no_op() {
        let checks = Arc::new(AtomicUsize::new(0));
        let service = UpdateService::new(
            "0.1.0".into(),
            UpdateConfig::default(),
            Arc::new(SlowSource {
                delay: Duration::from_secs(30),
                checks: checks.clone(),
            }),
            Arc::new(RecordingNotifier {
                phases: Arc::new(Mutex::new(Vec::new())),
            }),
            tempfile::tempdir().unwrap().path().to_path_buf(),
        );
        let running = service.clone();
        let first = tokio::spawn(async move { running.check_now().await });
        // Let the first check reach the source.
        tokio::time::sleep(Duration::from_millis(20)).await;

        let second = service.check_now().await;
        assert_eq!(
            checks.load(Ordering::SeqCst),
            1,
            "the second check did not start another one"
        );
        assert_eq!(second.status, UpdateStatus::Checking);
        first.abort();
    }

    /// A check that panics must not take the hourly loop down with it:
    /// an updater that stopped checking would silently never update
    /// again.
    #[tokio::test]
    async fn a_panicking_check_does_not_stop_the_worker_loop() {
        let checks = Arc::new(AtomicUsize::new(0));
        let service = UpdateService::new(
            "0.1.0".into(),
            UpdateConfig {
                interval: Duration::from_millis(10),
            },
            Arc::new(PanickingSource {
                checks: checks.clone(),
            }),
            Arc::new(RecordingNotifier {
                phases: Arc::new(Mutex::new(Vec::new())),
            }),
            tempfile::tempdir().unwrap().path().to_path_buf(),
        );
        let worker = tokio::spawn(service.run());
        tokio::time::timeout(Duration::from_secs(10), async {
            while checks.load(Ordering::SeqCst) < 2 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("the worker kept checking after the panic");
        worker.abort();
    }

    /// A panic in one of the updater's own paths poisons the state
    /// mutex. The frontend reads the status on a timer, so a poisoned
    /// mutex must not turn every read into an error.
    #[tokio::test]
    async fn a_poisoned_state_mutex_does_not_break_the_status_read() {
        let f = fixture(None, false);
        let inner = f.service.inner.clone();
        let _ = std::thread::spawn(move || {
            let _guard = inner.state.lock().unwrap();
            panic!("poison the mutex");
        })
        .join();
        let dto = f.service.status();
        assert_eq!(dto.status, UpdateStatus::Idle);
    }
}
