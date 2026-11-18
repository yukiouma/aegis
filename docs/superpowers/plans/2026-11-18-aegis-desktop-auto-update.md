# Aegis Desktop Auto-Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The `aegis-desktop` app detects a new release at launch and hourly, downloads and verifies the artifact in the background, tells the user once that an update is ready, and installs and restarts on Windows when they accept.

**Architecture:** A Rust worker task spawned in `setup()` owns the whole check/download/install lifecycle; it is the only thing that talks to the release server. The frontend never calls the updater plugin — it reads a status through one Tauri command and renders a dialog plus a Settings row. The verified artifact is cached on disk so a deferred update is not re-downloaded on every launch.

**Tech Stack:** Rust 2021, Tauri v2 (`tauri-plugin-updater` 2.14.1), `async-trait`, `semver`, `chrono`; TypeScript 5.8, React 19, TanStack Query v5, TanStack Router, MUI 9 via `@aegis/ui`, Vitest.

**Spec:** `docs/superpowers/specs/2026-11-18-aegis-desktop-auto-update-design.md`

## Global Constraints

- The Rust side of the desktop app is transport-only *for `aegis-server`*. The updater talks to the **release server**, not `aegis-server`, so it lives in its own `updater` module and does **not** go under `src-tauri/src/http/`.
- Command names are snake_case; the TS `api` wrappers use camelCase and call the snake_case command.
- Every DTO crossing the Tauri boundary is hand-mirrored: Rust `#[serde(rename_all = "camelCase")]` on the struct, camelCase TS interface. These are Tauri command payloads, so their JSON keys are camelCase.
- `ApiError` (`src-tauri/src/http/dto.rs`) is the single error type every command returns. The frontend narrows on `kind` in `src/shared/api/error.ts`, whose `errorMessage` switch is **exhaustive** over `kind`.
- The global `QueryClient` defaults are `staleTime: Infinity`, `retry: false`, `refetchOnWindowFocus: false`, `refetchOnReconnect: false`. Anything that must change while the app is open needs an explicit trigger.
- All user-visible strings come through `useI18n()` → `t("path.to.key")`. Keys are added to **both** `lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts` — `TranslationKey` is `keyof typeof en` and `zhCN` is type-checked against it, so a missing translation is a compile error.
- Cross-feature TS imports go through the feature barrel (`features/<name>/index.ts`), never through nested paths.
- MUI primitives and icons come from `@aegis/ui/mui` and `@aegis/ui/icons`. Never import from `@mui/material` directly.
- Rust tests live in `#[cfg(test)] mod tests` next to the code they cover. `tempfile::tempdir()` scopes per-test temp directories.
- The `tokio` `time` feature is declared on the **desktop crate's own** dependency (`tokio = { workspace = true, features = ["time"] }`), not on the workspace entry. `tauri` enables `tokio` with `["rt", "rt-multi-thread", "sync", "fs", "io-util"]` and **not** `time`.
- `semver` is a new dependency of the desktop crate, declared locally with a `why` comment (it is not a shared workspace dependency).
- The release server currently serves a manifest with `plaform` instead of `platforms` and version `0.1.0` (equal to the app). **The desktop side is built against the plugin's correct manifest shape regardless.** End-to-end verification needs the user to publish a fixed 0.1.1 release (Task 7).

## Review Focus

Five failure modes the spec implies that the task tests must pin down. Each line names the condition and the expected behaviour; the task that owns the code carries the test.

1. **The release server is unreachable at launch** (laptop offline, VPN down). The app must start normally, the login screen must work, and the failure must surface as a status the Settings row shows — never as a modal, never as a crash. → Task 3, Task 6.
2. **The manifest is malformed** — exactly the `plaform` typo that is live right now. The check must fail cleanly into a failed status; the worker must not panic, must not wedge, and must keep the app usable. → Task 2, Task 3.
3. **The user defers an update and relaunches repeatedly.** The dialog must not reappear for that version, and the artifact must not be re-downloaded on each launch. → Task 3, Task 5, Task 6.
4. **More than one window is open** (the main window plus a `project:<code>` workspace). The check must run once per process, not once per window, and the dialog must appear only in the main window. The "once per process" half is structural — there is exactly one `tauri::async_runtime::spawn` in `setup()` (Task 4); the observable half is pinned by the mount-point assertions in Task 6.
5. **A download is interrupted** (the network drops mid-transfer, or the app is killed). A half-written artifact must never be treated as valid: the next launch must discard it and download again. → Task 3 (the cache is verified on read and rewritten on failure).

---

### Task 1: The updater's vocabulary — status, errors, version ordering

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/updater.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/updater/error.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/updater/status.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/updater/version.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` (add `mod updater;`)

**Interfaces:**
- Consumes: nothing (first task).
- Produces:
  - `updater::error::UpdateError` — `Network { message: String }`, `Invalid { message: String }`, `Verification { message: String }`, `Install { message: String }`, `NotReady`. Derives `Debug`, `thiserror::Error`.
  - `updater::status::UpdateStatus` — serde-tagged enum, `#[serde(tag = "kind", rename_all = "camelCase")]`, variants `Idle`, `Checking`, `Downloading { version: String, downloaded: u64, total: Option<u64> }`, `Ready { version: String, notes: Option<String>, date: Option<String> }`, `Installing { version: String }`, `Failed { message: String }`.
  - `updater::status::UpdateStatusDto` — `#[serde(rename_all = "camelCase")]`, fields `status: UpdateStatus`, `current_version: String`, `last_checked_at: Option<String>`.
  - `updater::version::is_newer(candidate: &str, held: &str) -> bool`.
  - `updater.rs` barrel re-exports `pub mod error; pub mod status; pub mod version;`.

- [ ] **Step 1: Write the failing tests**

Create `apps/desktop/aegis-desktop/src-tauri/src/updater/status.rs` with the enum and DTO declarations *and* a `#[cfg(test)] mod tests` containing:

```rust
#[test]
fn update_status_serializes_with_a_kind_tag() {
    let status = UpdateStatus::Downloading {
        version: "0.1.1".into(),
        downloaded: 100,
        total: Some(1000),
    };
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json["kind"], "downloading");
    assert_eq!(json["version"], "0.1.1");
    assert_eq!(json["downloaded"], 100);
    assert_eq!(json["total"], 1000);
}

#[test]
fn update_status_ready_carries_the_release_metadata() {
    let status = UpdateStatus::Ready {
        version: "0.1.1".into(),
        notes: Some("notes".into()),
        date: Some("2026-11-18T06:05:10Z".into()),
    };
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json["kind"], "ready");
    assert_eq!(json["notes"], "notes");
    assert_eq!(json["date"], "2026-11-18T06:05:10Z");
}

#[test]
fn update_status_dto_uses_camel_case_field_names() {
    let dto = UpdateStatusDto {
        status: UpdateStatus::Idle,
        current_version: "0.1.0".into(),
        last_checked_at: Some("2026-11-18T06:05:10Z".into()),
    };
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["currentVersion"], "0.1.0");
    assert_eq!(json["lastCheckedAt"], "2026-11-18T06:05:10Z");
    assert_eq!(json["status"]["kind"], "idle");
}
```

In `apps/desktop/aegis-desktop/src-tauri/src/updater/version.rs`:

```rust
#[test]
fn a_higher_version_is_newer() {
    assert!(is_newer("0.2.0", "0.1.1"));
    assert!(is_newer("0.1.2", "0.1.1"));
}

#[test]
fn the_same_version_is_not_newer() {
    assert!(!is_newer("0.1.1", "0.1.1"));
}

#[test]
fn an_older_version_is_not_newer() {
    assert!(!is_newer("0.1.5", "0.2.0"));
}

#[test]
fn a_leading_v_is_tolerated() {
    assert!(is_newer("v0.2.0", "0.1.1"));
}

#[test]
fn an_unparsable_pair_is_never_newer() {
    assert!(!is_newer("not-a-version", "0.1.1"));
    assert!(!is_newer("0.1.1", "not-a-version"));
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p aegis-desktop --lib updater::`
Expected: FAIL — `cannot find function or module 'updater'`.

- [ ] **Step 3: Implement the error type**

