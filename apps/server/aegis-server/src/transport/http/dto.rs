//! Wire-level DTOs for the HTTP transport.
//!
//! Each wire DTO is a thin Rust struct with `Serialize`,
//! `Deserialize`, and `ToSchema`. Field names are `snake_case` to
//! match the apis surface. Handler code translates JSON ↔ apis DTOs
//! at the boundary; the apis crate deliberately has no serde /
//! utoipa derives.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// -- requests -------------------------------------------------------------

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub code: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginDomainRequest {
    pub code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

// -- responses ------------------------------------------------------------

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccessTokenResponse {
    pub access_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutResponse {}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuthClaimsResponse {
    pub code: String,
    pub role: Role,
    pub token_version: u32,
}

// -- Role -----------------------------------------------------------------

/// Wire-level mirror of `apis::user::Role`. The two enums have
/// identical variants; the conversion is a single 3-arm `match` in
/// `auth.rs`. Kept separate so the apis crate stays free of
/// serde / utoipa derives.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Root,
    Admin,
    General,
}

impl From<apis::user::Role> for Role {
    fn from(r: apis::user::Role) -> Self {
        match r {
            apis::user::Role::Root => Role::Root,
            apis::user::Role::Admin => Role::Admin,
            apis::user::Role::General => Role::General,
        }
    }
}

impl From<Role> for apis::user::Role {
    fn from(r: Role) -> Self {
        match r {
            Role::Root => apis::user::Role::Root,
            Role::Admin => apis::user::Role::Admin,
            Role::General => apis::user::Role::General,
        }
    }
}

// -- user requests / responses ---------------------------------------------

/// Wire-level request body for `POST /api/user`. Mirrors
/// `apis::user::CreateUserRequest`; the handler translates at the
/// boundary so the apis crate stays free of serde / utoipa.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub code: String,
    pub name: String,
    pub role: Role,
}

/// Wire-level request body for `PATCH /api/user/{code}`. Every field
/// is optional — only the fields that actually changed need to be
/// supplied. Deliberately omits `id`: the handler resolves the URL
/// `{code}` to a `UserView` via `get_by_code` and threads the
/// resulting `id` into `apis::user::UpdateUserRequest` internally.
///
/// Each `Option` field is `skip_serializing_if = "Option::is_none"`
/// so a partial update round-trips losslessly: deserializing
/// `{"name":"Alice"}` and re-serializing it produces the same JSON.
#[derive(Serialize, Deserialize, ToSchema, Default)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

/// Wire-level extractor for the `{code}` URL parameter.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PathCode {
    pub code: String,
}

/// Wire-level projection of a user — mirrors `apis::user::UserView`
/// field-for-field. Carries `Serialize` / `Deserialize` / `ToSchema`
/// so utoipa can document the response shape and the handler can
/// return it directly via `Json<UserViewResponse>`.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserViewResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Wire-level wrapper for `GET /api/user` responses. Wrapping the
/// vector in a struct leaves room for future pagination metadata
/// (`total`, `next_cursor`, …) without breaking the response shape.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserListResponse {
    pub users: Vec<UserViewResponse>,
}

