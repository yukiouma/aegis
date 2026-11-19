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

impl UpdateStatus {
    /// The phase of the status, ignoring its payload.
    ///
    /// A download reports its progress through the payload while the
    /// phase stays the same, so the phase is what tells the frontend
    /// whether the status it renders has changed.
    pub fn phase(&self) -> &'static str {
        match self {
            UpdateStatus::Idle => "idle",
            UpdateStatus::Checking => "checking",
            UpdateStatus::Downloading { .. } => "downloading",
            UpdateStatus::Ready { .. } => "ready",
            UpdateStatus::Installing { .. } => "installing",
            UpdateStatus::Failed { .. } => "failed",
        }
    }
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
    /// The message of the most recent failed check.
    ///
    /// It is set even when the status could not become `Failed` — a
    /// check that fails while an update is already held leaves the
    /// status at `Ready`, and this is where that failure is reported.
    pub last_error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
            last_error: Some("update network failure: offline".into()),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["currentVersion"], "0.1.0");
        assert_eq!(json["lastCheckedAt"], "2026-11-18T06:05:10Z");
        assert_eq!(json["lastError"], "update network failure: offline");
        assert_eq!(json["status"]["kind"], "idle");
    }
}
