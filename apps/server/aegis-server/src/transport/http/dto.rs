//! Wire-level DTOs for the HTTP transport.
//!
//! Each wire DTO is a thin Rust struct with `Serialize`,
//! `Deserialize`, and `ToSchema`. JSON field names use `camelCase`
//! (`#[serde(rename_all = "camelCase")]`) per the public API
//! conventions. Handler code translates JSON ↔ apis DTOs at the
//! boundary; the apis crate deliberately has no serde / utoipa
//! derives.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// -- requests -------------------------------------------------------------

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub code: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginDomainRequest {
    pub code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequest {
    pub refresh_token: String,
}

// -- responses ------------------------------------------------------------

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccessTokenResponse {
    pub access_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogoutResponse {}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct PathCode {
    pub code: String,
}

/// Wire-level projection of a user — mirrors `apis::user::UserView`
/// field-for-field. Carries `Serialize` / `Deserialize` / `ToSchema`
/// so utoipa can document the response shape and the handler can
/// return it directly via `Json<UserViewResponse>`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

// -- tag DTOs --------------------------------------------------------------

/// Wire-level request body for a single tag. Mirrors
/// `apis::project::TagData` field-for-field. Two strings, both
/// required (and validated non-empty after trim by the domain layer).
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagDataRequest {
    pub key: String,
    pub value: String,
}

/// Wire-level projection of a single tag. Mirrors
/// `apis::project::TagView` field-for-field.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagViewResponse {
    pub key: String,
    pub value: String,
}

impl From<apis::project::TagView> for TagViewResponse {
    fn from(view: apis::project::TagView) -> Self {
        Self {
            key: view.key,
            value: view.value,
        }
    }
}

// -- project membership DTOs -----------------------------------------------

/// Wire-level request payload for a project's membership. `default`
/// on each vector lets a JSON `{}` deserialize to a present-but-empty
/// membership, which is the difference between "leave alone" and
/// "wipe the team" during project update. `skip_serializing_if` on
/// each vector keeps empty membership objects round-tripping as `{}`
/// rather than `{"leaders":[],"workers":[]}`.
#[derive(Serialize, Deserialize, ToSchema, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemberDataRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leaders: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<TagDataRequest>>,
}

/// Wire-level request body for `PATCH /api/project/{code}`.
///
/// Membership fields preserve the missing-vs-empty distinction the
/// usecase relies on: `None` (field absent) leaves the team alone;
/// `Some(empty)` (a present `{}`) wipes the team. Both vector
/// fields use `#[serde(default)]` so a present `{}` deserializes to
/// `Some(ProjectMemberDataRequest { leaders: vec![], workers: vec![] })`
/// rather than failing on missing keys. `tags` follows the same
/// `None`-vs-`Some(empty)` semantics: missing leaves the tag list
/// alone, present-with-vec replaces it whole-list.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<TagDataRequest>>,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectViewResponse {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMemberViewResponse,
    pub unblind_members: ProjectMemberViewResponse,
    pub tags: Vec<TagViewResponse>,
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
            members: view.members.into(),
            unblind_members: view.unblind_members.into(),
            tags: view.tags.into_iter().map(Into::into).collect(),
            active: view.active,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectViewResponse>,
}

// -- terminology kind --------------------------------------------------------

/// Wire-level mirror of `apis::terminology::TerminologyKind`. The two
/// enums have identical variants; the conversion is a single 2-arm
/// `match`. Kept separate so the apis crate stays free of serde /
/// utoipa derives. `Default` is derived (defaulting to `Sdtm`, the
/// more common standard) so query DTOs that wrap it can derive
/// `Default` for serde-deserialization of partial query strings.
#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TerminologyKind {
    #[default]
    Sdtm,
    Adam,
}

impl From<apis::terminology::TerminologyKind> for TerminologyKind {
    fn from(k: apis::terminology::TerminologyKind) -> Self {
        match k {
            apis::terminology::TerminologyKind::Sdtm => TerminologyKind::Sdtm,
            apis::terminology::TerminologyKind::Adam => TerminologyKind::Adam,
        }
    }
}

impl From<TerminologyKind> for apis::terminology::TerminologyKind {
    fn from(k: TerminologyKind) -> Self {
        match k {
            TerminologyKind::Sdtm => apis::terminology::TerminologyKind::Sdtm,
            TerminologyKind::Adam => apis::terminology::TerminologyKind::Adam,
        }
    }
}

// -- terminology view DTOs ---------------------------------------------------

/// Wire-level extractor for the `{id}` URL parameter used by every
/// version / code-list / code-item path.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathId {
    pub id: i64,
}

