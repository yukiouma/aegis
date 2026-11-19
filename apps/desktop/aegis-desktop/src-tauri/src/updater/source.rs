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

#[async_trait]
impl UpdateSource for PluginUpdateSource {
    async fn check(&self) -> Result<Option<Arc<dyn PendingRelease>>, UpdateError> {
        let updater = self.handle.updater().map_err(|e| UpdateError::Invalid {
            message: e.to_string(),
        })?;
        let update = updater.check().await.map_err(classify_error)?;
        Ok(update.map(|u| Arc::new(PluginRelease::new(u)) as Arc<dyn PendingRelease>))
    }
}

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
        // The plugin's callback is `FnMut(usize, Option<u64>)`; ours is
        // `Fn(u64, Option<u64>)`, so the chunk length is widened here.
        self.update
            .download(
                |chunk_len, total| on_progress(chunk_len as u64, total),
                || {},
            )
            .await
            .map_err(|e| UpdateError::Network {
                message: e.to_string(),
            })
    }

    fn verify(&self, bytes: &[u8]) -> Result<(), UpdateError> {
        self.update
            .verify(bytes)
            .map_err(|e| UpdateError::Verification {
                message: e.to_string(),
            })
    }

    fn install(&self, bytes: &[u8]) -> Result<(), UpdateError> {
        self.update
            .install(bytes)
            .map_err(|e| UpdateError::Install {
                message: e.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_plugin_updater::RemoteRelease;

    /// The plugin parses the release manifest into `RemoteRelease`, which
    /// requires a `platforms` map keyed by target (or a top-level `url` +
    /// `signature`). The release server served `plaform` — a
    /// misspelling — which the parser ignores, leaving `platforms` absent
    /// and failing with "the `url` field was not set on the updater
    /// response". This test pins the shape the desktop depends on.
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
        let release: RemoteRelease =
            serde_json::from_value(manifest).expect("manifest should parse");
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
        let classified = classify_error(tauri_plugin_updater::Error::Network("status 500".into()));
        assert!(
            matches!(classified, UpdateError::Network { .. }),
            "got {classified:?}"
        );
    }
}
