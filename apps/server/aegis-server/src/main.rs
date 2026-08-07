//! aegis-server binary entry point.
//!
//! Loads `.env` (if present), builds [`aegis_server::Config`] from
//! the environment, and hands it to [`aegis_server::run`]. All
//! real work — pool construction, layer wiring, graceful shutdown —
//! lives in the library so it can be exercised by integration tests.

use aegis_server::config::Config;
use aegis_server::run;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load .env (best-effort). `dotenvy` is a no-op when the file
    // is missing, which is the expected production behaviour.
    let _ = dotenvy::dotenv();

    let config = Config::from_env()?;
    run(config).await
}