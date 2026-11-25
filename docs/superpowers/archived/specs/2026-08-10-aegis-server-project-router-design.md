# Aegis Server Project Router Design

**Date:** 2026-08-10
**Status:** Approved design

## Goal

Expose the product and project lifecycle operations from `apis::project::ProjectService` through authenticated HTTP routes in `apps/server/aegis-server` while following the server's existing Axum, utoipa, dependency-injection, error-response, and testing conventions.

The HTTP API exposes code-based resource lookup only. Numeric-ID lookup methods remain internal service operations.

## Scope

The change includes:

- Eight product and project HTTP endpoints.
- Bearer authentication on every endpoint.
- Role-based authorization for create and update operations.
- HTTP-specific request and response DTOs.
- `ProjectApiError` to HTTP error mapping.
- Project service construction and `AppState` wiring.
- OpenAPI schemas, tags, paths, security declarations, and responses.
- Unit and full-router tests.
- Server README route documentation.

The change does not include:

- Public numeric-ID lookup routes.
- Delete endpoints, because `ProjectService` does not define deletion.
- Pagination or filtering.
- Server-wide authorization middleware or a general RBAC framework.
- New live-database integration tests; PostgreSQL behavior remains covered by the project crate.

## HTTP Surface

| Method | Path | Service operation | Required role |
|---|---|---|---|
| `POST` | `/api/product` | `create_product` | `Root` or `Admin` |
| `GET` | `/api/product` | `list_products` | Any authenticated role |
| `GET` | `/api/product/{code}` | `get_product_by_code` | Any authenticated role |
| `PATCH` | `/api/product/{code}` | Resolve code, then `update_product` | `Root` or `Admin` |
| `POST` | `/api/project` | `create_project` | `Root` or `Admin` |
| `GET` | `/api/project` | `list_projects` | Any authenticated role |
| `GET` | `/api/project/{code}` | `get_project_by_code` | Any authenticated role |
| `PATCH` | `/api/project/{code}` | Resolve code, then `update_project` | `Root` or `Admin` |

All routes require a valid bearer access token through the existing `AuthClaims` extractor. Missing or invalid authentication returns `401 Unauthorized` through the existing authentication error path.

An authenticated caller without the `Root` or `Admin` role receives:

- HTTP status `403 Forbidden`.
- Error code `forbidden`.
- Message `admin or root role required`.

## HTTP Module Structure

Product and project operations share one flat feature module:

```text
transport/http/project/
├── handlers.rs
└── router.rs
```

`handlers.rs` contains all eight handlers and a small shared authorization helper. `router.rs` registers the product and project paths with `OpenApiRouter<AppState>`. Each handler is registered in a separate `routes!(...)` call, matching the server's existing utoipa-axum composition convention.

The top-level HTTP router mounts both resource prefixes from this module. Product and project receive separate OpenAPI tags even though their handlers share a module and service.

## Dependency Injection

`AppState` gains:

```rust
pub project: Arc<dyn apis::project::ProjectService>
```

Server startup constructs the service from the existing pool and user service:

1. Construct `project::ProductRepo` from a clone of the PostgreSQL pool.
2. Construct `project::ProjectRepo` from a clone of the pool.
3. Construct the project crate's user-service adapter around the existing `Arc<dyn apis::user::UserService>`.
4. Construct `ProjectUsecase` with `ProjectUsecaseConfig`.
5. Construct `ProjectServiceImpl` around the use case.
6. Store it as `Arc<dyn ProjectService>` in `AppState`.

The server crate adds a dependency on `lib/crates/project`. Existing auth and user service construction remains unchanged apart from sharing the user service with the project adapter.

## Authorization

Every handler extracts `AuthClaims`.

Read handlers require no role check beyond successful authentication. Create and update handlers call a local helper that accepts only `apis::user::Role::Root` and `apis::user::Role::Admin` (using the actual role type carried by the claims). Other roles return the server's `403 Forbidden` API error.

The helper is local to the project HTTP feature. This avoids introducing a general authorization abstraction before another feature needs it while ensuring all four write handlers use identical behavior.

## Wire DTOs

The server owns HTTP-specific DTOs. The `apis` crate remains independent of Serde and utoipa concerns.

New wire types cover:

- `CreateProductRequest`
- `UpdateProductRequest`
- `ProductViewResponse`
- `ProductListResponse`
- `CreateProjectRequest`
- `UpdateProjectRequest`
- `ProjectMemberDataRequest`
- `ProjectViewResponse`
- `ProjectMemberViewResponse`
- `UserSummaryViewResponse`
- `ProjectListResponse`

