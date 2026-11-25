# Auth User Registration Design

**Date:** 2026-08-10

## Goal

Add administrator-controlled user registration to `lib/crates/auth`, including an allowlisted domain policy, and expose it through the auth API and server's user-credential router.

## Usecase configuration and registration flow

`AuthUsecaseConfig` gains `allow_domains: Vec<String>`. `AuthUsecase::new` normalizes every configured domain by trimming whitespace and lowercasing it, then stores the normalized values. Registration normalizes the incoming `domain_name` identically. A domain is accepted only when it matches an allowlist entry; an empty allowlist rejects every registration.

Add a `RegisterUser` command containing:

- `user_code`
- `user_name`
- `domain_name`
- `hostname`
- `sid`
- `password`

`AuthUsecase::register_user` will:

1. Validate required fields and check the normalized domain allowlist before writes.
2. Look up the user through the auth-domain `UserService`; when absent, create it with the supplied code/name, `Role::General`, and `active = false`.
3. Look up credentials; when absent, hash the raw password with the existing Argon2 helper and create credentials with the initial token version.
4. Look up the exact domain identity; when absent, create it.
5. Return a registration view containing safe user and identity fields, never the raw password or password hash.

Existing records are reused. Unexpected repository errors propagate. Concurrent duplicate-create races are handled by re-reading the existing record where practical; no existing record is overwritten.

## Port and adapter changes

Extend the auth-domain `UserService` port with a create operation. `UserServiceImpl` adapts this to the existing `apis::user::UserService::create`, forcing the general role and inactive state through the request/backend create path.

Extend `DomainIdentityRepository` with `create(DomainIdentity)`. Add the PostgreSQL insert implementation and preserve existing unique-violation/error mapping.

The feature does not introduce a cross-repository transaction because the current ports are independent. Transactional all-or-nothing behavior is outside this scope.

## API contract

Add to `apis::auth`:

- `RegisterUserRequest` with the six registration fields.
- `RegisterUserResponse` with `user_code`, `user_name`, `role`, `active`, `domain_name`, `hostname`, and `sid`.

The response never exposes a password or password hash.

`AuthService` gains `register_user`, and `AuthServiceImpl` translates the API request into the usecase command and maps the result/errors back to API types. Add a dedicated `DomainNotAllowed` domain error mapped to API validation failure.

## HTTP contract

Add `POST /api/auth/user-credential` alongside the existing `PATCH` route.

- Request: JSON registration body.
- Authentication: existing bearer `AuthClaims` extractor.
- Authorization: only `Role::Root` and `Role::Admin`; `Role::General` receives `403 Forbidden`.
- Success: `201 Created` with the registration response.
- Existing patch behavior remains unchanged.

Update wire DTOs and OpenAPI annotations accordingly.

## Error and security behavior

- Disallowed domains fail before any repository writes.
- Empty required fields use existing validation errors.
- Missing records trigger creation; other repository failures propagate.
- Passwords are hashed with Argon2 before persistence.
- Passwords, hashes, and signing secrets are not returned or logged.

## Tests

Add usecase tests for allowlist normalization, empty/disallowed lists, write ordering, missing-record creation, reuse of existing records, Argon2 hashing, general/inactive user creation, and error propagation.

Add facade tests for request/response conversion and error mapping. Add HTTP tests for `201` success, root/admin authorization, general-user `403`, validation failures, and repository failures.
