//! Outbound HTTP client with optional Bearer header and a single auto-refresh
//! retry on 401.

#[cfg(test)]
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::Mutex;

use super::config::is_no_auth;
use super::dto::{ApiError, ErrorBody};

#[async_trait]
pub trait TokenStore: Send + Sync {
    async fn access_token(&self) -> Result<Option<String>, ApiError>;
    async fn refresh_token(&self) -> Result<Option<String>, ApiError>;
    async fn set_access_token(&self, value: &str) -> Result<(), ApiError>;
    async fn set_refresh_token(&self, value: &str) -> Result<(), ApiError>;
    async fn clear(&self) -> Result<(), ApiError>;
}

pub struct TauriStore {
    store: Arc<tauri_plugin_store::Store<tauri::Wry>>,
}

impl TauriStore {
    pub fn new(store: Arc<tauri_plugin_store::Store<tauri::Wry>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl TokenStore for TauriStore {
    async fn access_token(&self) -> Result<Option<String>, ApiError> {
        Ok(self.store.get("access_token").and_then(|v| v.as_str().map(|s| s.to_string())))
    }

    async fn refresh_token(&self) -> Result<Option<String>, ApiError> {
        Ok(self.store.get("refresh_token").and_then(|v| v.as_str().map(|s| s.to_string())))
    }

    async fn set_access_token(&self, value: &str) -> Result<(), ApiError> {
        self.store.set("access_token", serde_json::Value::String(value.to_string()));
        self.store.save().map_err(|e| ApiError::Store { message: e.to_string() })
    }

    async fn set_refresh_token(&self, value: &str) -> Result<(), ApiError> {
        self.store.set("refresh_token", serde_json::Value::String(value.to_string()));
        self.store.save().map_err(|e| ApiError::Store { message: e.to_string() })
    }

    async fn clear(&self) -> Result<(), ApiError> {
        self.store.delete("access_token");
        self.store.delete("refresh_token");
        self.store.save().map_err(|e| ApiError::Store { message: e.to_string() })
    }
}

#[cfg(test)]
#[derive(Default, Debug)]
pub struct MemoryStore {
    inner: Mutex<HashMap<String, String>>,
}

#[cfg(test)]
impl MemoryStore {
    pub fn new() -> Self { Self::default() }
}

#[cfg(test)]
#[async_trait]
impl TokenStore for MemoryStore {
    async fn access_token(&self) -> Result<Option<String>, ApiError> {
        Ok(self.inner.lock().await.get("access_token").cloned())
    }
    async fn refresh_token(&self) -> Result<Option<String>, ApiError> {
        Ok(self.inner.lock().await.get("refresh_token").cloned())
    }
    async fn set_access_token(&self, value: &str) -> Result<(), ApiError> {
        self.inner.lock().await.insert("access_token".into(), value.into());
        Ok(())
    }
    async fn set_refresh_token(&self, value: &str) -> Result<(), ApiError> {
        self.inner.lock().await.insert("refresh_token".into(), value.into());
        Ok(())
    }
    async fn clear(&self) -> Result<(), ApiError> {
        self.inner.lock().await.clear();
        Ok(())
    }
}

pub struct HttpClient {
    http: reqwest::Client,
    base_url: String,
    tokens: Arc<dyn TokenStore>,
    refresh_lock: Arc<Mutex<()>>,
}

impl HttpClient {
    pub fn new(base_url: String, tokens: Arc<dyn TokenStore>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent(concat!("aegis-desktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client builds");
        Self {
            http,
            base_url,
            tokens,
            refresh_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn tokens(&self) -> Arc<dyn TokenStore> {
        Arc::clone(&self.tokens)
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns a clone of the refresh-lock handle so multiple `HttpClient`
    /// instances that share a `TokenStore` can serialize their concurrent
    /// refresh attempts through a single mutex. In production there is only
    /// one client so the handle is unused; tests use it to model a shared
    /// session across two HTTP surfaces that hit the same backend.
    pub fn refresh_lock(&self) -> Arc<Mutex<()>> {
        Arc::clone(&self.refresh_lock)
    }

    #[cfg(test)]
    pub fn with_refresh_lock(
        base_url: String,
        tokens: Arc<dyn TokenStore>,
        refresh_lock: Arc<Mutex<()>>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent(concat!("aegis-desktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client builds");
        Self { http, base_url, tokens, refresh_lock }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    pub async fn request<TReq, TResp>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&TReq>,
    ) -> Result<TResp, ApiError>
    where
        TReq: Serialize + ?Sized,
        TResp: DeserializeOwned,
    {
        let bytes = self.request_bytes(method, path, body).await?;
        serde_json::from_slice::<TResp>(&bytes).map_err(|e| ApiError::Http {
            status: 0,
            code: "decode_failed".into(),
            message: e.to_string(),
        })
    }

    pub async fn request_bytes<TReq>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&TReq>,
    ) -> Result<Vec<u8>, ApiError>
    where
        TReq: Serialize + ?Sized,
    {
        let needs_auth = !is_no_auth(method.as_str(), path);
        let first_token = if needs_auth {
            self.tokens.access_token().await?
        } else {
            None
        };

        let (status, bytes) = self
            .send(method.clone(), path, body, first_token.clone())
            .await?;

        if status.as_u16() == 401 && needs_auth && first_token.is_some() {
            let _guard = self.refresh_lock.lock().await;
            let after_lock = self.tokens.access_token().await?;
            let token_for_retry = match after_lock {
                Some(t) if Some(&t) != first_token.as_ref() => t,
                _ => {
                    self.refresh_with_lock().await?;
                    self.tokens
                        .access_token()
                        .await?
                        .ok_or(ApiError::RefreshFailed)?
                }
            };
            let (retry_status, retry_bytes) = self
                .send(method, path, body, Some(token_for_retry))
                .await?;
            if retry_status.is_success() {
                Ok(retry_bytes)
            } else {
                Err(parse_error(retry_status, retry_bytes))
            }
        } else if status.is_success() {
            Ok(bytes)
        } else {
            Err(parse_error(status, bytes))
        }
    }

    async fn refresh_with_lock(&self) -> Result<(), ApiError> {
        let refresh_token = self
            .tokens
            .refresh_token()
            .await?
            .ok_or(ApiError::RefreshFailed)?;
        #[derive(Serialize)]
        struct Req<'a> {
            refresh_token: &'a str,
        }
        #[derive(serde::Deserialize)]
        struct Resp {
            access_token: String,
        }
        let url = self.url("/api/auth/refresh");
        let resp = self
            .http
            .post(&url)
            .json(&Req {
                refresh_token: &refresh_token,
            })
            .send()
            .await?;
        let status = resp.status();
        let bytes = resp.bytes().await?.to_vec();
        if !status.is_success() {
            self.tokens.clear().await?;
            return Err(ApiError::RefreshFailed);
        }
        let parsed_result: Result<Resp, _> = serde_json::from_slice(&bytes);
        let parsed = match parsed_result {
            Ok(p) => p,
            Err(_) => {
                self.tokens.clear().await.ok();
                return Err(ApiError::RefreshFailed);
            }
        };
        self.tokens.set_access_token(&parsed.access_token).await?;
        Ok(())
    }

    async fn send<TReq>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&TReq>,
        token: Option<String>,
    ) -> Result<(reqwest::StatusCode, Vec<u8>), ApiError>
    where
        TReq: Serialize + ?Sized,
    {
        let url = self.url(path);
        let mut rb = self.http.request(method, &url);
        if let Some(t) = token {
            rb = rb.bearer_auth(t);
        }
        if let Some(b) = body {
            rb = rb.json(b);
        }
        let resp = rb.send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await?.to_vec();
        Ok((status, bytes))
    }
}

fn parse_error(status: reqwest::StatusCode, bytes: Vec<u8>) -> ApiError {
    let parsed: Option<ErrorBody> = serde_json::from_slice(&bytes).ok();
    match parsed {
        Some(b) => ApiError::Http {
            status: status.as_u16(),
            code: b.code,
            message: b.message,
        },
        None => ApiError::Http {
            status: status.as_u16(),
            code: status.canonical_reason().unwrap_or("unknown").into(),
            message: String::from_utf8_lossy(&bytes).into_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

    /// Matcher that fails when the named header is *present* on the request.
    /// Complements `wiremock::matchers::header` (positive match) by
    /// asserting absence. Used to verify that NO_AUTH_PATHS endpoints do
    /// not carry a Bearer token.
    struct NoHeader(&'static str);
    impl Match for NoHeader {
        fn matches(&self, req: &Request) -> bool {
            !req.headers.contains_key(self.0)
        }
    }

    #[derive(Serialize, Debug)]
    struct LoginReq {
        code: String,
        password: String,
    }
    #[derive(Deserialize, Debug, PartialEq)]
    struct TokenPair {
        access_token: String,
        refresh_token: String,
    }

    fn client_for(server: &MockServer, tokens: Arc<MemoryStore>) -> HttpClient {
        HttpClient::new(server.uri(), tokens)
    }

    #[tokio::test]
    async fn bearer_header_attached_on_protected_endpoint() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_AAA").await.unwrap();
        store.set_refresh_token("RT_AAA").await.unwrap();
        let m = Mock::given(method("GET"))
            .and(path("/api/user"))
            .and(header("authorization", "Bearer AT_AAA"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"users": []})));
        server.register(m).await;
        let c = client_for(&server, store);
        let _: serde_json::Value = c.request(reqwest::Method::GET, "/api/user", None::<&()>).await.unwrap();
    }

    #[tokio::test]
    async fn bearer_header_absent_on_login() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        let m = Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .and(NoHeader("authorization"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"access_token": "AT", "refresh_token": "RT"})
            ));
        server.register(m).await;
        let c = client_for(&server, store);
        let bytes = c
            .request_bytes(
                reqwest::Method::POST,
                "/api/auth/login",
                Some(&LoginReq { code: "u".into(), password: "p".into() }),
            )
            .await
            .unwrap();
        let tp: TokenPair = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(tp.access_token, "AT");
        assert_eq!(tp.refresh_token, "RT");
    }

    #[tokio::test]
    async fn bearer_header_absent_on_user_credential() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_AAA").await.unwrap();
        let m = Mock::given(method("POST"))
            .and(path("/api/auth/user-credential"))
            .and(NoHeader("authorization"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({"user_code": "u", "user_name": "n", "role": "general", "active": true, "domain_name": "d", "hostname": "h", "sid": "s"})));
        server.register(m).await;
        let c = client_for(&server, store);
        let _: serde_json::Value = c
            .request(reqwest::Method::POST, "/api/auth/user-credential", None::<&()>)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn request_404_returns_http_error_with_code() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        let m = Mock::given(method("GET"))
            .and(path("/api/user/foo"))
            .respond_with(ResponseTemplate::new(404).set_body_json(
                serde_json::json!({"code": "not_found", "message": "user foo"})
            ));
        server.register(m).await;
        let c = client_for(&server, store);
        let err = c
            .request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user/foo", None)
            .await
            .unwrap_err();
        match err {
            ApiError::Http { status, code, .. } => {
                assert_eq!(status, 404);
                assert_eq!(code, "not_found");
            }
            _ => panic!("expected Http error, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn request_non_json_500_returns_status_text_code() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        let m = Mock::given(method("GET"))
            .and(path("/api/user"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal boom"));
        server.register(m).await;
        let c = client_for(&server, store);
        let err = c
            .request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user", None)
            .await
            .unwrap_err();
        match err {
            ApiError::Http { status, code, message } => {
                assert_eq!(status, 500);
                assert_eq!(code, "Internal Server Error");
                assert!(message.contains("internal boom"));
            }
            _ => panic!("expected Http, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn network_failure_returns_network_error() {
        // Bind a listener, capture its address, drop the listener so the
        // port immediately refuses connections. Then point the client at it.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let url = format!("http://{addr}");
        let store = Arc::new(MemoryStore::default());
        let c = HttpClient::new(url, store);
        let err = c
            .request::<(), serde_json::Value>(reqwest::Method::GET, "/healthz", None)
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::Network { .. }), "got {err:?}");
    }

    #[tokio::test]
    async fn auto_refresh_on_401_retries_with_new_token() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_STALE").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();

        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/auth/refresh"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({"access_token": "AT_NEW"})
                    )),
            )
            .await;

        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/user"))
                    .and(header("authorization", "Bearer AT_STALE"))
                    .respond_with(ResponseTemplate::new(401).set_body_json(
                        serde_json::json!({"code": "token_verification_failed", "message": "expired"})
                    )),
            )
            .await;
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/user"))
                    .and(header("authorization", "Bearer AT_NEW"))
                    .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"users": []}))),
            )
            .await;

        let c = client_for(&server, store.clone());
        let v: serde_json::Value = c
            .request(reqwest::Method::GET, "/api/user", None::<&()>)
            .await
            .unwrap();
        assert_eq!(v["users"].as_array().unwrap().len(), 0);
        assert_eq!(store.access_token().await.unwrap().as_deref(), Some("AT_NEW"));
    }

    #[tokio::test]
    async fn refresh_failure_clears_store_and_returns_refresh_failed() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_STALE").await.unwrap();
        store.set_refresh_token("RT_DEAD").await.unwrap();

        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/auth/refresh"))
                    .respond_with(ResponseTemplate::new(401).set_body_json(
                        serde_json::json!({"code": "token_verification_failed", "message": "dead"})
                    )),
            )
            .await;
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/user"))
                    .respond_with(ResponseTemplate::new(401).set_body_json(
                        serde_json::json!({"code": "token_verification_failed", "message": "expired"})
                    )),
            )
            .await;

        let c = client_for(&server, store.clone());
        let err = c
            .request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user", None)
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::RefreshFailed));
        assert_eq!(store.access_token().await.unwrap(), None);
        assert_eq!(store.refresh_token().await.unwrap(), None);
    }

    #[tokio::test]
    async fn concurrent_401s_share_one_refresh() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_STALE").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();

        let refresh_mock = Mock::given(method("POST"))
            .and(path("/api/auth/refresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"access_token": "AT_NEW"})
            ))
            .expect(1);
        server.register(refresh_mock).await;

        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/user"))
                    .and(header("authorization", "Bearer AT_STALE"))
                    .respond_with(ResponseTemplate::new(401).set_body_json(
                        serde_json::json!({"code": "token_verification_failed", "message": ""})
                    )),
            )
            .await;
        server
            .register(
                Mock::given(method("GET"))
                    .and(path("/api/user"))
                    .and(header("authorization", "Bearer AT_NEW"))
                    .respond_with(
                        ResponseTemplate::new(200).set_body_json(serde_json::json!({"users": []})),
                    ),
            )
            .await;

        let c1 = client_for(&server, store.clone());
        // Share c1's refresh lock with c2 so the two clients serialize
        // through one mutex.
        let c2 = HttpClient::with_refresh_lock(server.uri(), store.clone(), c1.refresh_lock());
        let (r1, r2) = tokio::join!(
            c1.request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user", None),
            c2.request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user", None),
        );
        r1.unwrap();
        r2.unwrap();
    }

    #[tokio::test]
    async fn memory_store_round_trips_tokens() {
        let s = MemoryStore::default();
        assert_eq!(s.access_token().await.unwrap(), None);
        s.set_access_token("AT").await.unwrap();
        s.set_refresh_token("RT").await.unwrap();
        assert_eq!(s.access_token().await.unwrap().as_deref(), Some("AT"));
        assert_eq!(s.refresh_token().await.unwrap().as_deref(), Some("RT"));
        s.clear().await.unwrap();
        assert_eq!(s.access_token().await.unwrap(), None);
    }
}
