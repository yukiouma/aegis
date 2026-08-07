use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::state::AppState;

pub struct AuthClaims(pub apis::auth::AuthClaims);

impl FromRequestParts<AppState> for AuthClaims {
    type Rejection = crate::transport::http::error::ApiError;

    async fn from_request_parts(
        _parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        todo!("implemented in Task 9")
    }
}