In `apps/desktop/aegis-desktop/src-tauri/src/updater/error.rs`:

```rust
//! Failure modes of the release-update path.

// The command layer maps these onto `ApiError`, so the conversion lives
// next to the variants it maps.
use crate::http::dto::ApiError;

/// Everything that can go wrong while checking, downloading, verifying or
/// installing a release.
///
/// The variants are split so the command layer can map a transport failure
/// onto `ApiError::Network` and everything else onto `ApiError::Update`.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// The release server could not be reached, or the request failed.
    #[error("update network failure: {message}")]
    Network { message: String },
    /// The response could not be read as a usable release manifest.
    #[error("update manifest invalid: {message}")]
    Invalid { message: String },
    /// The downloaded package failed signature verification.
    #[error("update verification failed: {message}")]
    Verification { message: String },
    /// The installer could not be launched.
    #[error("update install failed: {message}")]
    Install { message: String },
    /// There is no downloaded artifact ready to install.
    #[error("no update is ready to install")]
    NotReady,
}

impl From<UpdateError> for ApiError {
    fn from(err: UpdateError) -> Self {
        match err {
            // A transport failure to the release server is a network
            // failure like any other.
            UpdateError::Network { message } => ApiError::Network { message },
            other => ApiError::Update { message: other.to_string() },
        }
    }
}
```

- [ ] **Step 4: Implement the status types**

In `apps/desktop/aegis-desktop/src-tauri/src/updater/status.rs`:

```rust
//! The update phase, and the shape the frontend reads it in.

use serde::{Deserialize, Serialize};

/// Where the updater currently is.
///
/// Serialised as a tagged object (`{"kind": "ready", ...}`) so the
/// TypeScript side can narrow on `kind` the same way it narrows on
/// `ApiError.kind`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UpdateStatus {
    Idle,
    Checking,
    Downloading {
        version: String,
        downloaded: u64,
        total: Option<u64>,
    },
    Ready {
        version: String,
        notes: Option<String>,
        date: Option<String>,
    },
    Installing {
        version: String,
    },
    Failed {
        message: String,
    },
}

/// The status as the `update_status` command returns it.
///
/// `current_version` is the running app's version. `last_checked_at` is
/// set only once a check has actually completed, which is what lets the
/// frontend say "up to date" without claiming it before the first check.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatusDto {
    pub status: UpdateStatus,
    pub current_version: String,
    pub last_checked_at: Option<String>,
}
```

- [ ] **Step 5: Implement the version comparison**

In `apps/desktop/aegis-desktop/src-tauri/src/updater/version.rs`:

```rust
//! Version ordering for the release channel.

use semver::Version;

/// True when `candidate` is strictly newer than `held`.
///
/// Both sides are parsed as semver with a leading `v` tolerated. If
/// either side fails to parse the answer is `false`: an unparsable pair
/// cannot prove that the candidate is not a downgrade, and treating it
/// as newer is how a second prompt for an older release gets in.
pub fn is_newer(candidate: &str, held: &str) -> bool {
    match (
        Version::parse(candidate.trim_start_matches('v')),
        Version::parse(held.trim_start_matches('v')),
    ) {
        (Ok(candidate), Ok(held)) => candidate > held,
        _ => false,
    }
}
```

- [ ] **Step 6: Add the module and the `semver` dependency**

In `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`, add `mod updater;` alongside the other module declarations.

In `apps/desktop/aegis-desktop/src-tauri/Cargo.toml`, add to `[dependencies]`:

```toml
# `semver` orders the version the release server announces against the
# version the updater already holds, so a server-side rollback is never
# mistaken for a newer release. Not a shared workspace dependency —
# only the desktop updater needs it.
semver = "1"
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test -p aegis-desktop --lib updater::`
Expected: PASS — all the `status::` and `version::` tests above.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/updater.rs \
        apps/desktop/aegis-desktop/src-tauri/src/updater/ \
        apps/desktop/aegis-desktop/src-tauri/src/lib.rs \
        apps/desktop/aegis-desktop/src-tauri/Cargo.toml
git commit -m "feat(desktop): add updater status, error and version types"
```

---

### Task 2: The release-source seam — traits, the plugin adapter, and the manifest contract

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/updater/source.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/updater.rs` (add `pub mod source;`)

**Interfaces:**
- Consumes: `updater::error::UpdateError` (Task 1).
- Produces:
  - `updater::source::UpdateSource` — `async fn check(&self) -> Result<Option<Arc<dyn PendingRelease>>, UpdateError>`.
  - `updater::source::PendingRelease` — `fn version(&self) -> &str`, `fn notes(&self) -> Option<&str>`, `fn date(&self) -> Option<&str>`, `async fn download(&self, on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync)) -> Result<Vec<u8>, UpdateError>`, `fn verify(&self, bytes: &[u8]) -> Result<(), UpdateError>`, `fn install(&self, bytes: &[u8]) -> Result<(), UpdateError>`.
  - `updater::source::PluginUpdateSource::new(handle: tauri::AppHandle) -> Self`, implementing `UpdateSource`.
  - `updater::source::PluginRelease::new(update: tauri_plugin_updater::Update) -> Self`, implementing `PendingRelease`.

- [ ] **Step 1: Write the failing test — the manifest shape contract**

Create `apps/desktop/aegis-desktop/src-tauri/src/updater/source.rs` with the trait declarations *and* a `#[cfg(test)] mod tests` containing:

```rust
/// The plugin parses the release manifest into `RemoteRelease`, which
/// requires a `platforms` map keyed by target (or a top-level `url` +
/// `signature`). The release server served `plaform` — a misspelling —
/// which the parser ignores, leaving `platforms` absent and failing
/// with "the `url` field was not set on the updater response". This
/// test pins the shape the desktop depends on.
#[test]
fn a_manifest_with_platforms_parses() {
    let manifest = serde_json::json!({
        "version": "0.1.1",
        "notes": "Aegis Release 0.1.1",
        "pub_date": "2026-11-18T06:05:10.294Z",
        "platforms": {
            "windows-x86_64": {
                "signature": "sig",
                "url": "http://localhost:3000/aegis-desktop/releases/aegis-desktop_0.1.1_x64-setup.exe"
            }
        }
    });
    let release: RemoteRelease = serde_json::from_value(manifest).expect("manifest should parse");
    assert_eq!(release.version.to_string(), "0.1.1");
}

#[test]
fn a_manifest_with_a_misspelled_platforms_key_is_rejected() {
    let manifest = serde_json::json!({
        "version": "0.1.1",
        "plaform": {
            "windows-x86_64": {
                "signature": "sig",
                "url": "http://localhost:3000/aegis-desktop/releases/aegis-desktop_0.1.1_x64-setup.exe"
            }
        }
    });
    let result: Result<RemoteRelease, _> = serde_json::from_value(manifest);
    assert!(result.is_err(), "the misspelled key must not parse");
}

/// A response that could not be read as a manifest is an `Invalid`
/// failure, not a network failure — the release server answered, it
/// just answered with something unusable.
#[test]
fn a_manifest_parse_failure_is_classified_as_invalid() {
    let serde_error = serde_json::from_str::<serde_json::Value>("{ not json").unwrap_err();
    let classified = classify_error(tauri_plugin_updater::Error::Serialization(serde_error));
    assert!(
        matches!(classified, UpdateError::Invalid { .. }),
        "got {classified:?}"
    );
}

#[test]
fn a_download_failure_is_classified_as_a_network_failure() {
    let classified =
        classify_error(tauri_plugin_updater::Error::Network("status 500".into()));
    assert!(
        matches!(classified, UpdateError::Network { .. }),
        "got {classified:?}"
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p aegis-desktop --lib updater::source::`
Expected: FAIL — `cannot find type RemoteRelease in this scope`.

- [ ] **Step 3: Implement the traits**

In `apps/desktop/aegis-desktop/src-tauri/src/updater/source.rs`:

