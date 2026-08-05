use super::DomainError;

#[derive(Clone, PartialEq, Eq)]
pub struct DomainIdentity {
    pub user_code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

impl DomainIdentity {
    pub(crate) fn new(
        user_code: String,
        domain_name: String,
        hostname: String,
        sid: String,
    ) -> Result<Self, DomainError> {
        if user_code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        if domain_name.trim().is_empty()
            || hostname.trim().is_empty()
            || sid.trim().is_empty()
        {
            return Err(DomainError::EmptyPasswordHash);
        }
        Ok(Self {
            user_code,
            domain_name,
            hostname,
            sid,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn for_repository(
        user_code: String,
        domain_name: String,
        hostname: String,
        sid: String,
    ) -> Self {
        Self {
            user_code,
            domain_name,
            hostname,
            sid,
        }
    }
}

impl std::fmt::Debug for DomainIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DomainIdentity")
            .field("user_code", &self.user_code)
            .field("domain_name", &self.domain_name)
            .field("hostname", &self.hostname)
            .field("sid", &self.sid)
            .finish()
    }
}