/// Wire-level projection of a `TerminologyVersion`. Mirrors
/// `apis::terminology::TerminologyVersionView` field-for-field.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionViewResponse {
    pub id: i64,
    pub kind: TerminologyKind,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::terminology::TerminologyVersionView> for TerminologyVersionViewResponse {
    fn from(view: apis::terminology::TerminologyVersionView) -> Self {
        Self {
            id: view.id,
            kind: view.kind.into(),
            name: view.name,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

/// Wire-level wrapper for `GET /api/terminology/versions`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionListResponse {
    pub versions: Vec<TerminologyVersionViewResponse>,
}

/// Wire-level projection of a `CodeList`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CodeListViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::terminology::CodeListView> for CodeListViewResponse {
    fn from(view: apis::terminology::CodeListView) -> Self {
        Self {
            id: view.id,
            version_id: view.version_id,
            code: view.code,
            extensible: view.extensible,
            name: view.name,
            submission_value: view.submission_value,
            synonym: view.synonym,
            definition: view.definition,
            nci_preferred_term: view.nci_preferred_term,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

/// Paged envelope for `GET /api/terminology/code-lists`. `items`
/// carries the page's rows; `nextOffset = Some(n)` tells the client
/// the next call should pass `offset = n`; `nextOffset = None` means
/// the caller has reached the end of the result set.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagedCodeListListResponse {
    pub items: Vec<CodeListViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

impl From<apis::terminology::Page<apis::terminology::CodeListView>> for PagedCodeListListResponse {
    fn from(page: apis::terminology::Page<apis::terminology::CodeListView>) -> Self {
        Self {
            items: page.items.into_iter().map(Into::into).collect(),
            next_offset: page.next_offset,
        }
    }
}

/// Wire-level projection of a `CodeItem`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemViewResponse {
    pub id: i64,
    pub codelist_id: i64,
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::terminology::CodeItemView> for CodeItemViewResponse {
    fn from(view: apis::terminology::CodeItemView) -> Self {
        Self {
            id: view.id,
            codelist_id: view.codelist_id,
            version_id: view.version_id,
            code: view.code,
            submission_value: view.submission_value,
            synonym: view.synonym,
            definition: view.definition,
            nci_preferred_term: view.nci_preferred_term,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

/// Paged envelope for `GET /api/terminology/code-items`. Mirrors
/// [`PagedCodeListListResponse`] but scopes to a codelist.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagedCodeItemListResponse {
    pub items: Vec<CodeItemViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

impl From<apis::terminology::Page<apis::terminology::CodeItemView>> for PagedCodeItemListResponse {
    fn from(page: apis::terminology::Page<apis::terminology::CodeItemView>) -> Self {
        Self {
            items: page.items.into_iter().map(Into::into).collect(),
            next_offset: page.next_offset,
        }
    }
}

/// Non-paginated list wrapper used by natural-key lookups
/// (`GET /api/terminology/code-items/by-version-and-code`) where
/// pagination makes no sense — every row that matches the natural
/// key is returned in one shot.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemListResponse {
    pub items: Vec<CodeItemViewResponse>,
}

// -- terminology request DTOs -----------------------------------------------

/// Wire-level request body for `POST /api/terminology/versions`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTerminologyVersionRequest {
    pub kind: TerminologyKind,
    pub name: String,
}

/// Wire-level request body for `PATCH /api/terminology/versions/{id}`.
/// Every field is optional. `skip_serializing_if` keeps a
/// partial update round-trip lossless. Deliberately omits `id`: the
/// handler reads it from the `{id}` URL parameter and threads it
/// into `apis::terminology::UpdateTerminologyVersionRequest`.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTerminologyVersionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<TerminologyKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Wire-level request body for `POST /api/terminology/code-lists`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCodeListRequest {
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Wire-level request body for `PATCH /api/terminology/code-lists/{id}`.
///
/// Deliberately omits `id`: the handler reads it from the `{id}`
/// URL parameter and threads it into
/// `apis::terminology::UpdateCodeListRequest`.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCodeListRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synonym: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nci_preferred_term: Option<String>,
}

/// Wire-level request body for `POST /api/terminology/code-items`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCodeItemRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Wire-level request body for `PATCH /api/terminology/code-items/{id}`.
///
/// Deliberately omits `id`: the handler reads it from the `{id}`
/// URL parameter and threads it into
/// `apis::terminology::UpdateCodeItemRequest`.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCodeItemRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submission_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synonym: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nci_preferred_term: Option<String>,
}

/// Wire-level request body for `POST /api/terminology/code-items/batch`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub items: Vec<BatchCodeItemEntry>,
}

/// Wire-level entry inside a batch request.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchCodeItemEntry {
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Wire-level response for `POST /api/terminology/code-items/batch`.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsResponse {
    pub count: usize,
    pub codelist_id: i64,
    pub version_id: i64,
}

// -- terminology list / search query DTOs -----------------------------------

/// Query string for `GET /api/terminology/code-lists`. Unified list
/// + search: `fragment = None` (or empty) yields a plain
///   `ORDER BY id ASC` list; `fragment = Some(_)` runs the FTS
///   prefix-match path with `ts_rank DESC, id ASC` ordering.
///   `offset` / `limit` are clamped by the backend (default 50,
///   max 500).
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeListListQuery {
    pub version_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
}

/// Query string for `GET /api/terminology/code-items`. Mirrors
/// [`CodeListListQuery`] but scoped to a `codelistId`. Both
/// `versionId` and `codelistId` are optional on the wire: omit
/// (or send `null`) to list every code item known to the backend;
/// supply an id to restrict to a single owning version /
/// codelist (the typical per-version or per-codelist browse path).
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemListQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codelist_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
}

/// Query string for `GET /api/terminology/code-items/by-version-and-code`.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemByVersionAndCodeQuery {
    pub version_id: i64,
    pub code: String,
}

// ---- domain_model wire DTOs ----------------------------------------

// The DTOs below mirror the apis::domain_model types. Each request
// / view / query lives in its own struct so the wire format (field
// names, optionality, casing) can evolve independently of the
// backend contract. Handlers translate via `From` / `.into()` so
// they stay one-liners.

