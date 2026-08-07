//! Public-API compile test for the `auth` crate.
//!
//! Does NOT run any I/O. Locks the documented trait surface and the
//! in-crate type names so a regression in any layer is caught at
//! `cargo test -p auth` time.

use std::sync::Arc;
use std::time::Duration;

use apis::auth::AuthService;
use auth::{
    AccessTokenView, AuthClaimsView, AuthServiceImpl, AuthUsecase, AuthUsecaseConfig,
    DomainIdentityRepo, DomainIdentityRepository, InMemoryTokenVersionCache, LogoutAck, Role,
    TokenPairView, TokenVersionCache, UserCredentialsRepo, UserCredentialsRepository, UserService,
};

#[test]
fn public_types_are_nameable_from_crate_root() {
    fn assert_role(_: Role) {}
    fn assert_pair(_: TokenPairView) {}
    fn assert_claims(_: AuthClaimsView) {}
    fn assert_token(_: AccessTokenView) {}
    fn assert_ack(_: LogoutAck) {}

    assert_role(Role::Admin);
    assert_pair(TokenPairView {
        access_token: "a".into(),
        refresh_token: "r".into(),
    });
    assert_claims(AuthClaimsView {
        code: "u1".into(),
        role: Role::Admin,
        token_version: 1,
    });
    assert_token(AccessTokenView {
        access_token: "a".into(),
    });
    assert_ack(LogoutAck {});
}

#[test]
fn repo_constructors_accept_a_pg_pool() {
    let ctor: fn(sqlx::PgPool) -> UserCredentialsRepo = UserCredentialsRepo::new;
    let ctor2: fn(sqlx::PgPool) -> DomainIdentityRepo = DomainIdentityRepo::new;
    let _ = (ctor, ctor2);
}

#[test]
fn auth_usecase_config_has_expected_field_shape() {
    // Locks the public surface of `AuthUsecaseConfig` without running it.
    // The closure body never runs — its only job is to type-check every
    // field name against the actual struct definition.
    let _assert_config_shape: fn(cfg: AuthUsecaseConfig<UserCredentialsRepo, DomainIdentityRepo>) =
        |cfg| {
            let _: &UserCredentialsRepo = &cfg.credentials;
            let _: &DomainIdentityRepo = &cfg.identities;
            let _: &Arc<dyn UserService> = &cfg.user_service;
            let _: &Arc<dyn TokenVersionCache> = &cfg.cache;
            let _: &[u8] = &cfg.signing_key;
            let _: Duration = cfg.access_ttl;
            let _: Duration = cfg.refresh_ttl;
        };
}

#[test]
fn auth_usecase_new_accepts_an_auth_usecase_config() {
    fn assert_user_service_is_send_sync<T: Send + Sync>() {}
    assert_user_service_is_send_sync::<Box<dyn UserService>>();
    assert_user_service_is_send_sync::<Box<dyn TokenVersionCache>>();

    fn assert_repo_bounds<R: UserCredentialsRepository, D: DomainIdentityRepository>() {}
    assert_repo_bounds::<UserCredentialsRepo, DomainIdentityRepo>();
    let _ = AuthUsecase::<UserCredentialsRepo, DomainIdentityRepo>::new;
}

#[test]
fn in_memory_token_version_cache_is_nameable() {
    let _: InMemoryTokenVersionCache = InMemoryTokenVersionCache::new();
    fn assert_cache<T: TokenVersionCache>() {}
    assert_cache::<InMemoryTokenVersionCache>();
}

#[test]
fn auth_service_impl_is_object_safe() {
    // Pin the trait surface through a function pointer so object-safety
    // is checked at compile time without ever constructing an instance.
    let _: fn(AuthServiceImpl<UserCredentialsRepo, DomainIdentityRepo>) -> Box<dyn AuthService> =
        |s| Box::new(s);
}
