use serde::{Deserialize, Serialize};

use crate::http::client::HttpClient;
// `LogGuard`, `LogSubmitter`, `TraceIdGenerator` all live in the
// `logging_utils` workspace crate (Cargo name `logging-utils`, reached
// as `logging_utils::*` because Rust replaces `-` with `_`).
// `crate::trace_id_setup` only owns `load_or_create` and
// `DEVICE_PREFIX_FILE_NAME`; the type itself comes from `logging_utils`.
use logging_utils::{LogGuard, LogSubmitter, TraceIdGenerator};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Caller {
    User,
    Agent,
    System,
}

impl std::fmt::Display for Caller {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Caller::User => f.write_str("user"),
            Caller::Agent => f.write_str("agent"),
            Caller::System => f.write_str("system"),
        }
    }
}

pub struct RequestContext {
    pub trace_id: String,
    pub caller: Caller,
}

pub struct SharedAppState {
    http_client: HttpClient,
    trace_id_generator: TraceIdGenerator,
    log_submitter: LogSubmitter,
    log_guard: LogGuard,
}

impl SharedAppState {
    pub fn new(
        http_client: HttpClient,
        trace_id_generator: TraceIdGenerator,
        log_submitter: LogSubmitter,
        log_guard: LogGuard,
    ) -> Self {
        Self {
            http_client,
            trace_id_generator,
            log_submitter,
            log_guard,
        }
    }

    pub fn http_client(&self) -> &HttpClient {
        &self.http_client
    }

    pub fn trace_id_generator(&self) -> &TraceIdGenerator {
        &self.trace_id_generator
    }

    pub fn log_submitter(&self) -> &LogSubmitter {
        &self.log_submitter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_is_lowercase() {
        assert_eq!(format!("{}", Caller::User), "user");
        assert_eq!(format!("{}", Caller::Agent), "agent");
        assert_eq!(format!("{}", Caller::System), "system");
    }

    #[test]
    fn serializes_snake_case() {
        assert_eq!(serde_json::to_string(&Caller::User).unwrap(), "\"user\"");
        assert_eq!(serde_json::to_string(&Caller::Agent).unwrap(), "\"agent\"");
        assert_eq!(serde_json::to_string(&Caller::System).unwrap(), "\"system\"");
    }
}