/// Wire enum mirroring [`apis::domain_model::DomainCategory`].
/// The apis contract pins the JSON spelling per variant (e.g.
/// `"Special Purpose"` with a space); the round-trip test pins
/// the wire shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum DomainCategory {
    #[serde(rename = "Special Purpose")]
    SpecialPurpose,
    #[serde(rename = "Interventions")]
    Interventions,
    #[serde(rename = "Events")]
    Events,
    #[serde(rename = "Findings")]
    Findings,
    #[serde(rename = "Trial Design")]
    TrialDesign,
    #[serde(rename = "Relationships")]
    Relationships,
    #[serde(rename = "Study Reference")]
    StudyReference,
}

/// Wire enum mirroring [`apis::domain_model::SdtmVariableType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableType {
    Numeric,
    Character,
}

/// Wire enum mirroring [`apis::domain_model::SdtmVariableCore`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableCore {
    Req,
    Exp,
    Perm,
    Supp,
}

/// Wire enum mirroring [`apis::domain_model::SdtmRole`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum SdtmRole {
    Identifier,
    #[serde(rename = "Topic")]
    Topic,
    #[serde(rename = "Timing")]
    Timing,
    #[serde(rename = "Record Qualifier")]
    RecordQualifier,
    #[serde(rename = "Synonym Qualifier")]
    SynonymQualifier,
    #[serde(rename = "Variable Qualifier")]
    VariableQualifier,
    #[serde(rename = "Grouping Qualifier")]
    GroupingQualifier,
    Rule,
}

impl From<apis::domain_model::DomainCategory> for DomainCategory {
    fn from(c: apis::domain_model::DomainCategory) -> Self {
        match c {
            apis::domain_model::DomainCategory::SpecialPurpose => Self::SpecialPurpose,
            apis::domain_model::DomainCategory::Interventions => Self::Interventions,
            apis::domain_model::DomainCategory::Events => Self::Events,
            apis::domain_model::DomainCategory::Findings => Self::Findings,
            apis::domain_model::DomainCategory::TrialDesign => Self::TrialDesign,
            apis::domain_model::DomainCategory::Relationships => Self::Relationships,
            apis::domain_model::DomainCategory::StudyReference => Self::StudyReference,
        }
    }
}

impl From<DomainCategory> for apis::domain_model::DomainCategory {
    fn from(c: DomainCategory) -> Self {
        match c {
            DomainCategory::SpecialPurpose => Self::SpecialPurpose,
            DomainCategory::Interventions => Self::Interventions,
            DomainCategory::Events => Self::Events,
            DomainCategory::Findings => Self::Findings,
            DomainCategory::TrialDesign => Self::TrialDesign,
            DomainCategory::Relationships => Self::Relationships,
            DomainCategory::StudyReference => Self::StudyReference,
        }
    }
}

impl From<apis::domain_model::SdtmVariableType> for SdtmVariableType {
    fn from(t: apis::domain_model::SdtmVariableType) -> Self {
        match t {
            apis::domain_model::SdtmVariableType::Numeric => Self::Numeric,
            apis::domain_model::SdtmVariableType::Character => Self::Character,
        }
    }
}

impl From<SdtmVariableType> for apis::domain_model::SdtmVariableType {
    fn from(t: SdtmVariableType) -> Self {
        match t {
            SdtmVariableType::Numeric => Self::Numeric,
            SdtmVariableType::Character => Self::Character,
        }
    }
}

impl From<apis::domain_model::SdtmVariableCore> for SdtmVariableCore {
    fn from(c: apis::domain_model::SdtmVariableCore) -> Self {
        match c {
            apis::domain_model::SdtmVariableCore::Req => Self::Req,
            apis::domain_model::SdtmVariableCore::Exp => Self::Exp,
            apis::domain_model::SdtmVariableCore::Perm => Self::Perm,
            apis::domain_model::SdtmVariableCore::Supp => Self::Supp,
        }
    }
}

impl From<SdtmVariableCore> for apis::domain_model::SdtmVariableCore {
    fn from(c: SdtmVariableCore) -> Self {
        match c {
            SdtmVariableCore::Req => Self::Req,
            SdtmVariableCore::Exp => Self::Exp,
            SdtmVariableCore::Perm => Self::Perm,
            SdtmVariableCore::Supp => Self::Supp,
        }
    }
}

impl From<apis::domain_model::SdtmRole> for SdtmRole {
    fn from(r: apis::domain_model::SdtmRole) -> Self {
        match r {
            apis::domain_model::SdtmRole::Identifier => Self::Identifier,
            apis::domain_model::SdtmRole::Topic => Self::Topic,
            apis::domain_model::SdtmRole::Timing => Self::Timing,
            apis::domain_model::SdtmRole::RecordQualifier => Self::RecordQualifier,
            apis::domain_model::SdtmRole::SynonymQualifier => Self::SynonymQualifier,
            apis::domain_model::SdtmRole::VariableQualifier => Self::VariableQualifier,
            apis::domain_model::SdtmRole::GroupingQualifier => Self::GroupingQualifier,
            apis::domain_model::SdtmRole::Rule => Self::Rule,
        }
    }
}

impl From<SdtmRole> for apis::domain_model::SdtmRole {
    fn from(r: SdtmRole) -> Self {
        match r {
            SdtmRole::Identifier => Self::Identifier,
            SdtmRole::Topic => Self::Topic,
            SdtmRole::Timing => Self::Timing,
            SdtmRole::RecordQualifier => Self::RecordQualifier,
            SdtmRole::SynonymQualifier => Self::SynonymQualifier,
            SdtmRole::VariableQualifier => Self::VariableQualifier,
            SdtmRole::GroupingQualifier => Self::GroupingQualifier,
            SdtmRole::Rule => Self::Rule,
        }
    }
}

