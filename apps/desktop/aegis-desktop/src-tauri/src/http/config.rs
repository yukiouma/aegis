//! Compile-time configuration for the outbound HTTP client.

/// Base URL of the aegis-server, baked at build time by `build.rs`.
pub const BASE_URL: &str = env!("AEGIS_SERVER_URL");

/// `(method, path)` pairs that must NOT carry an `Authorization: Bearer` header.
/// Login gates cannot be reached with a stale token; `/healthz` is a public
/// probe; `/api/auth/user-credential` is user-specified to be unauthenticated
/// from the desktop client (see design NO_AUTH_PATHS for the discrepancy with
/// the server's own admin/root gate). `/api/auth/refresh` is added so the
/// auto-refresh path uses the same policy — the server does not enforce
/// Bearer on `/refresh` by design.
pub const NO_AUTH_PATHS: &[(&str, &str)] = &[
    ("POST", "/api/auth/login"),
    ("POST", "/api/auth/login-domain"),
    ("GET",  "/healthz"),
    ("POST", "/api/auth/user-credential"),
    ("POST", "/api/auth/refresh"),
];

/// Returns true if the given `(method, path)` is exempt from Bearer auth.
pub fn is_no_auth(method: &str, path: &str) -> bool {
    NO_AUTH_PATHS.iter().any(|(m, p)| *m == method && *p == path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_is_no_auth() {
        assert!(is_no_auth("POST", "/api/auth/login"));
    }

    #[test]
    fn refresh_is_no_auth() {
        assert!(is_no_auth("POST", "/api/auth/refresh"));
    }

    #[test]
    fn user_list_needs_auth() {
        assert!(!is_no_auth("GET", "/api/user"));
    }

    #[test]
    fn method_mismatch_is_not_no_auth() {
        assert!(!is_no_auth("GET", "/api/auth/login"));
    }
}
