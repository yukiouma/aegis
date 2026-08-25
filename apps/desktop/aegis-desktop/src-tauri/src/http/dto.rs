//! Cross-resource wire DTOs and the single `ApiError` returned by every command.

use serde::{Deserialize, Serialize};

/// Stable, machine-readable error code returned as part of the server
/// `ErrorBody`. The desktop client does not dispatch on these codes;
/// errors are forwarded to the frontend as opaque `ApiError::Http` records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

/// Three administrative tiers. Wire form is camelCase to match the
/// server's `Role` serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Root,
    Admin,
    General,
}

/// CDISC terminology kind. Wire form is lowercase (`"sdtm"`, `"adam"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TerminologyKind {
    Sdtm,
    Adam,
}

/// SDTM domain category. Wire form is the human-friendly string
/// (`"Special Purpose"`, `"Trial Design"`, …) matching the server's
/// `#[serde(rename = "...")]` attributes on `apis::domain_model::DomainCategory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainCategory {
    #[serde(rename = "Special Purpose")]
    SpecialPurpose,
    #[serde(rename = "Interventions")]
    Interventions,
    #[serde(rename = "Events")]
    Events,
    #[serde(rename = "Findings")]
    Findings,
    #[serde(rename = "Trial Design")]
    TrialDesign,
    #[serde(rename = "Relationships")]
    Relationships,
    #[serde(rename = "Study Reference")]
    StudyReference,
}

/// The single error type every `#[tauri::command]` returns to the frontend.
/// Serialized as a tagged object (`{"kind": "http", ...}` etc.) so the
/// frontend can discriminate by `kind`.
///
/// Note: serde tagged enums do not support newtype variants, so the
/// payload-bearing variants are struct-shaped (`{ message: ... }` etc.).
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ApiError {
    /// Reqwest returned a transport error (DNS, connect, TLS, timeout).
    #[error("network: {message}")]
    Network { message: String },

    /// Server returned a non-2xx response; `code` is the body's stable
    /// machine-readable token (or `status_text` for non-JSON 5xx).
    #[error("http {status} ({code}): {message}")]
    Http {
        status: u16,
        code: String,
        message: String,
    },

    /// Auth refresh failed (or no refresh token left). Frontend should
    /// route to login.
    #[error("refresh failed; please log in")]
    RefreshFailed,

    /// Functionality not available on this platform (e.g. `loginDomain` on
    /// non-Windows).
    #[error("not implemented on this platform: {detail}")]
    NotImplemented { detail: &'static str },

    /// Persistent token-store error.
    #[error("store error: {message}")]
    Store { message: String },

    /// Workbook parse failure (the `terminology` git crate could not read the
    /// .xls/.xlsx file). The frontend renders this through `errorMessage(err)`
    /// the same way as the other variants.
    #[error("workbook parse error: {message}")]
    Parse { message: String },
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::Network { message: err.to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_serializes_camel_case() {
        assert_eq!(serde_json::to_string(&Role::Root).unwrap(), "\"root\"");
        assert_eq!(serde_json::to_string(&Role::Admin).unwrap(), "\"admin\"");
        assert_eq!(serde_json::to_string(&Role::General).unwrap(), "\"general\"");
    }

    #[test]
    fn role_deserializes_camel_case() {
        let r: Role = serde_json::from_str("\"root\"").unwrap();
        assert_eq!(r, Role::Root);
    }

    #[test]
    fn error_body_roundtrip() {
        let body = ErrorBody { code: "validation_failed".into(), message: "bad code".into() };
        let j = serde_json::to_string(&body).unwrap();
        let back: ErrorBody = serde_json::from_str(&j).unwrap();
        assert_eq!(body, back);
    }

    #[test]
    fn api_error_http_serializes_with_kind_tag() {
        let e = ApiError::Http {
            status: 401,
            code: "invalid_credentials".into(),
            message: "nope".into(),
        };
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains("\"kind\":\"http\""), "got {j}");
        assert!(j.contains("\"status\":401"));
        assert!(j.contains("\"code\":\"invalid_credentials\""));
    }

    #[test]
    fn api_error_network_serializes() {
        let e = ApiError::Network { message: "dns".into() };
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains("\"kind\":\"network\""));
        assert!(j.contains("\"message\":\"dns\""));
    }

    #[test]
    fn api_error_refresh_failed_serializes() {
        let e = ApiError::RefreshFailed;
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains("\"kind\":\"refreshFailed\""));
    }

    #[test]
    fn terminology_kind_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&TerminologyKind::Sdtm).unwrap(),
            "\"sdtm\""
        );
        assert_eq!(
            serde_json::to_string(&TerminologyKind::Adam).unwrap(),
            "\"adam\""
        );
    }

    #[test]
    fn terminology_kind_deserializes_lowercase() {
        let k: TerminologyKind = serde_json::from_str("\"sdtm\"").unwrap();
        assert_eq!(k, TerminologyKind::Sdtm);
        let k: TerminologyKind = serde_json::from_str("\"adam\"").unwrap();
        assert_eq!(k, TerminologyKind::Adam);
    }

    #[test]
    fn domain_category_serializes_human_strings() {
        use super::DomainCategory::*;
        assert_eq!(serde_json::to_string(&SpecialPurpose).unwrap(), "\"Special Purpose\"");
        assert_eq!(serde_json::to_string(&Interventions).unwrap(), "\"Interventions\"");
        assert_eq!(serde_json::to_string(&Events).unwrap(), "\"Events\"");
        assert_eq!(serde_json::to_string(&Findings).unwrap(), "\"Findings\"");
        assert_eq!(serde_json::to_string(&TrialDesign).unwrap(), "\"Trial Design\"");
        assert_eq!(serde_json::to_string(&Relationships).unwrap(), "\"Relationships\"");
        assert_eq!(serde_json::to_string(&StudyReference).unwrap(), "\"Study Reference\"");
    }

    #[test]
    fn domain_category_deserializes_human_strings() {
        use super::DomainCategory::*;
        let c: DomainCategory = serde_json::from_str("\"Special Purpose\"").unwrap();
        assert_eq!(c, SpecialPurpose);
        let c: DomainCategory = serde_json::from_str("\"Study Reference\"").unwrap();
        assert_eq!(c, StudyReference);
    }

    #[test]
    fn parse_error_serializes_camel_case() {
        let e = super::ApiError::Parse { message: "no sheet".into() };
        let j = serde_json::to_string(&e).unwrap();
        assert_eq!(j, r#"{"kind":"parse","message":"no sheet"}"#);
    }

    #[test]
    fn parse_error_roundtrips() {
        let e = super::ApiError::Parse { message: "bad row".into() };
        let j = serde_json::to_string(&e).unwrap();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v["kind"], "parse");
        assert_eq!(v["message"], "bad row");
    }
}