/// Wire projection of an SDTM domain description. Carried on
/// [`SdtmDomainView`] and round-trips through the backend as a
/// single JSONB blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainDescription {
    pub lang: String,
    pub details: SdtmDomainDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainDescriptionDetail {
    pub description: String,
    pub structure: String,
}

/// Wire projection of an SDTM variable description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableDescription {
    pub lang: String,
    pub details: SdtmVariableDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableDescriptionDetail {
    pub label: String,
}

impl From<apis::domain_model::SdtmDomainDescription> for SdtmDomainDescription {
    fn from(d: apis::domain_model::SdtmDomainDescription) -> Self {
        Self {
            lang: d.lang,
            details: SdtmDomainDescriptionDetail {
                description: d.details.description,
                structure: d.details.structure,
            },
        }
    }
}

impl From<SdtmDomainDescription> for apis::domain_model::SdtmDomainDescription {
    fn from(d: SdtmDomainDescription) -> Self {
        Self {
            lang: d.lang,
            details: apis::domain_model::SdtmDomainDescriptionDetail {
                description: d.details.description,
                structure: d.details.structure,
            },
        }
    }
}

impl From<apis::domain_model::SdtmVariableDescription> for SdtmVariableDescription {
    fn from(d: apis::domain_model::SdtmVariableDescription) -> Self {
        Self {
            lang: d.lang,
            details: SdtmVariableDescriptionDetail {
                label: d.details.label,
            },
        }
    }
}

impl From<SdtmVariableDescription> for apis::domain_model::SdtmVariableDescription {
    fn from(d: SdtmVariableDescription) -> Self {
        Self {
            lang: d.lang,
            details: apis::domain_model::SdtmVariableDescriptionDetail {
                label: d.details.label,
            },
        }
    }
}

