//! Failure modes of the release-update path.

// The command layer maps these onto `ApiError`, so the conversion lives
// next to the variants it maps.
use crate::http::dto::ApiError;

/// Everything that can go wrong while checking, downloading, verifying or
/// installing a release.
///
/// The variants are split so the command layer can map a transport failure
/// onto `ApiError::Network` and everything else onto `ApiError::Update`.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// The release server could not be reached, or the request failed.
    #[error("update network failure: {message}")]
    Network { message: String },
    /// The response could not be read as a usable release manifest.
    #[error("update manifest invalid: {message}")]
    Invalid { message: String },
    /// The downloaded package failed signature verification.
    #[error("update verification failed: {message}")]
    Verification { message: String },
    /// The installer could not be launched.
    #[error("update install failed: {message}")]
    Install { message: String },
    /// There is no downloaded artifact ready to install.
    #[error("no update is ready to install")]
    NotReady,
}

impl From<UpdateError> for ApiError {
    fn from(err: UpdateError) -> Self {
        match err {
            // A transport failure to the release server is a network
            // failure like any other.
            UpdateError::Network { message } => ApiError::Network { message },
            other => ApiError::Update {
                message: other.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_errors_render_their_messages() {
        assert_eq!(
            UpdateError::Network {
                message: "offline".into()
            }
            .to_string(),
            "update network failure: offline"
        );
        assert_eq!(
            UpdateError::Invalid {
                message: "bad manifest".into()
            }
            .to_string(),
            "update manifest invalid: bad manifest"
        );
        assert_eq!(
            UpdateError::Verification {
                message: "bad signature".into()
            }
            .to_string(),
            "update verification failed: bad signature"
        );
        assert_eq!(
            UpdateError::Install {
                message: "no installer".into()
            }
            .to_string(),
            "update install failed: no installer"
        );
        assert_eq!(
            UpdateError::NotReady.to_string(),
            "no update is ready to install"
        );
    }
}
