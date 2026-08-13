//! Cross-platform wrapper for the OS-level identity tuple the
//! `loginDomain` command reads at request time. On Windows this calls
//! `windows_utils::get_user_info`; on non-Windows it returns a
//! `NotImplemented` error so the rest of the crate still compiles.

use crate::http::dto::ApiError;

/// Identity tuple that becomes `LoginDomainRequest { code, domain_name,
/// hostname, sid }` after the user fills in `code`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Identity {
    pub domain: String,
    pub host_machine: String,
    pub sid: String,
    pub userid: String,
}

/// Read the current OS identity. Returns `Err(ApiError::NotImplemented)`
/// on non-Windows targets; on Windows, OS-level lookup failures are
/// surfaced as `ApiError::Store` (the closest generic infrastructure
/// variant — there's no `Os` variant in `ApiError`).
pub fn current() -> Result<Identity, ApiError> {
    #[cfg(target_os = "windows")]
    {
        let info = windows_utils::get_user_info()
            .map_err(|e| ApiError::Store { message: e.to_string() })?;
        Ok(Identity {
            domain: info.domain,
            host_machine: info.host_machine,
            sid: info.sid,
            userid: info.userid,
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(ApiError::NotImplemented {
            detail: "OS identity lookup requires Windows",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_fields_are_public_strings() {
        let id = Identity {
            domain: "corp.example".into(),
            host_machine: "ws-001".into(),
            sid: "S-1-5-21-...".into(),
            userid: "alice".into(),
        };
        assert_eq!(id.domain, "corp.example");
        assert_eq!(id.host_machine, "ws-001");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn non_windows_returns_not_implemented() {
        let r = current();
        assert!(matches!(
            r,
            Err(ApiError::NotImplemented { detail }) if detail.contains("Windows")
        ));
    }

    #[test]
    fn identity_serializes_with_snake_case_keys() {
        let id = Identity {
            domain: "corp.example".into(),
            host_machine: "ws-001".into(),
            sid: "S-1-5-21-1234".into(),
            userid: "alice".into(),
        };
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(
            json,
            r#"{"domain":"corp.example","host_machine":"ws-001","sid":"S-1-5-21-1234","userid":"alice"}"#
        );
    }
}