impl From<apis::user::UserView> for UserViewResponse {
    fn from(view: apis::user::UserView) -> Self {
        Self {
            id: view.id,
            code: view.code,
            name: view.name,
            role: view.role.into(),
            active: view.active,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

// -- user-credential requests / responses -----------------------------------

/// Wire-level request body for `PATCH /api/auth/user-credential`.
///
/// `password` is the only mutable field today. `user_code` is
/// implied by [`AuthClaims`](crate::transport::http::auth::middleware::AuthClaims)
/// — a user can only update their own credential. The
/// `skip_serializing_if` keeps a partial update round-trip lossless
/// (a `{}` body stays `{}` on re-serialization).
///
/// There is no `CreateUserCredentialRequest` — credential creation
/// happens out of band (seed script / admin tool), so this route
/// only handles rotation of an existing password.
#[derive(Serialize, Deserialize, ToSchema, Default)]
pub struct UpdateUserCredentialRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// Wire-level projection of a user's credential.
///
/// `password_hash` is always a hashed representation (Argon2 in the
/// canonical backend) — the wire API never exposes the plaintext
/// password. The handler translates from the apis view via the
/// `From` impl below.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserCredentialViewResponse {
    pub user_code: String,
    pub password_hash: String,
    pub token_version: u32,
}

impl From<apis::auth::UserCredentialView> for UserCredentialViewResponse {
    fn from(view: apis::auth::UserCredentialView) -> Self {
        Self {
            user_code: view.user_code,
            password_hash: view.password_hash,
            token_version: view.token_version,
        }
    }
}

/// Wire-level request body for `POST /api/auth/user-credential`.
///
/// Authentication is enforced by the `BearerAuth` middleware; the
/// handler additionally rejects callers whose role is not
/// `Root`/`Admin`. The server hashes `password` before persisting and
/// never returns the value.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RegisterUserRequest {
    pub user_code: String,
    pub user_name: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
    pub password: String,
}

/// Wire-level response body for `POST /api/auth/user-credential`.
///
/// Never includes a password or password hash. Translates from
/// [`apis::auth::RegisterUserResponse`] via the `From` impl below.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RegisterUserResponse {
    pub user_code: String,
    pub user_name: String,
    pub role: Role,
    pub active: bool,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

impl From<apis::auth::RegisterUserResponse> for RegisterUserResponse {
    fn from(view: apis::auth::RegisterUserResponse) -> Self {
        Self {
            user_code: view.user_code,
            user_name: view.user_name,
            role: view.role.into(),
            active: view.active,
            domain_name: view.domain_name,
            hostname: view.hostname,
            sid: view.sid,
        }
    }
}

// -- product requests / responses ------------------------------------------

/// Wire-level request body for `POST /api/product`.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateProductRequest {
    pub code: String,
    pub name: String,
    pub description: String,
}

/// Wire-level request body for `PATCH /api/product/{code}`. Every
/// field is optional; `skip_serializing_if` keeps a partial update
/// round-trip lossless.
#[derive(Serialize, Deserialize, ToSchema, Default)]
pub struct UpdateProductRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