/// Wire projection of [`apis::domain_model::SdtmVersionView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVersionViewResponse {
    pub id: i64,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::domain_model::SdtmVersionView> for SdtmVersionViewResponse {
    fn from(v: apis::domain_model::SdtmVersionView) -> Self {
        Self {
            id: v.id,
            name: v.name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::domain_model::SdtmDomainView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::domain_model::SdtmDomainView> for SdtmDomainViewResponse {
    fn from(v: apis::domain_model::SdtmDomainView) -> Self {
        Self {
            id: v.id,
            version_id: v.version_id,
            name: v.name,
            category: v.category.into(),
            descriptions: v.descriptions.into_iter().map(Into::into).collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::domain_model::SdtmVariableView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableViewResponse {
    pub id: i64,
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::domain_model::SdtmVariableView> for SdtmVariableViewResponse {
    fn from(v: apis::domain_model::SdtmVariableView) -> Self {
        Self {
            id: v.id,
            domain_id: v.domain_id,
            name: v.name,
            variable_controlled: v.variable_controlled,
            variable_type: v.variable_type.into(),
            variable_core: v.variable_core.into(),
            variable_role: v.variable_role.map(Into::into),
            variable_sequence: v.variable_sequence,
            descriptions: v.descriptions.into_iter().map(Into::into).collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

// ---- list wrappers ----

/// Wire projection of `GET /api/domain-model/versions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVersionListResponse {
    pub versions: Vec<SdtmVersionViewResponse>,
}

impl From<apis::domain_model::SdtmVersionList> for SdtmVersionListResponse {
    fn from(l: apis::domain_model::SdtmVersionList) -> Self {
        Self {
            versions: l.versions.into_iter().map(Into::into).collect(),
        }
    }
}

/// Wire projection of `GET /api/domain-model/versions/{version_id}/domains`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmDomainListResponse {
    pub domains: Vec<SdtmDomainViewResponse>,
}

impl From<apis::domain_model::SdtmDomainList> for SdtmDomainListResponse {
    fn from(l: apis::domain_model::SdtmDomainList) -> Self {
        Self {
            domains: l.domains.into_iter().map(Into::into).collect(),
        }
    }
}

/// Wire projection of `GET /api/domain-model/domains/{domain_id}/variables`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableListResponse {
    pub variables: Vec<SdtmVariableViewResponse>,
}

impl From<apis::domain_model::SdtmVariableList> for SdtmVariableListResponse {
    fn from(l: apis::domain_model::SdtmVariableList) -> Self {
        Self {
            variables: l.variables.into_iter().map(Into::into).collect(),
        }
    }
}

// ---- request bodies ----

/// `POST /api/domain-model/versions` body.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmVersionRequest {
    pub name: String,
}

/// `PUT /api/domain-model/versions/{id}` body. All fields are
/// optional; absent fields are not touched (partial update).
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmVersionRequest {
    pub name: Option<String>,
}

/// `POST /api/domain-model/domains` body.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmDomainRequest {
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

/// `PUT /api/domain-model/domains/{id}` body. All fields optional
/// except via the apis partial-update contract.
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmDomainRequest {
    pub name: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}

/// `POST /api/domain-model/variables` body.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSdtmVariableRequest {
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
}

/// `PUT /api/domain-model/variables/{id}` body. Nullable fields use
/// `Option<Option<T>>` so the caller can distinguish "absent" from
/// "present and null" — see the spec's three-state semantics.
#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSdtmVariableRequest {
    pub name: Option<String>,
    pub variable_controlled: Option<Option<String>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}

// ---- adapters between wire DTOs and apis DTOs ----
//
// Create requests convert via `From` (one-arg, no `id`). Update
// requests are built inline by the handler because the apis DTO
// carries the `id` (read from the URL path), not the wire DTO.

impl From<CreateSdtmVersionRequest> for apis::domain_model::CreateSdtmVersionRequest {
    fn from(r: CreateSdtmVersionRequest) -> Self {
        Self { name: r.name }
    }
}

impl From<CreateSdtmDomainRequest> for apis::domain_model::CreateSdtmDomainRequest {
    fn from(r: CreateSdtmDomainRequest) -> Self {
        Self {
            version_id: r.version_id,
            name: r.name,
            category: r.category.into(),
            descriptions: r.descriptions.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<CreateSdtmVariableRequest> for apis::domain_model::CreateSdtmVariableRequest {
    fn from(r: CreateSdtmVariableRequest) -> Self {
        Self {
            domain_id: r.domain_id,
            name: r.name,
            variable_controlled: r.variable_controlled,
            variable_type: r.variable_type.into(),
            variable_core: r.variable_core.into(),
            variable_role: r.variable_role.map(Into::into),
            variable_sequence: r.variable_sequence,
            descriptions: r.descriptions.into_iter().map(Into::into).collect(),
        }
    }
}

// ===========================================================================
// CRF (Case Report Form) wire DTOs
// ===========================================================================

/// Wire projection of [`apis::crf::CrfItemKind`]. Flat enum — the
/// server side carries the same discriminant set; `From`/`Into`
/// conversions between the two layers are lossless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum CrfItemKind {
    Text,
    Selection,
    Checkbox,
    Datetime,
    Label,
}

impl From<apis::crf::CrfItemKind> for CrfItemKind {
    fn from(k: apis::crf::CrfItemKind) -> Self {
        match k {
            apis::crf::CrfItemKind::Text => Self::Text,
            apis::crf::CrfItemKind::Selection => Self::Selection,
            apis::crf::CrfItemKind::Checkbox => Self::Checkbox,
            apis::crf::CrfItemKind::Datetime => Self::Datetime,
            apis::crf::CrfItemKind::Label => Self::Label,
        }
    }
}

impl From<CrfItemKind> for apis::crf::CrfItemKind {
    fn from(k: CrfItemKind) -> Self {
        match k {
            CrfItemKind::Text => Self::Text,
            CrfItemKind::Selection => Self::Selection,
            CrfItemKind::Checkbox => Self::Checkbox,
            CrfItemKind::Datetime => Self::Datetime,
            CrfItemKind::Label => Self::Label,
        }
    }
}

/// Wire projection of [`apis::crf::AnnotationOwner`]. The DB-side
/// tuple variant `Form(i32)` becomes a struct-shaped variant so the
/// TS client can `switch (owner.kind)` cleanly. Each variant carries
/// the owning row's id under `id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AnnotationOwner {
    Form {
        id: i64,
    },
    Item {
        id: i64,
    },
    #[serde(rename = "option")]
    Option {
        id: i64,
    },
    Unit {
        id: i64,
    },
}

impl From<apis::crf::AnnotationOwner> for AnnotationOwner {
    fn from(o: apis::crf::AnnotationOwner) -> Self {
        match o {
            apis::crf::AnnotationOwner::Form(id) => Self::Form { id },
            apis::crf::AnnotationOwner::Item(id) => Self::Item { id },
            apis::crf::AnnotationOwner::Option(id) => Self::Option { id },
            apis::crf::AnnotationOwner::Unit(id) => Self::Unit { id },
        }
    }
}

impl From<AnnotationOwner> for apis::crf::AnnotationOwner {
    fn from(o: AnnotationOwner) -> Self {
        match o {
            AnnotationOwner::Form { id } => Self::Form(id),
            AnnotationOwner::Item { id } => Self::Item(id),
            AnnotationOwner::Option { id } => Self::Option(id),
            AnnotationOwner::Unit { id } => Self::Unit(id),
        }
    }
}

/// Path parameter for CRF routes. CRF uses `i64` ids to match the
/// underlying Postgres BIGSERIAL/BIGINT columns; kept as its own
/// type to avoid a footgun against the `i64`-based terminology
/// routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CrfPathId {
    pub id: i64,
}

/// Wire-level extractor for the `{project_code}` URL parameter
/// used by `/api/crf/projects/{project_code}/versions` and friends.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProjectPathCode {
    pub project_code: String,
}

// ---- view projections ----

/// Wire projection of [`apis::crf::CrfVersionView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfVersionViewResponse {
    pub id: i64,
    pub project_code: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::CrfVersionView> for CrfVersionViewResponse {
    fn from(v: apis::crf::CrfVersionView) -> Self {
        Self {
            id: v.id,
            project_code: v.project_code,
            name: v.name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::CrfFormView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormViewResponse {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::CrfFormView> for CrfFormViewResponse {
    fn from(v: apis::crf::CrfFormView) -> Self {
        Self {
            id: v.id,
            version_id: v.version_id,
            code: v.code,
            name: v.name,
            order: v.order,
            not_submitted: v.not_submitted,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::CrfItemView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemViewResponse {
    pub id: i64,
    pub form_id: i64,
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::CrfItemView> for CrfItemViewResponse {
    fn from(v: apis::crf::CrfItemView) -> Self {
        Self {
            id: v.id,
            form_id: v.form_id,
            code: v.code,
            name: v.name,
            kind: v.kind.into(),
            order: v.order,
            not_submitted: v.not_submitted,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::CrfOptionView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionViewResponse {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::CrfOptionView> for CrfOptionViewResponse {
    fn from(v: apis::crf::CrfOptionView) -> Self {
        Self {
            id: v.id,
            item_id: v.item_id,
            value: v.value,
            not_submitted: v.not_submitted,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::CrfUnitView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitViewResponse {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::CrfUnitView> for CrfUnitViewResponse {
    fn from(v: apis::crf::CrfUnitView) -> Self {
        Self {
            id: v.id,
            item_id: v.item_id,
            value: v.value,
            not_submitted: v.not_submitted,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::DomainAnnotationView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DomainAnnotationViewResponse {
    pub id: i64,
    pub form_id: i64,
    pub name: String,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::DomainAnnotationView> for DomainAnnotationViewResponse {
    fn from(v: apis::crf::DomainAnnotationView) -> Self {
        Self {
            id: v.id,
            form_id: v.form_id,
            name: v.name,
            description: v.description,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

/// Wire projection of [`apis::crf::AnnotationView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationViewResponse {
    pub id: i64,
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::crf::AnnotationView> for AnnotationViewResponse {
    fn from(v: apis::crf::AnnotationView) -> Self {
        Self {
            id: v.id,
            domain_annotation_id: v.domain_annotation_id,
            content: v.content,
            assign: v.assign,
            owner: v.owner.into(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

// ---- list response wrappers ----

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfVersionListResponse {
    pub versions: Vec<CrfVersionViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormListResponse {
    pub forms: Vec<CrfFormViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemListResponse {
    pub items: Vec<CrfItemViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionListResponse {
    pub options: Vec<CrfOptionViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitListResponse {
    pub units: Vec<CrfUnitViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DomainAnnotationListResponse {
    pub domain_annotations: Vec<DomainAnnotationViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationListResponse {
    pub annotations: Vec<AnnotationViewResponse>,
}

// ---- request DTOs ----

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfVersionRequest {
    pub name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfVersionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfFormRequest {
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfFormRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfItemRequest {
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i32,
    pub not_submitted: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfItemRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<CrfItemKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfOptionRequest {
    pub value: String,
    pub not_submitted: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfOptionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfUnitRequest {
    pub value: String,
    pub not_submitted: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCrfUnitRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_submitted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDomainAnnotationRequest {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDomainAnnotationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAnnotationRequest {
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAnnotationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assign: Option<bool>,
}

// ---- search query DTOs ----
//
// Each search endpoint takes a `versionId` from the URL path (the
// owning version is required for version-scoped substring search)
// and a `fragment` query parameter. The fragment is required — an
// empty / whitespace-only fragment is rejected at the usecase layer
// with `EmptySearchFragment`.

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfFragmentQuery {
    pub fragment: String,
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

    #[test]
    fn register_user_request_roundtrip() {
        let json = r#"{"user_code":"u1","user_name":"Alice","domain_name":"d","hostname":"h","sid":"s","password":"p"}"#;
        let req: RegisterUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.user_code, "u1");
        assert_eq!(req.password, "p");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn register_user_response_from_apis_view() {
        let apis_view = apis::auth::RegisterUserResponse {
            user_code: "u1".into(),
            user_name: "Alice".into(),
            role: apis::user::Role::Admin,
            active: true,
            domain_name: "d".into(),
            hostname: "h".into(),
            sid: "s".into(),
        };
        let resp: RegisterUserResponse = apis_view.into();
        assert_eq!(resp.user_code, "u1");
        assert!(matches!(resp.role, Role::Admin));
        assert!(resp.active);
        assert_eq!(resp.domain_name, "d");
    }

    // ---- project DTO round-trips -----

    fn sample_project_view() -> apis::project::ProjectView {
        apis::project::ProjectView {
            id: 2,
            code: "project-1".into(),
            description: "alpha".into(),
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
            tags: vec![
                apis::project::TagView {
                    key: "Product".into(),
                    value: "DEMO-001".into(),
                },
                apis::project::TagView {
                    key: "Region".into(),
                    value: "EU".into(),
                },
            ],
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
    fn create_project_request_minimal_roundtrip() {
        let json = r#"{"code":"pr1","description":"x"}"#;
        let req: CreateProjectRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "pr1");
        assert_eq!(req.description, "x");
        assert!(req.members.is_none());
        assert!(req.unblind_members.is_none());
        assert!(req.tags.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn create_project_request_with_empty_members_roundtrip() {
        let json = r#"{"code":"pr1","description":"x","members":{},"unblindMembers":{}}"#;
        let req: CreateProjectRequest = serde_json::from_str(json).unwrap();
        let members = req.members.as_ref().expect("members present");
        assert!(members.leaders.is_empty());
        assert!(members.workers.is_empty());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn create_project_request_with_tags_roundtrip() {
        let json =
            r#"{"code":"pr1","description":"x","tags":[{"key":"Product","value":"DEMO-001"}]}"#;
        let req: CreateProjectRequest = serde_json::from_str(json).unwrap();
        let tags = req.tags.as_ref().expect("tags present");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].key, "Product");
        assert_eq!(tags[0].value, "DEMO-001");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_project_request_omitted_membership_keeps_none() {
        let json = r#"{"description":"new"}"#;
        let req: UpdateProjectRequest = serde_json::from_str(json).unwrap();
        assert!(req.members.is_none());
        assert!(req.unblind_members.is_none());
        assert!(req.tags.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_project_request_empty_membership_becomes_some_empty() {
        let json = r#"{"members":{},"unblindMembers":{}}"#;
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
    fn update_project_request_empty_tags_becomes_some_empty() {
        let json = r#"{"tags":[]}"#;
        let req: UpdateProjectRequest = serde_json::from_str(json).unwrap();
        let tags = req.tags.as_ref().expect("tags present");
        assert!(tags.is_empty());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn tag_view_response_from_apis_view() {
        let view = apis::project::TagView {
            key: "Product".into(),
            value: "DEMO-001".into(),
        };
        let resp: TagViewResponse = view.into();
        assert_eq!(resp.key, "Product");
        assert_eq!(resp.value, "DEMO-001");
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
        assert_eq!(resp.members.leaders.len(), 1);
        assert_eq!(resp.members.leaders[0].code, "leader-1");
        assert!(resp.members.workers.is_empty());
        assert!(resp.unblind_members.leaders.is_empty());
        assert_eq!(resp.unblind_members.workers.len(), 1);
        assert_eq!(resp.unblind_members.workers[0].code, "worker-2");
        assert_eq!(resp.tags.len(), 2);
        assert_eq!(resp.tags[0].key, "Product");
        assert_eq!(resp.tags[1].value, "EU");
    }

    #[test]
    fn project_list_response_roundtrip() {
        let json = r#"{"projects":[]}"#;
        let resp: ProjectListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.projects.is_empty());
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    // ---- terminology DTO round-trips -----

    fn sample_terminology_version_view() -> apis::terminology::TerminologyVersionView {
        apis::terminology::TerminologyVersionView {
            id: 1,
            kind: apis::terminology::TerminologyKind::Sdtm,
            name: "2026-03-27".into(),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    fn sample_code_list_view() -> apis::terminology::CodeListView {
        apis::terminology::CodeListView {
            id: 11,
            version_id: 1,
            code: "C66741".into(),
            extensible: true,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: String::new(),
            definition: String::new(),
            nci_preferred_term: "Age".into(),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    fn sample_code_item_view() -> apis::terminology::CodeItemView {
        apis::terminology::CodeItemView {
            id: 100,
            codelist_id: 11,
            version_id: 1,
            code: "C1".into(),
            submission_value: "Y".into(),
            synonym: String::new(),
            definition: String::new(),
            nci_preferred_term: "Yes".into(),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    #[test]
    fn terminology_kind_round_trip_all_variants() {
        for k in [TerminologyKind::Sdtm, TerminologyKind::Adam] {
            let s = serde_json::to_string(&k).unwrap();
            let back: TerminologyKind = serde_json::from_str(&s).unwrap();
            assert_eq!(format!("{k:?}"), format!("{back:?}"));
        }
    }

    #[test]
    fn terminology_kind_from_apis_all_variants() {
        assert!(matches!(
            TerminologyKind::from(apis::terminology::TerminologyKind::Sdtm),
            TerminologyKind::Sdtm
        ));
        assert!(matches!(
            TerminologyKind::from(apis::terminology::TerminologyKind::Adam),
            TerminologyKind::Adam
        ));
    }

    #[test]
    fn path_id_roundtrip() {
        let json = r#"{"id":42}"#;
        let p: PathId = serde_json::from_str(json).unwrap();
        assert_eq!(p.id, 42);
        assert_eq!(serde_json::to_string(&p).unwrap(), json);
    }

    #[test]
    fn terminology_version_view_response_from_apis_view() {
        let resp: TerminologyVersionViewResponse = sample_terminology_version_view().into();
        assert_eq!(resp.id, 1);
        assert!(matches!(resp.kind, TerminologyKind::Sdtm));
        assert_eq!(resp.name, "2026-03-27");
    }

    #[test]
    fn terminology_version_list_response_roundtrip() {
        let json = r#"{"versions":[]}"#;
        let resp: TerminologyVersionListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.versions.is_empty());
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    #[test]
    fn code_list_view_response_from_apis_view() {
        let resp: CodeListViewResponse = sample_code_list_view().into();
        assert_eq!(resp.id, 11);
        assert_eq!(resp.version_id, 1);
        assert_eq!(resp.code, "C66741");
        assert!(resp.extensible);
        assert_eq!(resp.name, "AGE");
    }

    #[test]
    fn paged_code_list_list_response_roundtrip() {
        let json = r#"{"items":[]}"#;
        let resp: PagedCodeListListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.items.is_empty());
        assert_eq!(resp.next_offset, None);
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    #[test]
    fn paged_code_list_list_response_with_next_offset_roundtrip() {
        let json = r#"{"items":[],"nextOffset":50}"#;
        let resp: PagedCodeListListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.next_offset, Some(50));
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    #[test]
    fn paged_code_list_list_response_from_apis_page() {
        let page = apis::terminology::Page {
            items: vec![sample_code_list_view()],
            next_offset: Some(2),
        };
        let resp: PagedCodeListListResponse = page.into();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].code, "C66741");
        assert_eq!(resp.next_offset, Some(2));
    }

    #[test]
    fn code_item_view_response_from_apis_view() {
        let resp: CodeItemViewResponse = sample_code_item_view().into();
        assert_eq!(resp.id, 100);
        assert_eq!(resp.codelist_id, 11);
        assert_eq!(resp.version_id, 1);
        assert_eq!(resp.code, "C1");
        assert_eq!(resp.nci_preferred_term, "Yes");
    }

    #[test]
    fn paged_code_item_list_response_roundtrip() {
        let json = r#"{"items":[]}"#;
        let resp: PagedCodeItemListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.items.is_empty());
        assert_eq!(resp.next_offset, None);
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    #[test]
    fn paged_code_item_list_response_from_apis_page() {
        let page = apis::terminology::Page {
            items: vec![sample_code_item_view()],
            next_offset: None,
        };
        let resp: PagedCodeItemListResponse = page.into();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].code, "C1");
        assert_eq!(resp.next_offset, None);
    }

    #[test]
    fn create_terminology_version_request_roundtrip() {
        let json = r#"{"kind":"sdtm","name":"2026-03-27"}"#;
        let req: CreateTerminologyVersionRequest = serde_json::from_str(json).unwrap();
        assert!(matches!(req.kind, TerminologyKind::Sdtm));
        assert_eq!(req.name, "2026-03-27");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_terminology_version_request_partial_roundtrip() {
        let json = r#"{"name":"new"}"#;
        let req: UpdateTerminologyVersionRequest = serde_json::from_str(json).unwrap();
        assert!(req.kind.is_none());
        assert_eq!(req.name.as_deref(), Some("new"));
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn create_code_list_request_roundtrip() {
        let json = r#"{"versionId":1,"code":"C66741","extensible":true,"name":"AGE","submissionValue":"AGE","synonym":"","definition":"","nciPreferredTerm":"Age"}"#;
        let req: CreateCodeListRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.version_id, 1);
        assert_eq!(req.code, "C66741");
        assert!(req.extensible);
        assert_eq!(req.nci_preferred_term, "Age");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_code_list_request_partial_roundtrip() {
        let json = r#"{"name":"new"}"#;
        let req: UpdateCodeListRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.name.as_deref(), Some("new"));
        assert!(req.code.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn create_code_item_request_roundtrip() {
        let json = r#"{"codelistId":11,"versionId":1,"code":"C1","submissionValue":"Y","synonym":"","definition":"","nciPreferredTerm":"Yes"}"#;
        let req: CreateCodeItemRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.codelist_id, 11);
        assert_eq!(req.version_id, 1);
        assert_eq!(req.code, "C1");
        assert_eq!(req.nci_preferred_term, "Yes");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_code_item_request_partial_roundtrip() {
        let json = r#"{"code":"C2"}"#;
        let req: UpdateCodeItemRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code.as_deref(), Some("C2"));
        assert!(req.synonym.is_none());
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn code_list_list_query_with_fragment_roundtrip() {
        let json = r#"{"versionId":1,"fragment":"age","offset":0,"limit":50}"#;
        let q: CodeListListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.version_id, 1);
        assert_eq!(q.fragment.as_deref(), Some("age"));
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 50);
        assert_eq!(serde_json::to_string(&q).unwrap(), json);
    }

    #[test]
    fn code_list_list_query_minimal_roundtrip() {
        let json = r#"{"versionId":1}"#;
        let q: CodeListListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.version_id, 1);
        assert!(q.fragment.is_none());
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 0);
        // Optional fragment is omitted from the serialized form so
        // `?versionId=1` stays minimal; numeric `offset`/`limit`
        // serialize as `0` rather than being skipped.
        assert_eq!(
            serde_json::to_string(&q).unwrap(),
            r#"{"versionId":1,"offset":0,"limit":0}"#
        );
    }

    #[test]
    fn code_item_list_query_with_fragment_roundtrip() {
        let json = r#"{"codelistId":11,"fragment":"yes","offset":50,"limit":25}"#;
        let q: CodeItemListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.codelist_id, Some(11));
        assert_eq!(q.fragment.as_deref(), Some("yes"));
        assert_eq!(q.offset, 50);
        assert_eq!(q.limit, 25);
        assert_eq!(serde_json::to_string(&q).unwrap(), json);
    }

    #[test]
    fn code_item_list_query_without_codelist_id_roundtrip() {
        // Omitting `codelistId` (or sending `null`) must default to
        // `None` and serialize back without the key, preserving
        // the optional semantics on the wire.
        let json = r#"{"fragment":"x","offset":0,"limit":50}"#;
        let q: CodeItemListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.codelist_id, None);
        assert_eq!(q.fragment.as_deref(), Some("x"));
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 50);
        assert_eq!(serde_json::to_string(&q).unwrap(), json);

        // Explicit `null` is also accepted.
        let q_null: CodeItemListQuery =
            serde_json::from_str(r#"{"codelistId":null,"offset":0,"limit":50}"#).unwrap();
        assert_eq!(q_null.codelist_id, None);
    }

    #[test]
    fn code_item_by_version_and_code_query_roundtrip() {
        let json = r#"{"versionId":1,"code":"C1"}"#;
        let q: CodeItemByVersionAndCodeQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.version_id, 1);
        assert_eq!(q.code, "C1");
        assert_eq!(serde_json::to_string(&q).unwrap(), json);
    }

    #[test]
    fn batch_create_code_items_request_roundtrip() {
        let json = r#"{"codelistId":11,"versionId":1,"items":[{"code":"C1","submissionValue":"Y","synonym":"","definition":"","nciPreferredTerm":"Yes"}]}"#;
        let req: BatchCreateCodeItemsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.codelist_id, 11);
        assert_eq!(req.version_id, 1);
        assert_eq!(req.items.len(), 1);
        assert_eq!(req.items[0].code, "C1");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn batch_create_code_items_response_roundtrip() {
        let json = r#"{"count":3,"codelistId":11,"versionId":1}"#;
        let resp: BatchCreateCodeItemsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.count, 3);
        assert_eq!(resp.codelist_id, 11);
        assert_eq!(resp.version_id, 1);
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }
}
