//! Read the `sub` claim out of an HS256 access token's payload without
//! signature verification. The token lives in the local token store, so
//! any tampering still fails closed on the next server call.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;

use crate::http::dto::ApiError;

/// Decode the `sub` claim from a JWT payload.
///
/// Splits on `.` (must yield exactly 3 segments), base64-decodes the
/// payload segment (URL-safe, no pad), parses it as JSON, and extracts
/// `sub` as a string. Any malformed token, decode failure, or missing
/// `sub` returns `ApiError::Store { message: ... }` — the local
/// token store is the source of truth on the desktop, so this is a
/// pure read.
pub fn decode_sub(token: &str) -> Result<String, ApiError> {
    let segments: Vec<&str> = token.split('.').collect();
    if segments.len() != 3 {
        return Err(ApiError::Store {
            message: "malformed jwt: expected 3 segments".into(),
        });
    }
    let payload = segments[1];

    let bytes = URL_SAFE_NO_PAD
        .decode(payload.as_bytes())
        .map_err(|e| ApiError::Store { message: format!("base64 decode: {e}") })?;

    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|e| ApiError::Store { message: format!("json parse: {e}") })?;

    value
        .get("sub")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ApiError::Store { message: "missing sub claim".into() })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a JWT with arbitrary header + payload + signature.
    /// The signature is not verified, so any string works.
    fn jwt(header: &str, payload_json: &str, sig: &str) -> String {
        let b64 = |s: &str| URL_SAFE_NO_PAD.encode(s.as_bytes());
        format!("{}.{}.{}", b64(header), b64(payload_json), sig)
    }

    #[test]
    fn decodes_sub_from_well_formed_jwt() {
        let token = jwt(
            r#"{"alg":"HS256","typ":"JWT"}"#,
            r#"{"sub":"alice","role":"admin","ver":1,"exp":0,"iat":0}"#,
            "sig",
        );
        assert_eq!(decode_sub(&token).unwrap(), "alice");
    }

    #[test]
    fn rejects_token_with_wrong_segment_count() {
        let err = decode_sub("only.two").unwrap_err();
        match err {
            ApiError::Store { message } => {
                assert!(message.contains("3 segments"), "got: {message}");
            }
            other => panic!("expected Store, got {other:?}"),
        }
    }

    #[test]
    fn rejects_invalid_base64_payload() {
        // "!!!" is not valid base64; URL_SAFE_NO_PAD rejects it.
        let err = decode_sub("hdr.!!!.sig").unwrap_err();
        assert!(matches!(err, ApiError::Store { .. }));
    }

    #[test]
    fn rejects_payload_without_sub() {
        let token = jwt(
            r#"{"alg":"HS256"}"#,
            r#"{"role":"admin"}"#,
            "sig",
        );
        let err = decode_sub(&token).unwrap_err();
        match err {
            ApiError::Store { message } => {
                assert!(message.contains("sub"), "got: {message}");
            }
            other => panic!("expected Store, got {other:?}"),
        }
    }

    #[test]
    fn rejects_payload_with_non_string_sub() {
        let token = jwt(
            r#"{"alg":"HS256"}"#,
            r#"{"sub":42}"#,
            "sig",
        );
        assert!(matches!(decode_sub(&token), Err(ApiError::Store { .. })));
    }
}