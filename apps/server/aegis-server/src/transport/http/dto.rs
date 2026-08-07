//! Wire-level DTOs for the HTTP transport.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub code: String,
    pub password: String,
}