//! Cross-platform wrapper for the OS-level identity tuple the
//! `loginDomain` command reads at request time. On Windows this calls
//! `windows_utils::get_user_info`; on non-Windows it returns a static
//! "not implemented" error so the rest of the crate still compiles.

/// Identity tuple that becomes `LoginDomainRequest { code, domain_name,
/// hostname, sid }` after the user fills in `code`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub domain: String,
    pub host_machine: String,
    pub sid: String,
    pub userid: String,
}

/// Read the current OS identity. Returns `Err` on non-Windows targets so
/// callers (e.g. `http::auth::login_domain`) can translate to
/// `ApiError::NotImplemented`.
pub fn current() -> Result<Identity, String> {
    #[cfg(target_os = "windows")]
    {
        let info = windows_utils::get_user_info().map_err(|e| e.to_string())?;
        Ok(Identity {
            domain: info.domain,
            host_machine: info.host_machine,
            sid: info.sid,
            userid: info.userid,
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("OS identity lookup requires Windows".into())
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
    fn non_windows_returns_err() {
        let r = current();
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Windows"));
    }
}
