//! Shared state injected into every handler via `axum::extract::State`.

use std::sync::Arc;

/// Shared state injected into every handler.
///
/// Cloned per worker task (axum's `State<T>: Clone` requires it);
/// both fields are `Arc`, so the clone is cheap. The `user` field is
/// held so the auth `UserServiceImpl` can be wired once at startup
/// without a separate registry; future user-CRUD handlers will use it
/// directly.
#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<dyn apis::auth::AuthService>,
    pub user: Arc<dyn apis::user::UserService>,
}