The existing `PathCode` DTO is reused for `{code}` extraction.

Response conversion uses `From<apis::project::...>` implementations. Request conversion either uses `From` implementations or direct construction where the URL supplies the target ID.

List responses are wrapped for consistency and future extensibility:

```json
{ "products": [] }
```

```json
{ "projects": [] }
```

Optional update fields use `skip_serializing_if = "Option::is_none"`, matching the existing partial-update DTO behavior.

### Membership semantics

`members` and `unblind_members` are optional request fields.

For project creation:

- Missing team data creates no membership rows for that team.
- A present but empty team also creates no membership rows.

For project updates:

- Missing team data leaves that team unchanged.
- A present but empty team removes all members from that team.
- A present non-empty team replaces that team's membership with the supplied user codes.

The wire DTO must preserve the distinction between a missing field and a present empty object during update deserialization.

## Handler Data Flow

Create and list handlers convert the incoming DTO, call the corresponding `ProjectService` method, and convert the returned view into the wire response.

Get handlers pass the path code to `get_product_by_code` or `get_project_by_code`.

Update handlers follow the existing user update pattern:

1. Extract and authorize the caller.
2. Resolve the path code through `get_*_by_code`.
3. Place the returned numeric ID into the service update request.
4. Forward optional body fields unchanged.
5. Call `update_product` or `update_project`.
6. Convert the returned view into the response DTO.

This keeps numeric IDs out of the public route and body while satisfying the service contract.

## Error Mapping

`ApiError` gains a `Project(#[from] ProjectApiError)` variant and a local authorization failure representation if an existing suitable server error does not already exist.

Project service errors map as follows:

| `ProjectApiError` | HTTP status | Stable error code |
|---|---:|---|
| `Validation(_)` | `400 Bad Request` | `validation_failed` |
| `NotFound` | `404 Not Found` | `not_found` |
| `ProductNotFound(_)` | `404 Not Found` | `product_not_found` |
| `UserNotFound(_)` | `404 Not Found` | `user_not_found` |
| `DuplicateCode(_)` | `409 Conflict` | `duplicate_code` |
| `Repository(_)` | `500 Internal Server Error` | `repository_error` |

Authenticated callers lacking write permission receive `403 Forbidden` with code `forbidden`.

The existing `ApiError::into_response` path remains responsible for the common JSON error shape and logging server errors. Project repository details are not exposed beyond the existing error message policy.

## OpenAPI

The OpenAPI document gains:

- `product` and `project` tags.
- Schemas for every new request and response DTO.
- All eight routes through automatic `OpenApiRouter` collection.
- `BearerAuth` security requirements on every route.
- Success and applicable error responses, including `401` on all routes and `403` on write routes.

No manual `paths(...)` list is added to `ApiDoc`; paths continue to be collected by utoipa-axum route registration.

## Testing Strategy

Implementation follows test-driven development and the server's existing mock-service patterns.

### DTO tests

Tests verify:

- Request conversion to `apis::project` request types.
- Product and project view conversion to response DTOs.
- Nested product, membership, and user-summary conversion.
- Omitted update fields remain `None`.
- Present empty membership objects remain distinguishable from omitted membership fields.
- List response wrappers use `products` and `projects` keys.

### Handler tests

Tests cover:

- Successful create, list, get-by-code, and update behavior for both resources.
- URL code resolution before update.
- Request and response payload mapping.
- Missing or invalid bearer credentials returning `401`.
- Every write endpoint returning `403` for an authenticated non-`Root`/non-`Admin` role.
- Successful writes for both `Root` and `Admin`.
- Validation, not-found, referenced-product, referenced-user, duplicate-code, and repository error responses.

### Router and OpenAPI tests

Full-router tests verify:

- Both route prefixes are mounted under `/api`.
- Expected methods are available and unsupported methods are rejected.
- Authentication applies to all eight endpoints.
- OpenAPI includes all product and project paths, schemas, tags, security requirements, and write-route `403` responses.

### Verification

The completion gate includes:

- New targeted tests.
- The complete `aegis-server` test suite.
- Relevant project and API crate tests.
- Workspace formatting and compilation checks.
- README route-table verification.

## Documentation

Update `apps/server/aegis-server/README.md` with the eight routes, bearer-authentication requirement, and `Root`/`Admin` write restriction.
