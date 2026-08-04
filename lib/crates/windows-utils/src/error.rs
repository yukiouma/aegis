use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WindowsUtilsError {
    #[error("failed to get user name: {0}")]
    UserName(#[source] windows::core::Error),

    #[error("failed to get {kind}")]
    ComputerName {
        kind: ComputerNameKind,
        #[source]
        source: windows::core::Error,
    },

    #[error("failed to open the LSA policy handle")]
    LsaOpenPolicy(#[source] windows::core::Error),

    #[error("failed to query the LSA account domain information")]
    LsaQueryPolicy(#[source] windows::core::Error),

    #[error("LSA returned no account domain SID")]
    MissingDomainSid,

    #[error("failed to convert the account domain SID to a string")]
    ConvertSid(#[source] windows::core::Error),

    #[error("the account domain SID string is not valid UTF-8")]
    SidNotUtf8(#[source] std::string::FromUtf8Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputerNameKind {
    Domain,
    Hostname,
}

impl std::fmt::Display for ComputerNameKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Domain => f.write_str("DNS domain name"),
            Self::Hostname => f.write_str("DNS hostname"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computer_name_kind_display() {
        assert_eq!(ComputerNameKind::Domain.to_string(), "DNS domain name");
        assert_eq!(ComputerNameKind::Hostname.to_string(), "DNS hostname");
    }
}