```rust
//! Where releases come from.
//!
//! The updater's policy never touches `tauri_plugin_updater` directly;
//! it talks to these two traits. That keeps the state machine testable
//! offline — a fake source needs no network and no Tauri runtime.

use std::sync::Arc;

use async_trait::async_trait;
use tauri::AppHandle;
use tauri_plugin_updater::{Update, UpdaterExt};

use super::error::UpdateError;

/// The release server, as the updater sees it.
#[async_trait]
pub trait UpdateSource: Send + Sync {
    /// The release the server announces, if it is newer than the running
    /// app. `None` means "nothing newer" — not an error.
    async fn check(&self) -> Result<Option<Arc<dyn PendingRelease>>, UpdateError>;
}

/// A single announced release, and the package that goes with it.
///
/// `verify` and `install` stay on the object rather than being reduced to
/// "bytes plus a version" because the plugin's `Update::install` and
/// `Update::verify` need `self` to reach the release's signature and the
/// plugin's configuration. The bytes alone are neither installable nor
/// verifiable.
#[async_trait]
pub trait PendingRelease: Send + Sync {
    /// The announced version.
    fn version(&self) -> &str;
    /// The release notes, when the server sent them.
    fn notes(&self) -> Option<&str>;
    /// The publication date as RFC 3339, when the server sent it.
    fn date(&self) -> Option<&str>;
    /// Downloads the package, reporting `(chunk, total)` as it goes.
    async fn download(
        &self,
        on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Result<Vec<u8>, UpdateError>;
    /// Checks the package signature against the configured public key.
    fn verify(&self, bytes: &[u8]) -> Result<(), UpdateError>;
    /// Installs an already-downloaded and already-verified package.
    fn install(&self, bytes: &[u8]) -> Result<(), UpdateError>;
}
```

- [ ] **Step 4: Implement the plugin adapter**

Append to `apps/desktop/aegis-desktop/src-tauri/src/updater/source.rs`:

