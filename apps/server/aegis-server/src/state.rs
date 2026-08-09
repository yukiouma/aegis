//! Shared state injected into every handler via `axum::extract::State`.

use std::sync::Arc;

/// Shared state injected into every handler.
///
/// Cloned per worker task (axum's `State<T>: Clone` requires it);
/// all services are `Arc`, so the clone is cheap.
#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<dyn apis::auth::AuthService>,
    pub user: Arc<dyn apis::user::UserService>,
    pub project: Arc<dyn apis::project::ProjectService>,
}