/// Wire-level projection of a product.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProductViewResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::project::ProductView> for ProductViewResponse {
    fn from(view: apis::project::ProductView) -> Self {
        Self {
            id: view.id,
            code: view.code,
            name: view.name,
            description: view.description,
            active: view.active,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProductListResponse {
    pub products: Vec<ProductViewResponse>,
}

// -- project membership DTOs -----------------------------------------------

/// Wire-level request payload for a project's membership. `default`
/// on each vector lets a JSON `{}` deserialize to a present-but-empty
/// membership, which is the difference between "leave alone" and
/// "wipe the team" during project update. `skip_serializing_if` on
/// each vector keeps empty membership objects round-tripping as `{}`
/// rather than `{"leaders":[],"workers":[]}`.
#[derive(Serialize, Deserialize, ToSchema, Default, Clone)]
pub struct ProjectMemberDataRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leaders: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserSummaryViewResponse {
    pub code: String,
    pub name: String,
}

impl From<apis::project::UserSummaryView> for UserSummaryViewResponse {
    fn from(view: apis::project::UserSummaryView) -> Self {
        Self {
            code: view.code,
            name: view.name,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema, Default)]
pub struct ProjectMemberViewResponse {
    pub leaders: Vec<UserSummaryViewResponse>,
    pub workers: Vec<UserSummaryViewResponse>,
}

impl From<apis::project::ProjectMemberView> for ProjectMemberViewResponse {
    fn from(view: apis::project::ProjectMemberView) -> Self {
        Self {
            leaders: view.leaders.into_iter().map(Into::into).collect(),
            workers: view.workers.into_iter().map(Into::into).collect(),
        }
    }
}

// -- project requests / responses ------------------------------------------

/// Wire-level request body for `POST /api/project`.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    pub product_id: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}

/// Wire-level request body for `PATCH /api/project/{code}`.
///
/// Membership fields preserve the missing-vs-empty distinction the
/// usecase relies on: `None` (field absent) leaves the team alone;
/// `Some(empty)` (a present `{}`) wipes the team. Both vector
/// fields use `#[serde(default)]` so a present `{}` deserializes to
/// `Some(ProjectMemberDataRequest { leaders: vec![], workers: vec![] })`
/// rather than failing on missing keys.
#[derive(Serialize, Deserialize, ToSchema, Default)]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProjectViewResponse {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product: ProductViewResponse,
    pub members: ProjectMemberViewResponse,
    pub unblind_members: ProjectMemberViewResponse,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::project::ProjectView> for ProjectViewResponse {
    fn from(view: apis::project::ProjectView) -> Self {
        Self {
            id: view.id,
            code: view.code,
            description: view.description,
            product: view.product.into(),
            members: view.members.into(),
            unblind_members: view.unblind_members.into(),
            active: view.active,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectViewResponse>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_request_roundtrip() {
        let json = r#"{"code":"u1","password":"p"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "u1");
        assert_eq!(req.password, "p");
        let out = serde_json::to_string(&req).unwrap();
        assert_eq!(out, json);
    }

    #[test]
    fn login_domain_request_roundtrip() {
        let json = r#"{"code":"u1","domain_name":"d","hostname":"h","sid":"s"}"#;
        let req: LoginDomainRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "u1");
        assert_eq!(req.domain_name, "d");
        assert_eq!(req.hostname, "h");
        assert_eq!(req.sid, "s");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn refresh_request_roundtrip() {
        let json = r#"{"refresh_token":"r"}"#;
        let req: RefreshRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "r");
    }

    #[test]
    fn logout_request_roundtrip() {
        let json = r#"{"refresh_token":"r"}"#;
        let req: LogoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "r");
    }

    #[test]
    fn token_pair_response_roundtrip() {
        let json = r#"{"access_token":"a","refresh_token":"r"}"#;
        let res: TokenPairResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.access_token, "a");
        assert_eq!(res.refresh_token, "r");
    }

    #[test]
    fn access_token_response_roundtrip() {
        let json = r#"{"access_token":"a"}"#;
        let res: AccessTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.access_token, "a");
    }

    #[test]
    fn logout_response_roundtrip() {
        let res: LogoutResponse = serde_json::from_str("{}").unwrap();
        let out = serde_json::to_string(&res).unwrap();
        assert_eq!(out, "{}");
    }

    #[test]
    fn auth_claims_response_roundtrip() {
        let json = r#"{"code":"u1","role":"admin","token_version":7}"#;
        let res: AuthClaimsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.code, "u1");
        assert!(matches!(res.role, Role::Admin));
        assert_eq!(res.token_version, 7);
        assert_eq!(serde_json::to_string(&res).unwrap(), json);
    }

    #[test]
    fn role_round_trip_all_variants() {
        for r in [Role::Root, Role::Admin, Role::General] {
            let s = serde_json::to_string(&r).unwrap();
            let back: Role = serde_json::from_str(&s).unwrap();
            assert_eq!(format!("{r:?}"), format!("{back:?}"));
        }
    }

    #[test]
    fn role_from_apis_role_all_variants() {
        assert!(matches!(Role::from(apis::user::Role::Root), Role::Root));
        assert!(matches!(Role::from(apis::user::Role::Admin), Role::Admin));
        assert!(matches!(
            Role::from(apis::user::Role::General),
            Role::General
        ));
    }

    // ---- user DTO round-trips (new) -----

    #[test]
    fn create_user_request_roundtrip() {
        let json = r#"{"code":"u1","name":"Alice","role":"admin"}"#;
        let req: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "u1");
        assert_eq!(req.name, "Alice");
        assert!(matches!(req.role, Role::Admin));
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_user_request_partial_roundtrip() {
        let json = r#"{"name":"Alice"}"#;
        let req: UpdateUserRequest = serde_json::from_str(json).unwrap();
        assert!(req.code.is_none());
        assert_eq!(req.name.as_deref(), Some("Alice"));
        assert!(req.role.is_none());
        assert!(req.active.is_none());
        let out = serde_json::to_string(&req).unwrap();
        assert_eq!(out, json);
    }

    #[test]
    fn update_user_request_full_roundtrip() {
        let json = r#"{"code":"u2","name":"Bob","role":"root","active":true}"#;
        let req: UpdateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code.as_deref(), Some("u2"));
        assert_eq!(req.name.as_deref(), Some("Bob"));
        assert!(matches!(req.role, Some(Role::Root)));
        assert_eq!(req.active, Some(true));
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn path_code_roundtrip() {
        let json = r#"{"code":"u1"}"#;
        let p: PathCode = serde_json::from_str(json).unwrap();
        assert_eq!(p.code, "u1");
        assert_eq!(serde_json::to_string(&p).unwrap(), json);
    }

    #[test]
    fn user_view_response_roundtrip() {
        let json = r#"{"id":42,"code":"u1","name":"Alice","role":"admin","active":true,"created_at":"2026-01-02T03:04:05Z","updated_at":"2026-01-02T03:04:05Z"}"#;
        let v: UserViewResponse = serde_json::from_str(json).unwrap();
        assert_eq!(v.id, 42);
        assert_eq!(v.code, "u1");
        assert_eq!(v.name, "Alice");
        assert!(matches!(v.role, Role::Admin));
        assert!(v.active);
        assert_eq!(serde_json::to_string(&v).unwrap(), json);
    }

    #[test]
    fn user_list_response_roundtrip() {
        let json = r#"{"users":[{"id":1,"code":"u1","name":"A","role":"admin","active":true,"created_at":"2026-01-02T03:04:05Z","updated_at":"2026-01-02T03:04:05Z"}]}"#;
        let v: UserListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(v.users.len(), 1);
        assert_eq!(v.users[0].code, "u1");
        assert_eq!(serde_json::to_string(&v).unwrap(), json);
    }

    #[test]
    fn user_view_response_from_apis_user_view() {
        let apis_view = apis::user::UserView {
            id: 7,
            code: "u7".into(),
            name: "Seven".into(),
            role: apis::user::Role::General,
            active: false,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        };
        let resp: UserViewResponse = apis_view.into();
        assert_eq!(resp.id, 7);
        assert_eq!(resp.code, "u7");
        assert_eq!(resp.name, "Seven");
        assert!(matches!(resp.role, Role::General));
        assert!(!resp.active);
    }

    // ---- user-credential DTO round-trips (new) -----

    #[test]
    fn update_user_credential_request_partial_roundtrip() {
        // An empty update body must round-trip losslessly — the
        // handler reads `user_code` from `AuthClaims`, and an
        // absent `password` means "no change".
        let json = r#"{}"#;
        let req: UpdateUserCredentialRequest = serde_json::from_str(json).unwrap();
        assert!(req.password.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_user_credential_request_full_roundtrip() {
        let json = r#"{"password":"hunter2"}"#;
        let req: UpdateUserCredentialRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.password.as_deref(), Some("hunter2"));
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn user_credential_view_response_roundtrip() {
        // The response carries the *hashed* password (Argon2 in the
        // canonical backend) — never the plaintext.
        let json = r#"{"user_code":"u1","password_hash":"argon2id$...","token_version":7}"#;
        let v: UserCredentialViewResponse = serde_json::from_str(json).unwrap();
        assert_eq!(v.user_code, "u1");
        assert_eq!(v.password_hash, "argon2id$...");
        assert_eq!(v.token_version, 7);
        assert_eq!(serde_json::to_string(&v).unwrap(), json);
    }

    #[test]
    fn user_credential_view_response_from_apis_view() {
        let apis_view = apis::auth::UserCredentialView {
            user_code: "u7".into(),
            password_hash: "argon2id$...".into(),
            token_version: 5,
        };
        let resp: UserCredentialViewResponse = apis_view.into();
        assert_eq!(resp.user_code, "u7");
        assert_eq!(resp.password_hash, "argon2id$...");
        assert_eq!(resp.token_version, 5);
    }

    // ---- product DTO round-trips -----

    fn sample_product_view() -> apis::project::ProductView {
        apis::project::ProductView {
            id: 1,
            code: "product-1".into(),
            name: "Atlas".into(),
            description: "core platform".into(),
            active: true,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    fn sample_project_view() -> apis::project::ProjectView {
        apis::project::ProjectView {
            id: 2,
            code: "project-1".into(),
            description: "alpha".into(),
            product: sample_product_view(),
            members: apis::project::ProjectMemberView {
                leaders: vec![apis::project::UserSummaryView {
                    code: "leader-1".into(),
                    name: "Leader One".into(),
                }],
                workers: vec![],
            },
            unblind_members: apis::project::ProjectMemberView {
                leaders: vec![],
                workers: vec![apis::project::UserSummaryView {
                    code: "worker-2".into(),
                    name: "Worker Two".into(),
                }],
            },
            active: true,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    #[test]
    fn create_product_request_roundtrip() {
        let json = r#"{"code":"p1","name":"Atlas","description":"core"}"#;
        let req: CreateProductRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "p1");
        assert_eq!(req.name, "Atlas");
        assert_eq!(req.description, "core");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_product_request_partial_roundtrip() {
        let json = r#"{"name":"Atlas v2"}"#;
        let req: UpdateProductRequest = serde_json::from_str(json).unwrap();
        assert!(req.code.is_none());
        assert_eq!(req.name.as_deref(), Some("Atlas v2"));
        assert!(req.description.is_none());
        assert!(req.active.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_product_request_full_roundtrip() {
        let json = r#"{"code":"p2","name":"B","description":"d","active":false}"#;
        let req: UpdateProductRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code.as_deref(), Some("p2"));
        assert_eq!(req.name.as_deref(), Some("B"));
        assert_eq!(req.description.as_deref(), Some("d"));
        assert_eq!(req.active, Some(false));
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn product_view_response_from_apis_view() {
        let view = sample_product_view();
        let resp: ProductViewResponse = view.into();
        assert_eq!(resp.id, 1);
        assert_eq!(resp.code, "product-1");
        assert_eq!(resp.name, "Atlas");
        assert_eq!(resp.description, "core platform");
        assert!(resp.active);
    }

    #[test]
    fn product_list_response_roundtrip() {
        let json = r#"{"products":[]}"#;
        let resp: ProductListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.products.is_empty());
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    // ---- project DTO round-trips -----

    #[test]
    fn create_project_request_minimal_roundtrip() {
        let json = r#"{"code":"pr1","description":"x","product_id":42}"#;
        let req: CreateProjectRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "pr1");
        assert_eq!(req.description, "x");
        assert_eq!(req.product_id, 42);
        assert!(req.members.is_none());
        assert!(req.unblind_members.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn create_project_request_with_empty_members_roundtrip() {
        let json =
            r#"{"code":"pr1","description":"x","product_id":42,"members":{},"unblind_members":{}}"#;
        let req: CreateProjectRequest = serde_json::from_str(json).unwrap();
        let members = req.members.as_ref().expect("members present");
        assert!(members.leaders.is_empty());
        assert!(members.workers.is_empty());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_project_request_omitted_membership_keeps_none() {
        let json = r#"{"description":"new"}"#;
        let req: UpdateProjectRequest = serde_json::from_str(json).unwrap();
        assert!(req.members.is_none());
        assert!(req.unblind_members.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_project_request_empty_membership_becomes_some_empty() {
        let json = r#"{"members":{},"unblind_members":{}}"#;
        let req: UpdateProjectRequest = serde_json::from_str(json).unwrap();
        let members = req.members.as_ref().expect("members present");
        assert!(members.leaders.is_empty());
        assert!(members.workers.is_empty());
        let unblind = req
            .unblind_members
            .as_ref()
            .expect("unblind members present");
        assert!(unblind.leaders.is_empty());
        assert!(unblind.workers.is_empty());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_project_request_partial_with_membership_roundtrip() {
        let json = r#"{"description":"y","active":false,"members":{"leaders":["l1"]}}"#;
        let req: UpdateProjectRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.description.as_deref(), Some("y"));
        assert_eq!(req.active, Some(false));
        let members = req.members.as_ref().expect("members present");
        assert_eq!(members.leaders, vec!["l1".to_string()]);
        assert!(members.workers.is_empty());
        assert!(req.unblind_members.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn user_summary_view_response_from_apis_view() {
        let view = apis::project::UserSummaryView {
            code: "u1".into(),
            name: "Alice".into(),
        };
        let resp: UserSummaryViewResponse = view.into();
        assert_eq!(resp.code, "u1");
        assert_eq!(resp.name, "Alice");
    }

    #[test]
    fn project_member_view_response_from_apis_view() {
        let view = apis::project::ProjectMemberView {
            leaders: vec![apis::project::UserSummaryView {
                code: "l1".into(),
                name: "Leader".into(),
            }],
            workers: vec![apis::project::UserSummaryView {
                code: "w1".into(),
                name: "Worker".into(),
            }],
        };
        let resp: ProjectMemberViewResponse = view.into();
        assert_eq!(resp.leaders.len(), 1);
        assert_eq!(resp.leaders[0].code, "l1");
        assert_eq!(resp.workers.len(), 1);
        assert_eq!(resp.workers[0].code, "w1");
    }

    #[test]
    fn project_view_response_from_apis_view() {
        let view = sample_project_view();
        let resp: ProjectViewResponse = view.into();
        assert_eq!(resp.id, 2);
        assert_eq!(resp.code, "project-1");
        assert_eq!(resp.product.code, "product-1");
        assert_eq!(resp.members.leaders.len(), 1);
        assert_eq!(resp.members.leaders[0].code, "leader-1");
        assert!(resp.members.workers.is_empty());
        assert!(resp.unblind_members.leaders.is_empty());
        assert_eq!(resp.unblind_members.workers.len(), 1);
        assert_eq!(resp.unblind_members.workers[0].code, "worker-2");
    }

    #[test]
    fn project_list_response_roundtrip() {
        let json = r#"{"projects":[]}"#;
        let resp: ProjectListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.projects.is_empty());
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }
}
