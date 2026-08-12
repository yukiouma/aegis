//! `GET /healthz` — no auth, returns the server's "ok" probe.

use super::client::HttpClient;
use super::dto::ApiError;

pub async fn ping(c: &HttpClient) -> Result<String, ApiError> {
    let bytes = c
        .request_bytes(reqwest::Method::GET, "/healthz", None::<&()>)
        .await?;
    String::from_utf8(bytes).map_err(|e| ApiError::Http {
        status: 0,
        code: "decode_failed".into(),
        message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

    use super::super::client::{HttpClient, MemoryStore};

    struct NoAuth(&'static str);
    impl Match for NoAuth {
        fn matches(&self, req: &Request) -> bool {
            !req.headers.contains_key(self.0)
        }
    }

    #[tokio::test]
    async fn ping_returns_ok() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/healthz"))
                    .and(NoAuth("authorization"))
                    .respond_with(
                        ResponseTemplate::new(200)
                            .insert_header("content-type", "text/plain; charset=utf-8")
                            .set_body_string("ok"),
                    ),
            )
            .await;
        let c = HttpClient::new(server.uri(), store);
        assert_eq!(ping(&c).await.unwrap(), "ok");
    }
}