```rust
/// The real source: the Tauri updater plugin, configured from
/// `tauri.conf.json`'s `plugins.updater` block (endpoint, public key,
/// insecure-transport flag and Windows install mode all come from there).
pub struct PluginUpdateSource {
    handle: AppHandle,
}

impl PluginUpdateSource {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
}

/// Classify a plugin error.
///
/// A response that could not be read as a release manifest is `Invalid`,
/// not a network failure — that is the case the currently-served
/// manifest (a misspelled `platforms` key) produces, and calling it a
/// network error would point the reader at the wrong thing.
fn classify_error(err: tauri_plugin_updater::Error) -> UpdateError {
    use tauri_plugin_updater::Error as PluginError;
    let message = err.to_string();
    match err {
        PluginError::Serialization(_)
        | PluginError::ReleaseNotFound
        | PluginError::Semver(_)
        | PluginError::UrlParse(_)
        | PluginError::TargetNotFound(_)
        | PluginError::TargetsNotFound(_)
        | PluginError::EmptyEndpoints => UpdateError::Invalid { message },
        PluginError::Minisign(_) => UpdateError::Verification { message },
        _ => UpdateError::Network { message },
    }
}

/// The `UpdateSource` impl on `PluginUpdateSource`:
///
/// - `self.handle.updater()` → `Err` becomes `UpdateError::Invalid`.
/// - `updater.check().await` → `Err` goes through `classify_error`;
///   `Ok(None)` stays `Ok(None)`; `Ok(Some(update))` becomes
///   `Ok(Some(Arc::new(PluginRelease::new(update)) as Arc<dyn PendingRelease>))`.
///
/// A release backed by the plugin's own `Update`.
pub struct PluginRelease {
    update: Update,
    /// `OffsetDateTime`'s `Display` is RFC 3339, so the plugin's date
    /// becomes a string without pulling in the `time` crate directly.
    date: Option<String>,
}

impl PluginRelease {
    pub fn new(update: Update) -> Self {
        let date = update.date.map(|d| d.to_string());
        Self { update, date }
    }
}

/// The `PendingRelease` impl on `PluginRelease`:
///
/// - `version()` → `&self.update.version`.
/// - `notes()` → `self.update.body.as_deref()`.
/// - `date()` → `self.date.as_deref()`.
/// - `download(on_progress)` → `self.update.download(|chunk_len, total|
///   on_progress(chunk_len as u64, total), || {}).await`. The plugin's
///   callback is `FnMut(usize, Option<u64>)`; ours is `Fn(u64, Option<u64>)`,
///   so the chunk length is widened at the boundary. `Err` becomes
///   `UpdateError::Network`.
/// - `verify(bytes)` → `self.update.verify(bytes)`; `Err` becomes
///   `UpdateError::Verification`.
/// - `install(bytes)` → `self.update.install(bytes)`; `Err` becomes
///   `UpdateError::Install`.

#[async_trait]
impl PendingRelease for PluginRelease {
    fn version(&self) -> &str {
        &self.update.version
    }

    fn notes(&self) -> Option<&str> {
        self.update.body.as_deref()
    }

    fn date(&self) -> Option<&str> {
        self.date.as_deref()
    }

    async fn download(
        &self,
        on_progress: &(dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Result<Vec<u8>, UpdateError> {
        self.update
            .download(
                |chunk_len, total| on_progress(chunk_len as u64, total),
                || {},
            )
            .await
            .map_err(|e| UpdateError::Network { message: e.to_string() })
    }

    fn verify(&self, bytes: &[u8]) -> Result<(), UpdateError> {
        self.update
            .verify(bytes)
            .map_err(|e| UpdateError::Verification { message: e.to_string() })
    }

    fn install(&self, bytes: &[u8]) -> Result<(), UpdateError> {
        self.update
            .install(bytes)
            .map_err(|e| UpdateError::Install { message: e.to_string() })
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p aegis-desktop --lib updater::source::`
Expected: PASS — both manifest-shape tests.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/updater/source.rs \
        apps/desktop/aegis-desktop/src-tauri/src/updater.rs
git commit -m "feat(desktop): add the updater release-source seam"
```

---

### Task 3: The update service — the policy, the disk cache, and the worker

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/updater/notifier.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/updater/service.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/updater.rs` (add `pub mod notifier; pub mod service;`)

**Interfaces:**
- Consumes: `updater::error::UpdateError`, `updater::status::{UpdateStatus, UpdateStatusDto}`, `updater::version::is_newer` (Task 1); `updater::source::{UpdateSource, PendingRelease}` (Task 2).
- Produces:
  - `updater::notifier::UpdateNotifier` — `fn notify_ready(&self, version: &str, notes: Option<&str>, date: Option<&str>)`.
  - `updater::notifier::TauriNotifier::new(handle: tauri::AppHandle) -> Self`, emitting `aegis:update-ready` to the `"main"` window.
  - `updater::service::UpdateConfig` — `pub interval: std::time::Duration`; `Default` = 3600 s.
  - `updater::service::UpdateService::new(current_version: String, config: UpdateConfig, source: Arc<dyn UpdateSource>, notifier: Arc<dyn UpdateNotifier>, data_dir: std::path::PathBuf) -> Self`.
  - `updater::service::UpdateService::status(&self) -> UpdateStatusDto`.
  - `updater::service::UpdateService::check_now(&self) -> UpdateStatusDto` (async).
  - `updater::service::UpdateService::install_ready(&self) -> Result<(), UpdateError>` (async).
  - `updater::service::UpdateService::run(self)` (async) — the worker loop.
  - `updater::service::artifact_cache_path(data_dir: &Path, version: &str) -> PathBuf` — `<data_dir>/updates/<version>/artifact.bin`.

- [ ] **Step 1: Write the failing service tests**

Create `apps/desktop/aegis-desktop/src-tauri/src/updater/service.rs` with the `UpdateConfig` and `UpdateService` declarations stubbed so the file compiles, plus a `#[cfg(test)] mod tests` containing:

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tempfile::TempDir;

use super::*;

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

/// Records the notifications the service emits.
struct RecordingNotifier {
    sent: Arc<Mutex<Vec<String>>>,
}

impl UpdateNotifier for RecordingNotifier {
    fn notify_ready(&self, version: &str, _notes: Option<&str>, _date: Option<&str>) {
        self.sent.lock().unwrap().push(version.to_string());
    }
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
        sent: Arc::new(Mutex::new(Vec::new())),
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
    assert!(dto.last_checked_at.is_some(), "a completed check is timestamped");
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
    assert_eq!(*f.notifier.sent.lock().unwrap(), vec!["0.1.1".to_string()]);
    assert_eq!(f.downloads.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn the_same_version_is_not_downloaded_or_announced_twice() {
    let f = fixture(Some("0.1.1"), false);
    f.service.check_now().await;
    f.service.check_now().await;
    assert_eq!(f.downloads.load(Ordering::SeqCst), 1);
    assert_eq!(f.notifier.sent.lock().unwrap().len(), 1);
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
    assert!(f.notifier.sent.lock().unwrap().is_empty());
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
    assert!(f.notifier.sent.lock().unwrap().is_empty());
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
                sent: Arc::new(Mutex::new(Vec::new())),
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
    assert_eq!(downloads.load(Ordering::SeqCst), 1, "the cached artifact is reused");
    assert!(matches!(dto.status, UpdateStatus::Ready { .. }));
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
                sent: Arc::new(Mutex::new(Vec::new())),
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
    assert_eq!(downloads.load(Ordering::SeqCst), 2, "the corrupt artifact is replaced");
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p aegis-desktop --lib updater::service::`
Expected: FAIL — the service's methods do not exist yet.

- [ ] **Step 3: Implement the notifier**

Create `apps/desktop/aegis-desktop/src-tauri/src/updater/notifier.rs`:

```rust
//! Telling the frontend that a release is ready.

use tauri::{AppHandle, Emitter};

/// Announces a downloaded, verified release to the frontend.
pub trait UpdateNotifier: Send + Sync {
    fn notify_ready(&self, version: &str, notes: Option<&str>, date: Option<&str>);
}

/// Emits `aegis:update-ready` to the main window.
///
/// The main window is the only target: the dialog lives in the
/// authenticated shell of the main window, and the workspace windows
/// have their own layout.
pub struct TauriNotifier {
    handle: AppHandle,
}

impl TauriNotifier {
    pub fn new(handle: AppHandle) -> Self {
        Self { handle }
    }
}

impl UpdateNotifier for TauriNotifier {
    fn notify_ready(&self, version: &str, notes: Option<&str>, date: Option<&str>) {
        let payload = serde_json::json!({
            "version": version,
            "notes": notes,
            "date": date,
        });
        if let Err(e) = self.handle.emit_to("main", "aegis:update-ready", payload) {
            tracing::warn!(error = %e, "failed to emit aegis:update-ready");
        }
    }
}
```

- [ ] **Step 4: Implement the service**

Create `apps/desktop/aegis-desktop/src-tauri/src/updater/service.rs`:

```rust
//! The update policy: when to check, what to download, and what to tell
//! the frontend about it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
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
```

with these implementations:

- `UpdateService::new(current_version, config, source, notifier, data_dir) -> Self` — builds `Inner` with `status: UpdateStatus::Idle`, `last_checked_at: None`, `ready: None`.
- `UpdateService::status(&self) -> UpdateStatusDto` — locks the state and clones the status, the current version and the timestamp.
- `UpdateService::check_now(&self) -> UpdateStatusDto` — if `self.inner.checking.swap(true, Ordering::SeqCst)` was already `true`, return `self.status()` unchanged; otherwise run `check_once()`, clear the flag, and return the status.
- `UpdateService::check_once(&self) -> UpdateStatusDto` — the body:
  1. Set the status to `Checking`.
  2. `self.inner.source.check().await`. On `Err`, set `Failed { message }`, stamp `last_checked_at`, and return.
  3. On `Ok(None)`, set `Idle`, stamp `last_checked_at`, and return.
  4. On `Ok(Some(release))`, take `version = release.version().to_string()`.
  5. If the state already holds a ready release and the announced version is **not newer** than the held one (`!is_newer(&version, &held_version)`), return the current status without downloading or notifying.
  6. Set the status to `Downloading { version, downloaded: 0, total: None }`.
  7. Try the cached artifact for `version`: read it and `release.verify(&bytes)`. If it verifies, use those bytes. Otherwise delete the stale file and download.
  8. Download with a progress callback that updates the status to `Downloading { version, downloaded, total }`.
  9. Verify the downloaded bytes. On failure set `Failed { message }` and return — a release is never announced unverified.
  10. Write the bytes to the cache (when the version is cacheable), prune the other cache directories, store the `ReadyRelease`, set the status to `Ready { version, notes, date }`, and call `notifier.notify_ready(&version, notes, date)`.
  11. Stamp `last_checked_at`.
- `UpdateService::install_ready(&self) -> Result<(), UpdateError>` — take the ready release out of the state; if there is none, return `Err(UpdateError::NotReady)`. Set the status to `Installing { version }`, call `release.install(&bytes)`. On error, put the ready release back and restore the status to `Ready` so the update stays installable, then return the error.
- `UpdateService::run(self)` — `loop { self.check_now().await; tokio::time::sleep(self.inner.config.interval).await; }`.
- A private `fn prune_cache(&self, keep: &str)` — removes every directory under `<data_dir>/updates` other than `keep`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p aegis-desktop --lib updater::service::`
Expected: PASS — every service test above.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/updater/
git commit -m "feat(desktop): add the update service, cache and worker"
```

---

### Task 4: The Tauri command surface

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/updater.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands.rs` (add `pub mod updater;`)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs` (add the `Update` variant)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` (register the commands, build the service, spawn the worker)
- Modify: `apps/desktop/aegis-desktop/src-tauri/Cargo.toml` (the `tokio` `time` feature)
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts` (the `ApiError` mirror)
- Modify: `apps/desktop/aegis-desktop/src/shared/api/error.ts` (the `errorMessage` arm)
- Test: `apps/desktop/aegis-desktop/src/test/shared/api/error.test.ts`

**Interfaces:**
- Consumes: `updater::service::{UpdateService, UpdateConfig}`, `updater::status::UpdateStatusDto`, `updater::error::UpdateError`, `updater::source::PluginUpdateSource`, `updater::notifier::TauriNotifier` (Tasks 1–3).
- Produces:
  - Commands `update_status`, `check_for_updates`, `install_ready_update`, all `Result<_, ApiError>`.
  - `ApiError::Update { message: String }`, serialising to `kind: "update"`.
  - The TS mirror `{ kind: "update"; message: string }`.

- [ ] **Step 1: Write the failing Rust test for the new error variant**

In `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs`, add to the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn update_error_serializes_with_the_update_kind() {
    let error = ApiError::Update {
        message: "verification failed".into(),
    };
    let json = serde_json::to_value(&error).unwrap();
    assert_eq!(json["kind"], "update");
    assert_eq!(json["message"], "verification failed");
}
```

- [ ] **Step 2: Write the failing TS test**

In `apps/desktop/aegis-desktop/src/test/shared/api/error.test.ts`, add:

```ts
it("renders an update error as its message", () => {
  expect(
    errorMessage({ kind: "update", message: "verification failed" }),
  ).toBe("verification failed");
});
```

- [ ] **Step 3: Run both tests to verify they fail**

Run: `cargo test -p aegis-desktop --lib http::dto::` and `pnpm --filter aegis-desktop test -- src/test/shared/api/error.test.ts`
Expected: FAIL — `ApiError::Update` does not exist; the TS type does not accept `kind: "update"`.

- [ ] **Step 4: Add the `Update` variant on both sides**

In `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs`, add to the `ApiError` enum:

```rust
/// The release-update path failed (check, download, verification or
/// install). The updater's own error kinds are folded into one so the
/// frontend has a single branch to render.
#[error("update failed: {message}")]
Update { message: String },
```

In `apps/desktop/aegis-desktop/src/shared/api/types.ts`, add to the `ApiError` union:

```ts
  | { kind: "update"; message: string };
```

In `apps/desktop/aegis-desktop/src/shared/api/error.ts`, add to the `errorMessage` switch:

```ts
    case "update":
      return err.message;
```

- [ ] **Step 5: Run both tests to verify they pass**

Run: `cargo test -p aegis-desktop --lib http::dto::` and `pnpm --filter aegis-desktop test -- src/test/shared/api/error.test.ts`
Expected: PASS.

- [ ] **Step 6: Implement the commands**

Create `apps/desktop/aegis-desktop/src-tauri/src/commands/updater.rs`:

```rust
//! Tauri command shims for the release-update path.
//!
//! Unlike the other command modules there is no `http/` twin: the
//! updater talks to the release server, not to `aegis-server`. The
//! policy lives in `crate::updater`.

use tauri::State;
use tracing::Instrument;

use crate::http::dto::ApiError;
use crate::updater::service::UpdateService;
use crate::updater::status::UpdateStatusDto;

/// The current update status.
#[tauri::command]
pub async fn update_status(
    service: State<'_, UpdateService>,
) -> Result<UpdateStatusDto, ApiError>;

/// Runs a check now, instead of waiting for the hourly one.
#[tauri::command]
pub async fn check_for_updates(
    service: State<'_, UpdateService>,
) -> Result<UpdateStatusDto, ApiError>;

/// Installs the downloaded release. On Windows this never returns — the
/// plugin launches the installer and exits the process.
#[tauri::command]
pub async fn install_ready_update(
    service: State<'_, UpdateService>,
) -> Result<(), ApiError>;
```

The bodies follow the tracing pattern of `src-tauri/src/commands/healthz.rs`
exactly: build a `tracing::info_span!("command", command = "<name>")`, wrap
the work in an `async move { … }.instrument(span).await`, log `"enter"`
before and `"success"` (or `tracing::error!(error = %e, "failed")`) after.
`update_status` returns `service.status()`, `check_for_updates` returns
`service.check_now().await`, and `install_ready_update` returns
`service.install_ready().await.map_err(Into::into)`.

- [ ] **Step 7: Register the commands and wire the service**

In `apps/desktop/aegis-desktop/src-tauri/src/commands.rs`, add `pub mod updater;`.

In `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`, add the three commands to the `tauri::generate_handler!` list:

```rust
            // updater
            commands::updater::update_status,
            commands::updater::check_for_updates,
            commands::updater::install_ready_update,
```

and in `setup`, after the app-data directory is resolved and before `Ok(())`:

```rust
            // The update worker. One per process: the frontend is loaded
            // in every window, so a frontend-owned timer would check and
            // download once per open window.
            let handle = app.handle().clone();
            let service = updater::service::UpdateService::new(
                handle.package_info().version.to_string(),
                updater::service::UpdateConfig::default(),
                std::sync::Arc::new(updater::source::PluginUpdateSource::new(handle.clone())),
                std::sync::Arc::new(updater::notifier::TauriNotifier::new(handle)),
                app_data_dir.clone(),
            );
            app.manage(service.clone());
            tauri::async_runtime::spawn(async move { service.run().await });
```

- [ ] **Step 8: Declare the `tokio` `time` feature**

In `apps/desktop/aegis-desktop/src-tauri/Cargo.toml`, change the `tokio` dependency to:

```toml
# The `time` feature is needed by the updater worker's hourly sleep.
# `tauri` enables `tokio` without it, so it is declared here, on the
# crate that needs it, rather than on the workspace entry.
tokio = { workspace = true, features = ["time"] }
```

- [ ] **Step 9: Build and run the Rust tests**

Run: `cargo build -p aegis-desktop && cargo test -p aegis-desktop --lib && cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings`
Expected: PASS, and clippy clean.

- [ ] **Step 10: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/ \
        apps/desktop/aegis-desktop/src/shared/api/types.ts \
        apps/desktop/aegis-desktop/src/shared/api/error.ts \
        apps/desktop/aegis-desktop/src/test/shared/api/error.test.ts
git commit -m "feat(desktop): expose the updater commands and the update error kind"
```

---

### Task 5: The frontend data layer

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts` (add the update DTOs)
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts` (add the three wrappers and export the DTO types)
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts` (add `queryKeys.updater.status()`)
- Modify: `apps/desktop/aegis-desktop/src/features/settings/data/persist.ts` (add the deferral accessors)
- Modify: `apps/desktop/aegis-desktop/src/features/settings/index.ts` (export them)
- Create: `apps/desktop/aegis-desktop/src/features/updater/index.ts`
- Create: `apps/desktop/aegis-desktop/src/features/updater/data/update-status.ts`
- Create: `apps/desktop/aegis-desktop/src/features/updater/data/update-prompt.ts`
- Test: `apps/desktop/aegis-desktop/src/test/features/updater/update-status.test.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/updater/update-deferral.test.tsx`

**Interfaces:**
- Consumes: the commands and the `ApiError::Update` kind (Task 4).
- Produces:
  - TS types `UpdateStatus` (a discriminated union on `kind`) and `UpdateStatusDto`.
  - `api.updateStatus(): Promise<UpdateStatusDto>`, `api.checkForUpdates(): Promise<UpdateStatusDto>`, `api.installReadyUpdate(): Promise<void>`.
  - `queryKeys.updater.status()` → `["updater", "status"] as const`.
  - `readUpdateDeferral(): Promise<string | null>` and `writeUpdateDeferral(version: string | null): Promise<void>`, exported from the settings feature barrel.
  - `useUpdateStatus()`, `useCheckForUpdates()`, `useInstallUpdate()`, `useUpdateReadyEvent()`, `useUpdatePrompt()`.

- [ ] **Step 1: Write the failing test for the deferral accessors**

Create `apps/desktop/aegis-desktop/src/test/features/updater/update-deferral.test.tsx`, following the store-mocking pattern in `src/test/features/settings/settings-persist.test.tsx` — mock `@tauri-apps/plugin-store` with an in-memory `Map` *before* importing the module under test, and export the map so the test can seed it:

```tsx
import { beforeEach, describe, expect, it, vi } from "vitest";

const store = new Map<string, unknown>();
vi.mock("@tauri-apps/plugin-store", () => ({
  load: () => Promise.resolve({
    get: (k: string) => Promise.resolve(store.get(k)),
    set: (k: string, v: unknown) => { store.set(k, v); return Promise.resolve(); },
    delete: (k: string) => { store.delete(k); return Promise.resolve(); },
    save: () => Promise.resolve(),
  }),
}));

import {
  readUpdateDeferral,
  writeUpdateDeferral,
} from "../../../features/settings";

beforeEach(() => store.clear());

describe("the update deferral", () => {
  it("round-trips the deferred version through the settings store", async () => {
    expect(await readUpdateDeferral()).toBeNull();
    await writeUpdateDeferral("0.1.1");
    expect(await readUpdateDeferral()).toBe("0.1.1");
    await writeUpdateDeferral(null);
    expect(await readUpdateDeferral()).toBeNull();
  });

  it("writes the deferral under a namespaced key", async () => {
    await writeUpdateDeferral("0.1.1");
    expect(store.get("update.deferredVersion")).toBe("0.1.1");
  });
});
```

- [ ] **Step 2: Write the failing test for the hooks**

Create `apps/desktop/aegis-desktop/src/test/features/updater/update-status.test.tsx`, using `renderWithQueryClient` from `src/test/helpers/render-with-query-client.tsx` as the wrapper and `mockCommands` / `mockInvoke` from `src/test/helpers/tauri-mock.ts`:

```tsx
import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

// In-memory pub-sub for events, so the test can fire the payload
// directly — the same pattern as
// src/test/features/settings/settings-persist.test.tsx.
type Handler = (e: { payload: unknown }) => void;
const handlers: Handler[] = [];
vi.mock("@tauri-apps/api/event", () => ({
  listen: (_name: string, h: Handler) => {
    handlers.push(h);
    return Promise.resolve(() => {
      const idx = handlers.indexOf(h);
      if (idx >= 0) handlers.splice(idx, 1);
    });
  },
  emit: vi.fn(),
}));

import {
  useCheckForUpdates,
  useInstallUpdate,
  useUpdateReadyEvent,
  useUpdateStatus,
} from "../../../features/updater";
import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { renderWithQueryClient } from "../../helpers/render-with-query-client";

describe("the updater hooks", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    handlers.length = 0;
  });

  it("reads the status through the update_status command", async () => {
    mockCommands({
      update_status: () => ({
        status: { kind: "ready", version: "0.1.1", notes: null, date: null },
        currentVersion: "0.1.0",
        lastCheckedAt: "2026-11-18T06:05:10Z",
      }),
    });
    const { result } = renderHook(() => useUpdateStatus(), {
      wrapper: renderWithQueryClient,
    });
    await waitFor(() => expect(result.current.data?.status.kind).toBe("ready"));
    expect(result.current.data?.currentVersion).toBe("0.1.0");
  });

  it("checkForUpdates posts to the check_for_updates command", async () => {
    mockCommands({
      check_for_updates: () => ({
        status: { kind: "idle" },
        currentVersion: "0.1.0",
        lastCheckedAt: null,
      }),
    });
    const { result } = renderHook(() => useCheckForUpdates(), {
      wrapper: renderWithQueryClient,
    });
    await act(async () => {
      await result.current.mutateAsync();
    });
    expect(mockInvoke).toHaveBeenCalledWith("check_for_updates");
  });

  it("installReadyUpdate posts to the install_ready_update command", async () => {
    mockCommands({ install_ready_update: () => undefined });
    const { result } = renderHook(() => useInstallUpdate(), {
      wrapper: renderWithQueryClient,
    });
    await act(async () => {
      await result.current.mutateAsync();
    });
    expect(mockInvoke).toHaveBeenCalledWith("install_ready_update");
  });

  it("invalidates the status query when the ready event fires", async () => {
    mockCommands({
      update_status: () => ({
        status: { kind: "idle" },
        currentVersion: "0.1.0",
        lastCheckedAt: null,
      }),
    });
    function Probe() {
      useUpdateReadyEvent();
      return null;
    }
    const { client } = renderWithQueryClient(<Probe />);
    await waitFor(() => expect(handlers.length).toBe(1));
    const spy = vi.spyOn(client, "invalidateQueries");
    await act(async () => {
      handlers[0]({ payload: { version: "0.1.1" } });
    });
    expect(spy).toHaveBeenCalledWith(
      expect.objectContaining({ queryKey: ["updater", "status"] }),
    );
  });
});
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/updater/`
Expected: FAIL — the modules and functions do not exist.

- [ ] **Step 4: Add the TS types**

In `apps/desktop/aegis-desktop/src/shared/api/types.ts`, add:

```ts
// Mirrors `updater::status::UpdateStatus`. The Rust enum is tagged with
// `kind = "camelCase"`, so the wire form is
// `{ kind: "ready", version: "...", ... }`.
export type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | {
      kind: "downloading";
      version: string;
      downloaded: number;
      total: number | null;
    }
  | { kind: "ready"; version: string; notes: string | null; date: string | null }
  | { kind: "installing"; version: string }
  | { kind: "failed"; message: string };

// Mirrors `updater::status::UpdateStatusDto`.
export interface UpdateStatusDto {
  status: UpdateStatus;
  currentVersion: string;
  lastCheckedAt: string | null;
}
```

- [ ] **Step 5: Add the API wrappers**

In `apps/desktop/aegis-desktop/src/shared/api/index.ts`, add to the `api` object:

```ts
  // updater
  updateStatus: (): Promise<UpdateStatusDto> =>
    call<UpdateStatusDto>("update_status"),
  checkForUpdates: (): Promise<UpdateStatusDto> =>
    call<UpdateStatusDto>("check_for_updates"),
  installReadyUpdate: (): Promise<void> =>
    call<void>("install_ready_update"),
```

and add `UpdateStatus` and `UpdateStatusDto` to the type imports at the top and the type re-exports at the bottom of the file.

- [ ] **Step 6: Add the query key**

In `apps/desktop/aegis-desktop/src/shared/query/keys.ts`, add to the `queryKeys` object:

```ts
  updater: {
    status: () => ["updater", "status"] as const,
  },
```

- [ ] **Step 7: Implement the deferral accessors**

In `apps/desktop/aegis-desktop/src/features/settings/data/persist.ts`, add:

```ts
/**
 * The settings-store key holding the version whose update prompt the
 * user deferred. One key for the whole app: the prompt is about the
 * application, not about a project.
 */
const updateDeferredKey = "update.deferredVersion";

/**
 * Read the version whose update prompt was deferred, or `null` when
 * nothing has been deferred.
 */
export async function readUpdateDeferral(): Promise<string | null> {
  const store = await getStore();
  const value = await store.get<string | null>(updateDeferredKey);
  return typeof value === "string" ? value : null;
}

/**
 * Write the deferred version. `null` clears it.
 */
export async function writeUpdateDeferral(
  version: string | null,
): Promise<void> {
  const store = await getStore();
  if (version === null) {
    await store.delete(updateDeferredKey);
  } else {
    await store.set(updateDeferredKey, version);
  }
  await store.save();
}
```

In `apps/desktop/aegis-desktop/src/features/settings/index.ts`, add `readUpdateDeferral` and `writeUpdateDeferral` to the re-export list from `./data/persist`.

- [ ] **Step 8: Implement the hooks**

Create `apps/desktop/aegis-desktop/src/features/updater/data/update-status.ts` with four hooks:

- **`useUpdateStatus(): UseQueryResult<UpdateStatusDto, ApiError>`** —
  `useQuery` with `queryKey: queryKeys.updater.status()` and
  `queryFn: () => api.updateStatus()`. The one non-obvious option is
  `refetchInterval`:

  ```ts
  refetchInterval: (query) =>
    query.state.data?.status.kind === "downloading" ? 2000 : false,
  ```

  It polls only while a download is running: the global client sets
  `staleTime: Infinity`, so without this the progress bar would never
  move. Once the download settles the polling stops and the status is
  refreshed only by the `aegis:update-ready` event.

- **`useCheckForUpdates(): UseMutationResult<UpdateStatusDto, ApiError, void>`** —
  `useMutation` with `mutationFn: () => api.checkForUpdates()` and an
  `onSuccess` that writes the returned DTO into the cache with
  `qc.setQueryData(queryKeys.updater.status(), dto)`.

- **`useInstallUpdate(): UseMutationResult<void, ApiError, void>`** —
  `useMutation` with `mutationFn: () => api.installReadyUpdate()`. No
  `onSuccess`: on Windows the process exits inside the call, so this
  promise never resolves there.

- **`useUpdateReadyEvent(): void`** — subscribes to `aegis:update-ready`
  and invalidates the status query, so a release that finishes
  downloading after the page mounted is picked up. The effect
  dynamically imports `@tauri-apps/api/event` and calls `listen`,
  storing the returned unlisten function and calling it in the cleanup
  — the same shape as `useListenForSettingsChanges` in
  `features/settings/data/persist.ts`. The handler body is
  `void qc.invalidateQueries({ queryKey: queryKeys.updater.status() })`.

Create `apps/desktop/aegis-desktop/src/features/updater/data/update-prompt.ts`:

```ts
import { useCallback, useEffect, useState } from "react";

import { readUpdateDeferral, writeUpdateDeferral } from "../../settings";
import { useUpdateStatus } from "./update-status";

/**
 * Whether the update dialog should be open.
 *
 * The prompt is derived from three facts: the status says a release is
 * ready, the deferral has actually been read from the store, and the
 * ready version is not the deferred one. The deferral must be loaded
 * first — otherwise the dialog flashes open on every launch for a
 * release the user already deferred.
 */
export function useUpdatePrompt() {
  const status = useUpdateStatus();
  const [deferredVersion, setDeferredVersion] = useState<string | null>(null);
  const [deferralLoaded, setDeferralLoaded] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const version = await readUpdateDeferral();
      if (cancelled) return;
      setDeferredVersion(version);
      setDeferralLoaded(true);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const ready =
    status.data?.status.kind === "ready" ? status.data.status : null;
  const shouldPrompt =
    deferralLoaded && ready !== null && ready.version !== deferredVersion;

  const defer = useCallback(async () => {
    if (!ready) return;
    setDeferredVersion(ready.version);
    await writeUpdateDeferral(ready.version);
  }, [ready]);

  return { ready, shouldPrompt, defer, status };
}
```

Create `apps/desktop/aegis-desktop/src/features/updater/index.ts`:

```ts
// Public API of the updater feature.

export {
  useUpdateStatus,
  useCheckForUpdates,
  useInstallUpdate,
  useUpdateReadyEvent,
} from "./data/update-status";
export { useUpdatePrompt } from "./data/update-prompt";
```

- [ ] **Step 9: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/updater/`
Expected: PASS.

- [ ] **Step 10: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: no errors.

- [ ] **Step 11: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/ \
        apps/desktop/aegis-desktop/src/features/settings/ \
        apps/desktop/aegis-desktop/src/features/updater/ \
        apps/desktop/aegis-desktop/src/test/features/updater/
git commit -m "feat(desktop): add the updater data layer and deferral store"
```

---

### Task 6: The updater UI — the dialog, the Settings row, and the i18n keys

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/updater/components/UpdateReadyDialog.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/updater/components/UpdateStatusRow.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/updater/components/index.ts`
- Modify: `apps/desktop/aegis-desktop/src/features/updater/index.ts` (export the components)
- Modify: `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx` (mount the dialog)
- Modify: `apps/desktop/aegis-desktop/src/features/settings/pages/SettingsPage.tsx` (render the row)
- Modify: `lib/packages/ui/src/i18n/locales/en.ts` (add the keys)
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts` (add the keys)
- Test: `apps/desktop/aegis-desktop/src/test/features/updater/update-ready-dialog.test.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/updater/update-status-row.test.tsx`

**Interfaces:**
- Consumes: `useUpdatePrompt`, `useInstallUpdate`, `useUpdateStatus`, `useCheckForUpdates` (Task 5); the i18n keys added in this task.
- Produces:
  - `UpdateReadyDialog` — no props; owns its hooks.
  - `UpdateStatusRow` — props `status: UpdateStatus | undefined`, `currentVersion: string`, `lastCheckedAt: string | null`, `onCheck: () => void`, `onInstall: () => void`, `checking: boolean`, `installing: boolean`, `errorMessage: string | null`.

- [ ] **Step 1: Add the i18n keys**

In `lib/packages/ui/src/i18n/locales/en.ts`, add:

```ts
  'updater.title': 'Aegis {version} is ready to install',
  'updater.message':
    'The update has been downloaded and verified. Install it now, or do it later from Settings.',
  'updater.action.install': 'Update now',
  'updater.action.later': 'Later',
  'updater.action.check': 'Check for updates',
  'updater.status.current': 'Installed version: {version}',
  'updater.status.checking': 'Checking for updates…',
  'updater.status.downloading': 'Downloading update… {percent}%',
  'updater.status.ready': 'Update available: {version}',
  'updater.status.upToDate': 'Aegis {version} is up to date.',
  'updater.status.failed': 'Update check failed: {message}',
  'updater.error.installFailed': 'Failed to install the update: {message}',
```

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, add the same keys with Chinese values:

```ts
  'updater.title': 'Aegis {version} 已准备好安装',
  'updater.message': '更新已下载并验证通过。可立即安装，或稍后在设置中安装。',
  'updater.action.install': '立即更新',
  'updater.action.later': '稍后',
  'updater.action.check': '检查更新',
  'updater.status.current': '已安装版本：{version}',
  'updater.status.checking': '正在检查更新…',
  'updater.status.downloading': '正在下载更新… {percent}%',
  'updater.status.ready': '有新版本可用：{version}',
  'updater.status.upToDate': 'Aegis {version} 已是最新版本。',
  'updater.status.failed': '检查更新失败：{message}',
  'updater.error.installFailed': '安装更新失败：{message}',
```

- [ ] **Step 2: Write the failing dialog test**

Create `apps/desktop/aegis-desktop/src/test/features/updater/update-ready-dialog.test.tsx`. Mock the store and `@tauri-apps/api/core` before importing the component, and wrap the render in `AegisI18nProvider` + `AegisThemeProvider` + a `QueryClientProvider` (the pattern in `src/test/features/crf/crf-mission-assign-drawer.test.tsx`):

```tsx
it("offers Update now and Later for a ready release", async () => {
  mockCommands({
    update_status: () => ({
      status: { kind: "ready", version: "0.1.1", notes: null, date: null },
      currentVersion: "0.1.0",
      lastCheckedAt: "2026-11-18T06:05:10Z",
    }),
    install_ready_update: () => undefined,
  });
  render(<UpdateReadyDialog />, { wrapper });
  expect(
    await screen.findByText("Aegis 0.1.1 is ready to install"),
  ).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Update now" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Later" })).toBeInTheDocument();
});

it("does not open for a release the user already deferred", async () => {
  store.set("update.deferredVersion", "0.1.1");
  mockCommands({
    update_status: () => ({
      status: { kind: "ready", version: "0.1.1", notes: null, date: null },
      currentVersion: "0.1.0",
      lastCheckedAt: "2026-11-18T06:05:10Z",
    }),
  });
  render(<UpdateReadyDialog />, { wrapper });
  await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("update_status"));
  expect(
    screen.queryByText("Aegis 0.1.1 is ready to install"),
  ).not.toBeInTheDocument();
});

it("records the deferral when Later is clicked", async () => {
  mockCommands({
    update_status: () => ({
      status: { kind: "ready", version: "0.1.1", notes: null, date: null },
      currentVersion: "0.1.0",
      lastCheckedAt: "2026-11-18T06:05:10Z",
    }),
  });
  render(<UpdateReadyDialog />, { wrapper });
  await userEvent.click(await screen.findByRole("button", { name: "Later" }));
  await waitFor(() =>
    expect(store.get("update.deferredVersion")).toBe("0.1.1"),
  );
});

it("does not open when the check failed", async () => {
  mockCommands({
    update_status: () => ({
      status: { kind: "failed", message: "release server unreachable" },
      currentVersion: "0.1.0",
      lastCheckedAt: "2026-11-18T06:05:10Z",
    }),
  });
  render(<UpdateReadyDialog />, { wrapper });
  await waitFor(() => expect(mockInvoke).toHaveBeenCalledWith("update_status"));
  expect(
    screen.queryByText("Aegis 0.1.1 is ready to install"),
  ).not.toBeInTheDocument();
});
```

- [ ] **Step 3: Write the failing status-row test**

Create `apps/desktop/aegis-desktop/src/test/features/updater/update-status-row.test.tsx`:

```tsx
it("shows the ready version and an install button", () => {
  render(
    <UpdateStatusRow
      status={{ kind: "ready", version: "0.1.1", notes: null, date: null }}
      currentVersion="0.1.0"
      lastCheckedAt="2026-11-18T06:05:10Z"
      onCheck={() => {}}
      onInstall={() => {}}
      checking={false}
      installing={false}
      errorMessage={null}
    />,
    { wrapper },
  );
  expect(screen.getByText("Update available: 0.1.1")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Update now" })).toBeInTheDocument();
});

it("says nothing is newer only after a check has completed", () => {
  render(
    <UpdateStatusRow
      status={{ kind: "idle" }}
      currentVersion="0.1.0"
      lastCheckedAt={null}
      onCheck={() => {}}
      onInstall={() => {}}
      checking={false}
      installing={false}
      errorMessage={null}
    />,
    { wrapper },
  );
  expect(
    screen.queryByText("Aegis 0.1.0 is up to date."),
  ).not.toBeInTheDocument();
});

it("shows the download progress", () => {
  render(
    <UpdateStatusRow
      status={{
        kind: "downloading",
        version: "0.1.1",
        downloaded: 25,
        total: 100,
      }}
      currentVersion="0.1.0"
      lastCheckedAt={null}
      onCheck={() => {}}
      onInstall={() => {}}
      checking={false}
      installing={false}
      errorMessage={null}
    />,
    { wrapper },
  );
  expect(screen.getByText("Downloading update… 25%")).toBeInTheDocument();
});
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/updater/`
Expected: FAIL — the components do not exist.

- [ ] **Step 5: Implement the dialog**

Create `apps/desktop/aegis-desktop/src/features/updater/components/UpdateReadyDialog.tsx`.

The component takes no props and owns its hooks:

```tsx
const { t } = useI18n();
const { ready, shouldPrompt, defer } = useUpdatePrompt();
const install = useInstallUpdate();
// One listener for the whole window: the status query it invalidates
// is shared with the Settings row.
useUpdateReadyEvent();
```

`version` is `ready?.version ?? ""`. It renders a MUI `Dialog` with
`open={shouldPrompt}` and `onClose={handleLater}`, where `handleLater` is
`() => { void defer(); }`, and `aria-label={t("updater.title", { version })}`.

Inside the dialog:

- `DialogTitle` — `t("updater.title", { version })`.
- `DialogContent` — a `DialogContentText` with `t("updater.message")`,
  and, when `install.isError`, an `Alert severity="error"` with
  `sx={{ mt: 2 }}` carrying
  `t("updater.error.installFailed", { message: errorMessage(install.error) })`.
  `errorMessage` comes from `src/shared/api/error`.
- `DialogActions` — a `Button` labelled `t("updater.action.later")` that
  calls `handleLater`, and a `Button variant="contained"` labelled
  `t("updater.action.install")` that calls `install.mutateAsync()`. Both
  are disabled while `install.isPending`.

- [ ] **Step 6: Implement the status row**

Create `apps/desktop/aegis-desktop/src/features/updater/components/UpdateStatusRow.tsx`.

Props:

```ts
export interface UpdateStatusRowProps {
  status: UpdateStatus | undefined;
  currentVersion: string;
  lastCheckedAt: string | null;
  onCheck: () => void;
  onInstall: () => void;
  checking: boolean;
  installing: boolean;
  errorMessage: string | null;
}
```

The component is presentational: it imports no API module and no hook. It
lays out a `Box` with `display: "flex"`, `flexDirection: "column"` and
`gap: 1`, containing, in this order:

1. Always — a `Typography variant="body2"` with
   `t("updater.status.current", { version: currentVersion })`.
2. When `status.kind === "checking"` — a `Typography variant="body2"` with
   `t("updater.status.checking")`.
3. When `status.kind === "downloading"` — a `Typography variant="body2"`
   with `t("updater.status.downloading", { percent })`, followed by a
   `LinearProgress variant="determinate" value={percent}`. The percentage
   is `Math.round((status.downloaded / status.total) * 100)` when `total`
   is set, and `0` otherwise.
4. When `status.kind === "ready"` — a `Typography variant="body2"` with
   `t("updater.status.ready", { version: status.version })`, followed by a
   `Button variant="contained"` labelled `t("updater.action.install")`,
   calling `onInstall` and disabled while `installing` is true.
5. When `status.kind === "idle"` **and** `lastCheckedAt !== null` — a
   `Typography variant="body2"` with
   `t("updater.status.upToDate", { version: currentVersion })`. The
   `lastCheckedAt` guard is what stops the row claiming the app is up to
   date before the first check has ever run.
6. When `status.kind === "failed"` — a `Typography variant="body2"` with
   `color="error"` and `t("updater.status.failed", { message: status.message })`.
7. When `errorMessage !== null` — an `Alert severity="error"` carrying it.
8. Always — a `Button variant="outlined"` labelled
   `t("updater.action.check")`, calling `onCheck` and disabled while
   `checking` is true.

All strings come from `useI18n()`. All MUI primitives come from
`@aegis/ui/mui`.

- [ ] **Step 7: Wire the components into the barrels and the pages**

Create `apps/desktop/aegis-desktop/src/features/updater/components/index.ts`:

```ts
export { UpdateReadyDialog } from "./UpdateReadyDialog";
export { UpdateStatusRow, type UpdateStatusRowProps } from "./UpdateStatusRow";
```

and append to `apps/desktop/aegis-desktop/src/features/updater/index.ts`:

```ts
export { UpdateReadyDialog, UpdateStatusRow } from "./components";
```

In `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx`, import and mount the dialog:

```tsx
import { UpdateReadyDialog } from "../../updater";
```

and add `<UpdateReadyDialog />` inside the existing `<Box component="main" …>` element, before `<Outlet />`.

In `apps/desktop/aegis-desktop/src/features/settings/pages/SettingsPage.tsx`:
- import `useCheckForUpdates`, `useInstallUpdate`, `useUpdateStatus` from `"../../updater"` and `UpdateStatusRow` from `"../../updater"`;
- call the three hooks in `SettingsPage`;
- render the row below the language `FormControl`:

```tsx
      <UpdateStatusRow
        status={updateStatus.data?.status}
        currentVersion={updateStatus.data?.currentVersion ?? ""}
        lastCheckedAt={updateStatus.data?.lastCheckedAt ?? null}
        onCheck={() => void checkForUpdates.mutate()}
        onInstall={() => void installUpdate.mutate()}
        checking={checkForUpdates.isPending}
        installing={installUpdate.isPending}
        errorMessage={
          checkForUpdates.isError
            ? errorMessage(checkForUpdates.error)
            : installUpdate.isError
              ? errorMessage(installUpdate.error)
              : null
        }
      />
```

- [ ] **Step 8: Pin the mount points**

`AppLayout` is the only route component that mounts the dialog, and
`ProjectWorkspaceLayout` must not. Add to the existing
`src/test/features/app/app-layout.test.tsx` an assertion that the update
dialog is rendered by the authenticated shell, and to the existing
`src/test/features/project-workspace/project-workspace-layout.test.tsx` an
assertion that it is not rendered by the workspace shell. Follow each
file's existing setup — both already mock the Tauri commands they need.

- [ ] **Step 9: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/updater/`
Expected: PASS.

- [ ] **Step 10: Typecheck and build**

Run: `pnpm --filter aegis-desktop typecheck && pnpm --filter aegis-desktop build`
Expected: no errors.

- [ ] **Step 11: Run the full frontend suite**

Run: `pnpm --filter aegis-desktop test`
Expected: PASS, including the existing `app-layout` tests (the dialog is mounted in `AppLayout`, so those tests must still pass).

- [ ] **Step 12: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/ \
        apps/desktop/aegis-desktop/src/test/features/updater/ \
        lib/packages/ui/src/i18n/locales/
git commit -m "feat(desktop): add the update dialog, the settings row and the i18n keys"
```

---

### Task 7: End-to-end verification against the local release server

**Files:**
- No source changes. This task verifies the built feature.

**Interfaces:**
- Consumes: the whole feature (Tasks 1–6) and a release server serving a **corrected** manifest for version `0.1.1`.
- Produces: a verified feature.

**Prerequisite (owned by the user, outside this repository):** the release server at `http://localhost:3000` must serve a manifest whose `platforms` key is spelled correctly, whose `version` is `0.1.1`, and whose `url` points at a **genuine 0.1.1 build** signed with the key matching the pubkey in `src-tauri/tauri.conf.json`. The artifact must not be the 0.1.0 installer relabelled — installing that would put 0.1.0 back and make the app re-offer the update forever.

- [ ] **Step 1: Run the full verification gate**

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build
```

Expected: all pass.

- [ ] **Step 2: Confirm the manifest the server serves is the one the plugin accepts**

Run: `curl -s http://localhost:3000/aegis-desktop/releases/manifest.json`
Expected: the JSON has a `platforms` key (not `plaform`) and `"version": "0.1.1"`.

- [ ] **Step 3: Run the app and watch the first check**

Run: `pnpm --filter aegis-desktop tauri dev`
Expected: the log shows a check completing, and the app's Settings page shows either "Update available: 0.1.1" or "Downloading update… N%". If the check failed, the Settings row shows "Update check failed: …" and the app stays usable.

- [ ] **Step 4: Verify the prompt appears once and the deferral persists**

Log in, open Settings, and confirm the "Update available" row is present. Reload the webview (or restart the app) and confirm the dialog does not appear for the deferred version and the Settings row still shows the update.

- [ ] **Step 5: Verify the artifact is not re-downloaded on the next launch**

Restart the app and watch the download progress. The artifact must come from the on-disk cache: the download must not run again, and the status must reach "ready" without a transfer.

- [ ] **Step 6: Verify the install and the restart**

Click "Update now" on the Settings row. Expected: the NSIS installer launches in passive mode and the app restarts into version 0.1.1. Confirm the restarted app reports `0.1.1` as its current version and no longer offers an update.

- [ ] **Step 7: Commit any fixes the verification surfaced**

If the verification found a bug, fix it, re-run the gate from Step 1, and commit it as its own `fix(desktop): …` commit. If it found none, there is nothing to commit.
