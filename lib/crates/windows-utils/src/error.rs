use thiserror::Error;

#[cfg(target_os = "windows")]
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
