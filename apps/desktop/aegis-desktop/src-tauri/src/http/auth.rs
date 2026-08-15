//! Auth-flow HTTP functions: login, login-domain, refresh, logout.

use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;
use crate::system::identity;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub code: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginDomainRequest {
    pub code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenResponse {
    pub access_token: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoutResponse {}

pub async fn login(c: &HttpClient, body: LoginRequest) -> Result<(), ApiError> {
    let bytes = c
        .request_bytes(reqwest::Method::POST, "/api/auth/login", Some(&body))
        .await?;
    let tp: TokenPairResponse = serde_json::from_slice(&bytes).map_err(|e| ApiError::Http {
        status: 0,
        code: "decode_failed".into(),
        message: e.to_string(),
    })?;
    c.tokens().set_access_token(&tp.access_token).await?;
    c.tokens().set_refresh_token(&tp.refresh_token).await?;
    Ok(())
}

/// Log in using the OS-level domain identity. The user code is taken from
/// `identity::current().userid` — the caller supplies nothing.
pub async fn login_domain(c: &HttpClient) -> Result<(), ApiError> {
    let id = identity::current()?;
    let body = LoginDomainRequest {
        code: id.userid,
        domain_name: id.domain,
        hostname: id.host_machine,
        sid: id.sid,
    };
    let bytes = c
        .request_bytes(reqwest::Method::POST, "/api/auth/login-domain", Some(&body))
        .await?;
    let tp: TokenPairResponse = serde_json::from_slice(&bytes).map_err(|e| ApiError::Http {
        status: 0,
        code: "decode_failed".into(),
        message: e.to_string(),
    })?;
    c.tokens().set_access_token(&tp.access_token).await?;
    c.tokens().set_refresh_token(&tp.refresh_token).await?;
    Ok(())
}

pub async fn refresh(c: &HttpClient) -> Result<(), ApiError> {
    let refresh_token = c
        .tokens()
        .refresh_token()
        .await?
        .ok_or(ApiError::RefreshFailed)?;
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Req<'a> {
        refresh_token: &'a str,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Resp {
        access_token: String,
    }
    let url = format!("{}/api/auth/refresh", c.base_url().trim_end_matches('/'));
    let resp = c
        .http()
        .post(&url)
        .json(&Req { refresh_token: &refresh_token })
        .send()
        .await?;
    let status = resp.status();
    let bytes = resp.bytes().await?.to_vec();
    if !status.is_success() {
        c.tokens().clear().await?;
        return Err(ApiError::RefreshFailed);
    }
    let parsed_result: Result<Resp, _> = serde_json::from_slice(&bytes);
    let parsed = match parsed_result {
        Ok(p) => p,
        Err(_) => {
            c.tokens().clear().await.ok();
            return Err(ApiError::RefreshFailed);
        }
    };
    c.tokens().set_access_token(&parsed.access_token).await?;
    Ok(())
}

pub async fn logout(c: &HttpClient) -> Result<(), ApiError> {
    let rt = c.tokens().refresh_token().await?;
    if let Some(refresh_token) = rt {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Req<'a> {
            refresh_token: &'a str,
        }
        let body = Req { refresh_token: &refresh_token };
        // Best-effort server logout; swallow network errors but still clear.
        let _ = c
            .request_bytes(reqwest::Method::POST, "/api/auth/logout", Some(&body))
            .await;
    }
    c.tokens().clear().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::client::{HttpClient, MemoryStore, TokenStore};

    #[tokio::test]
    async fn login_persists_tokens() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/auth/login"))
                    .and(body_json(serde_json::json!({"code": "u", "password": "p"})))
                    .respond_with(ResponseTemplate::new(200).set_body_json(
                        serde_json::json!({"accessToken": "AT", "refreshToken": "RT"})
                    )),
            )
            .await;
        let c = HttpClient::new(server.uri(), store.clone());
        login(
            &c,
            LoginRequest {
                code: "u".into(),
                password: "p".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(store.access_token().await.unwrap().as_deref(), Some("AT"));
        assert_eq!(store.refresh_token().await.unwrap().as_deref(), Some("RT"));
    }

    #[tokio::test]
    async fn login_propagates_401() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        server
            .register(
                Mock::given(method("POST"))
                    .and(path("/api/auth/login"))
                    .respond_with(ResponseTemplate::new(401).set_body_json(
                        serde_json::json!({"code": "invalid_credentials", "message": "bad"})
                    )),
            )
            .await;
        let c = HttpClient::new(server.uri(), store.clone());
        let err = login(
            &c,
            LoginRequest {
                code: "u".into(),
                password: "wrong".into(),
            },
        )
        .await
        .unwrap_err();
        match err {
            ApiError::Http { status, code, .. } => {
                assert_eq!(status, 401);
                assert_eq!(code, "invalid_credentials");
            }
            _ => panic!("got {err:?}"),
        }
        assert_eq!(store.access_token().await.unwrap(), None);
    }

    #[tokio::test]
    async fn logout_clears_tokens() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        let c = HttpClient::new(server.uri(), store.clone());
        // No mock for /api/auth/logout; the best-effort server call may
        // fail but the local clear must still happen.
        let _ = logout(&c).await;
        assert_eq!(store.access_token().await.unwrap(), None);
        assert_eq!(store.refresh_token().await.unwrap(), None);
    }

    /// Compile-time proof that `login_domain` takes only the client — the
    /// user code comes from the OS identity, not the caller. Unlike
    /// `login_domain_propagates_the_identity_error` below, this is not
    /// gated on the target OS, so it catches an arity regression on
    /// Windows too (where a real identity lookup makes a behavioural test
    /// non-deterministic).
    #[allow(dead_code)]
    fn assert_login_domain_takes_only_the_client(c: &HttpClient) {
        let _future = login_domain(c);
    }

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn login_domain_propagates_the_identity_error() {        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        let c = HttpClient::new(server.uri(), store.clone());

        // No `code` argument: the user code now comes from the OS identity.
        let err = login_domain(&c).await.unwrap_err();

        // The error is whatever `identity::current()` returned, not a
        // rewritten one. On non-Windows that is `NotImplemented`.
        match err {
            ApiError::NotImplemented { detail } => {
                assert!(detail.contains("Windows"), "got {detail}");
            }
            other => panic!("expected NotImplemented, got {other:?}"),
        }
        assert!(store.access_token().await.unwrap().is_none());
    }
}
