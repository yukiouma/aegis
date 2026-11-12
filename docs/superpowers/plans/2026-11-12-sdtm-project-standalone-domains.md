# SDTM Project — Standalone Domains & Variables Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the sdtm-project crate's override model with self-contained project domains and variables, and add `conversion_rules` to a project variable.

**Architecture:** `sdtm_project_domains` and `sdtm_project_variables` each carry the full metadata of their row — the two `*_overrides` tables, their repository ports, their `apis` methods and their HTTP endpoints are deleted. A project variable gains `conversion_rules: Vec<ConversionRule>`, where each rule names a `VariableSource` (protocol / derived / assigned / CRF / eDiT) and an opaque `rule` expression. A domain's `key_variables` must always name variables that exist in that domain; a rename or delete of a referenced variable is rejected rather than silently rewriting the study's key.

**Tech Stack:** Rust 2024, Cargo workspace, SQLx (runtime API, no offline cache), PostgreSQL, axum + utoipa (server), serde/serde_json.

**Spec:** [`docs/superpowers/specs/2026-11-12-sdtm-project-standalone-domains-design.md`](../specs/2026-11-12-sdtm-project-standalone-domains-design.md) — the plan argues from the spec; read both.

## Global Constraints

- `aegis-desktop` is **not** touched. No TS files, no Rust shell files.
- The `sdtm-project` crate keeps its **copy-not-import** convention: `DomainCategory`, `SdtmVariableType`, `SdtmVariableCore`, `SdtmRole`, `SdtmVariableControlled` and the description DTOs are duplicated into the crate, never imported from `domain-model`. The same applies to `apis`, which duplicates the crate's value types.
- The crate's Postgres DDL is **rewritten in place**. There is no `ALTER TABLE` migration anywhere in this plan, and the two `*_overrides` migration files are deleted outright.
- Wire DTOs are hand-written in both halves (`apis/src/sdtm_project.rs` and `apps/server/aegis-server/src/transport/http/dto.rs`); both change in the same commit.
- The `apis` layer is snake_case. The HTTP layer is camelCase (`#[serde(rename_all = "camelCase")]`).
- `VariableSource` serializes adjacently tagged: `#[serde(tag = "source", content = "from")]`, with per-variant renames `Protocol`, `Derived`, `Assigned`, `CRF`, `eDT`. There is no blanket `rename_all` — no single casing rule yields both `CRF` and `eDT`.
- Error mapping: `KeyVariableNotFound` → HTTP 400, code `key_variable_not_found`; `VariableReferencedByKeyVariables` → HTTP 409, code `variable_referenced_by_key_variables`.
- Every commit must leave the three touched crates building. Tasks 1–4 are sequenced so this holds at each task boundary; Task 3 is internally atomic (see its header).
- **Verification runs against `-p apis -p sdtm-project -p aegis-server`, not `--workspace`.** `windows-utils` is a workspace member with a `compile_error!` on non-Windows targets, so `cargo build --workspace` fails on macOS before any of this work. That failure is pre-existing.
- `aegis-server`'s lib tests have **28 pre-existing failures** on a clean tree (auth handlers, dto round-trips, `openapi_json_returns_200_with_valid_doc`). They are unrelated to this work and are not caused by it — a run that shows exactly those 28 failing is green for this plan's purposes.

## Review Focus

Five input classes the spec implies that a naive implementation gets wrong. Each has a test in the task that owns the code.

1. **A `key_variables` code that names a variable in a *different* domain.** The existence check is scoped to the domain being written, not to the project. A code that exists elsewhere in the same version must still be rejected. → Task 3, `update_domain_rejects_key_variable_from_another_domain`.
2. **`update_domain` with `key_variables: []`.** An explicit empty list must *clear* the column, not be read as "leave unchanged" (the `Option<Vec<String>>` "unchanged" state is `None`, not `Some(vec![])`). → Task 3, `update_domain_with_empty_key_variables_clears_the_list`.
3. **A conversion rule whose source is `Crf(None)` / `Assigned(None)`.** The JSONB round-trip must preserve `"from": null` for a payload variant and must **omit** the `from` key entirely for a unit variant (`Protocol`). A blanket `Option<FromCrf>` on a flat struct gets one of the two wrong. → Task 3, `conversion_rules_round_trip_through_the_repository`, and Task 1, `variable_source_serializes_to_the_agreed_json`.
4. **A variable with `variable_controlled: None`.** The JSONB column must be SQL `NULL`, not the JSON literal `null` — otherwise `Option<SdtmVariableControlled>` reads back as `Some` and the "unset" state is lost. → Task 3, `variable_without_controlled_terminology_round_trips_as_null`.
5. **A variable whose `variable_code` is renamed while it is a key variable.** The rename must be rejected with `VariableReferencedByKeyVariables` *before* the write, and the variable must still carry its old code afterwards. → Task 3, `update_variable_rejects_rename_when_the_old_code_is_a_key_variable`.

---

### Task 1: `apis` value types and error variants

The new value types and the two new error variants are additive: nothing existing changes shape, so this task compiles and ships on its own. The server's error mapping must learn the new variants in the same task, because `sdtm_project_status` / `sdtm_project_code` match exhaustively.

**Files:**
- Modify: `lib/crates/apis/src/sdtm_project.rs`
- Modify: `lib/crates/apis/tests/public_api.rs`
- Modify: `apps/server/aegis-server/src/transport/http/error.rs`

**Interfaces:**
- Consumes: nothing (first task).
- Produces:
  - `apis::sdtm_project::VariableSource` — `enum { Protocol, Derived, Assigned(Option<FromCrf>), Crf(Option<FromCrf>), Edt }`, `Serialize + Deserialize + Clone + PartialEq + Eq + Debug`, adjacently tagged with `source` / `from`.
  - `apis::sdtm_project::FromCrf` — `struct { pub form_code: String }`.
  - `apis::sdtm_project::ConversionRule` — `struct { pub source: VariableSource, pub rule: String }`.
  - `apis::sdtm_project::SdtmVariableControlled` — `struct { pub code: Option<Vec<String>>, pub format: Option<Vec<String>> }`, both members `skip_serializing_if = "Option::is_none"`.
  - `apis::sdtm_project::SdtmProjectApiError::KeyVariableNotFound { variable_code: String }`
  - `apis::sdtm_project::SdtmProjectApiError::VariableReferencedByKeyVariables { variable_code: String }`

- [ ] **Step 1: Write the failing tests in `lib/crates/apis/tests/public_api.rs`**

Add to the `apis::sdtm_project` import list: `ConversionRule`, `FromCrf`, `SdtmVariableControlled`, `VariableSource`.

Add these two tests:

```rust
/// The new value types are nameable and constructible from the test.
#[test]
fn sdtm_project_value_types_are_nameable() {
    fn assert_source(_: VariableSource) {}
    fn assert_from_crf(_: FromCrf) {}
    fn assert_rule(_: ConversionRule) {}
    fn assert_controlled(_: SdtmVariableControlled) {}

    assert_source(VariableSource::Protocol);
    assert_source(VariableSource::Derived);
    assert_source(VariableSource::Edt);
    assert_source(VariableSource::Assigned(None));
    assert_source(VariableSource::Crf(Some(FromCrf {
        form_code: "AE".into(),
    })));
    assert_from_crf(FromCrf {
        form_code: "AE".into(),
    });
    assert_rule(ConversionRule {
        source: VariableSource::Protocol,
        rule: "AESEV = AESEV".into(),
    });
    assert_controlled(SdtmVariableControlled {
        code: Some(vec!["C66731".into()]),
        format: None,
    });
}

/// The wire spelling of `VariableSource` is pinned: unit variants omit
/// the `from` key entirely, payload variants carry it (as `null` when
/// the form is absent). A flat `Option<FromCrf>` field cannot express
/// both, which is why the enum is adjacently tagged.
#[test]
fn variable_source_serializes_to_the_agreed_json() {
    let cases: Vec<(VariableSource, &str)> = vec![
        (VariableSource::Protocol, r#"{"source":"Protocol"}"#),
        (VariableSource::Derived, r#"{"source":"Derived"}"#),
        (VariableSource::Edt, r#"{"source":"eDT"}"#),
        (
            VariableSource::Crf(Some(FromCrf {
                form_code: "AE".into(),
            })),
            r#"{"source":"CRF","from":{"form_code":"AE"}}"#,
        ),
        (
            VariableSource::Assigned(None),
            r#"{"source":"Assigned","from":null}"#,
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(
            serde_json::to_string(&source).unwrap(),
            expected,
            "unexpected JSON for {source:?}",
        );
    }
}

/// A conversion rule is the source's own fields plus the rule text,
/// side by side — not the source nested under a second `source` key.
#[test]
fn conversion_rule_serializes_the_source_flat() {
    let rule = ConversionRule {
        source: VariableSource::Crf(Some(FromCrf {
            form_code: "AE".into(),
        })),
        rule: "AESEV".into(),
    };
    let json = serde_json::to_string(&rule).unwrap();
    assert_eq!(json, r#"{"source":"CRF","from":{"form_code":"AE"},"rule":"AESEV"}"#);
    let back: ConversionRule = serde_json::from_str(&json).unwrap();
    assert_eq!(back, rule);
}
```

`serde_json` is already a dependency of the `apis` crate, so the test can use it without a `Cargo.toml` change.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p apis sdtm_project_value_types_are_nameable variable_source_serializes_to_the_agreed_json`
Expected: COMPILE ERROR — `cannot find type VariableSource in module apis::sdtm_project` (plus the same for `FromCrf`, `ConversionRule`, `SdtmVariableControlled`).

- [ ] **Step 3: Add the value types to `lib/crates/apis/src/sdtm_project.rs`**

Insert after the `SdtmRole` enum, before the `SdtmProjectApiError` enum:

```rust
/// Where a project variable's value comes from.
///
/// Adjacently tagged: the unit variants are flat strings, the two
/// payload variants carry an optional `from` object. The variant
/// spellings are pinned per-variant — no blanket `rename_all` can
/// produce both `CRF` and `eDT`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", content = "from")]
pub enum VariableSource {
    #[serde(rename = "Protocol")]
    Protocol,
    #[serde(rename = "Derived")]
    Derived,
    #[serde(rename = "Assigned")]
    Assigned(Option<FromCrf>),
    #[serde(rename = "CRF")]
    Crf(Option<FromCrf>),
    #[serde(rename = "eDT")]
    Edt,
}

/// The CRF form a source draws from.
///
/// `form_code` is the code field of the CRF form. It is **not**
/// validated against the `crf` crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FromCrf {
    pub form_code: String,
}

/// One rule that turns a source into a variable's value.
///
/// `rule` is opaque text: this crate neither parses nor evaluates it.
///
/// The source is **flattened** into the rule, so a rule serialises as
/// `{"source": "CRF", "from": {…}, "rule": "…"}` — the source's own
/// fields sit beside the rule text. Without the flatten the source
/// would nest under a second `source` key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversionRule {
    #[serde(flatten)]
    pub source: VariableSource,
    pub rule: String,
}

/// Controlled-terminology metadata attached to an SDTM variable.
///
/// Both lists are optional and are omitted from the JSON when unset,
/// so the stored shape is e.g. `{"code": ["C66731"]}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableControlled {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Vec<String>>,
}
```

- [ ] **Step 4: Add the two error variants to `SdtmProjectApiError`**

In `lib/crates/apis/src/sdtm_project.rs`, add to the `SdtmProjectApiError` enum:

```rust
    #[error("key variable not found in domain: {variable_code}")]
    KeyVariableNotFound { variable_code: String },

    #[error("variable referenced by key variables: {variable_code}")]
    VariableReferencedByKeyVariables { variable_code: String },
```

- [ ] **Step 5: Map the new variants in the server's error layer**

In `apps/server/aegis-server/src/transport/http/error.rs`:

- In `sdtm_project_status`, add a new arm before the `Repository` arm:
  ```rust
        SdtmProjectApiError::KeyVariableNotFound { .. } => StatusCode::BAD_REQUEST,
        SdtmProjectApiError::VariableReferencedByKeyVariables { .. } => StatusCode::CONFLICT,
  ```
- In `sdtm_project_code`, add:
  ```rust
        SdtmProjectApiError::KeyVariableNotFound { .. } => "key_variable_not_found",
        SdtmProjectApiError::VariableReferencedByKeyVariables { .. } => {
            "variable_referenced_by_key_variables"
        }
  ```

- [ ] **Step 6: Add a server-side test for the new mapping**

In `apps/server/aegis-server/src/transport/http/error.rs`, append a test module (create one if the file has none):

```rust
#[cfg(test)]
mod tests {
    use super::{ApiError, StatusCode};
    use apis::sdtm_project::SdtmProjectApiError;

    /// The two key-variable errors map to the documented status codes
    /// and stable codes.
    #[test]
    fn sdtm_project_key_variable_errors_map_to_status_and_code() {
        let missing = ApiError::SdtmProject(SdtmProjectApiError::KeyVariableNotFound {
            variable_code: "AETERM".into(),
        });
        assert_eq!(missing.status(), StatusCode::BAD_REQUEST);
        assert_eq!(missing.code(), "key_variable_not_found");

        let referenced =
            ApiError::SdtmProject(SdtmProjectApiError::VariableReferencedByKeyVariables {
                variable_code: "AETERM".into(),
            });
        assert_eq!(referenced.status(), StatusCode::CONFLICT);
        assert_eq!(referenced.code(), "variable_referenced_by_key_variables");
    }
}
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test -p apis && cargo test -p aegis-server --lib transport::http::error`
Expected: PASS — all tests green.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/apis/src/sdtm_project.rs lib/crates/apis/tests/public_api.rs lib/crates/apis/Cargo.toml apps/server/aegis-server/src/transport/http/error.rs
git commit -m "$(cat <<'EOF'
feat(apis): add sdtm project conversion-rule value types

Adds VariableSource / FromCrf / ConversionRule / SdtmVariableControlled
to the sdtm-project port, plus the two key-variable error variants the
standalone domain model needs. The server's error layer learns the new
variants in the same commit so its exhaustive matches stay exhaustive.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Remove the override sub-resource from the API surface

The override sub-resource (`POST/GET/PUT/DELETE …/override` on both domains and variables) is deleted from the wire, from the `apis` port and from the facade. The `sdtm-project` crate's *internal* override machinery (its domain aggregates, repository ports and usecase methods) survives this task untouched — Task 3 deletes it alongside the rest of the internal rewrite. That is deliberate: it keeps this commit compiling, and the internal pieces become unreachable only at the very end.

**Files:**
- Modify: `lib/crates/apis/src/sdtm_project.rs`
- Modify: `lib/crates/apis/tests/public_api.rs`
- Modify: `lib/crates/sdtm-project/src/adapter/facade/internal/service.rs`
- Modify: `lib/crates/sdtm-project/src/adapter/facade/internal/tests.rs`
- Modify: `apps/server/aegis-server/src/transport/http/sdtm_project/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/sdtm_project/router.rs`
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`
- Modify: `apps/server/aegis-server/src/transport/http/error.rs`
- Modify: `apps/server/aegis-server/src/state.rs`
- Modify: `apps/server/aegis-server/tests/integration_auth.rs`

**Interfaces:**
- Consumes: the `SdtmProjectApiError` variants from Task 1.
- Produces: `apis::sdtm_project::SdtmProjectService` with **no** override methods; `apis::sdtm_project::SdtmProjectDomainView` and `SdtmProjectVariableView` with **no** `domain_override` / `variable_override` field; the server's `/api/sdtm-project` namespace with **no** path containing `/override`.

- [ ] **Step 1: Write the failing test in the server's router tests**

In `apps/server/aegis-server/src/transport/http/router.rs`, inside `mod tests`, edit `openapi_json_advertises_the_sdtm_project_namespace`:

- Delete these eight entries from the `for (method, path) in [...]` loop:
  `("post", "/api/sdtm-project/domains/{domain_id}/override")`,
  `("get", "/api/sdtm-project/domains/{domain_id}/override")`,
  `("put", "/api/sdtm-project/domains/{domain_id}/override")`,
  `("delete", "/api/sdtm-project/domains/{domain_id}/override")`,
  `("post", "/api/sdtm-project/variables/{variable_id}/override")`,
  `("get", "/api/sdtm-project/variables/{variable_id}/override")`,
  `("put", "/api/sdtm-project/variables/{variable_id}/override")`,
  `("delete", "/api/sdtm-project/variables/{variable_id}/override")`.
- Delete the entire second `for (path, schema) in [...]` loop that asserts the override envelope schemas.
- Add this assertion right after the first loop:

```rust
        // The override sub-resource is gone: no route under
        // /api/sdtm-project mentions `/override` any more.
        for (path, item) in doc["paths"].as_object().expect("paths object") {
            assert!(
                !path.contains("/override"),
                "override route still advertised: {path}",
            );
            let _ = item;
        }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p aegis-server --lib transport::http::router::tests::openapi_json_advertises_the_sdtm_project_namespace`
Expected: FAIL — `override route still advertised: /api/sdtm-project/domains/{domain_id}/override`.

- [ ] **Step 3: Delete the override surface from `lib/crates/apis/src/sdtm_project.rs`**

Remove:
- the `SdtmProjectDomainOverrideView` and `SdtmProjectVariableOverrideView` structs;
- the `domain_override` field on `SdtmProjectDomainView` and the `variable_override` field on `SdtmProjectVariableView`;
- the `CreateSdtmProjectDomainOverrideRequest`, `UpdateSdtmProjectDomainOverrideRequest`, `CreateSdtmProjectVariableOverrideRequest` and `UpdateSdtmProjectVariableOverrideRequest` structs;
- the eight trait methods `create_domain_override`, `get_domain_override`, `update_domain_override`, `delete_domain_override`, `create_variable_override`, `get_variable_override`, `update_variable_override`, `delete_variable_override` from `trait SdtmProjectService`, and their doc comments;
- the `SdtmProjectDomainOverrideNotFound`, `SdtmProjectVariableOverrideNotFound`, `DuplicateSdtmProjectDomainOverride` and `DuplicateSdtmProjectVariableOverride` variants from `SdtmProjectApiError`.

- [ ] **Step 4: Update `lib/crates/apis/tests/public_api.rs`**

Remove the override names from the `apis::sdtm_project` import list and delete every `assert_*` line and struct literal that mentions an override type. Keep the `FakeSdtmProjectService` stub in sync: delete its eight override method implementations.

- [ ] **Step 5: Delete the override methods from the facade**

In `lib/crates/sdtm-project/src/adapter/facade/internal/service.rs`:
- delete the eight trait-method implementations named in Step 3;
- delete the `domain_override_view_from_internal` and `variable_override_view_from_internal` mappers;
- in `domain_view_from_internal`, delete the `domain_override:` line; in `variable_view_from_internal`, delete the `variable_override:` line;
- in the `From<UsecaseError> for SdtmProjectApiError` impl, delete the four override arms (`SdtmProjectDomainOverrideNotFound`, `SdtmProjectVariableOverrideNotFound`, `DuplicateSdtmProjectDomainOverride`, `DuplicateSdtmProjectVariableOverride`);
- delete the now-unused imports of the override types from the `apis::sdtm_project::{…}` and `crate::usecase::{…}` import lists.

In `lib/crates/sdtm-project/src/adapter/facade/internal/tests.rs`: delete the tests that exercise the override methods and any override DTOs in the fake service.

- [ ] **Step 6: Delete the override handlers and routes from the server**

In `apps/server/aegis-server/src/transport/http/sdtm_project/handlers.rs`: delete the eight handler functions (`create_domain_override`, `get_domain_override`, `update_domain_override`, `delete_domain_override`, `create_variable_override`, `get_variable_override`, `update_variable_override`, `delete_variable_override`) together with their `#[utoipa::path(...)]` attributes and doc comments, and the two `// ---- SdtmProject…Override ----` section banners.

In `apps/server/aegis-server/src/transport/http/sdtm_project/router.rs`: delete the eight `.routes(routes!(handlers::…_override))` lines and the two `// ---- SdtmProject…Override ----` banners.

- [ ] **Step 7: Delete the override DTOs from the server's wire layer**

In `apps/server/aegis-server/src/transport/http/dto.rs`: delete `CreateSdtmProjectDomainOverrideRequest`, `UpdateSdtmProjectDomainOverrideRequest`, `CreateSdtmProjectVariableOverrideRequest`, `UpdateSdtmProjectVariableOverrideRequest`, `SdtmProjectDomainOverrideViewResponse`, `SdtmProjectVariableOverrideViewResponse`, `SdtmProjectDomainOverrideResponse` and `SdtmProjectVariableOverrideResponse`, their `From<apis::sdtm_project::…>` impls, and their unit tests. Also delete the `domain_override` field on `SdtmProjectDomainViewResponse` and the `variable_override` field on `SdtmProjectVariableViewResponse`, and fix the `From<apis::sdtm_project::SdtmProjectDomainView>` / `…SdtmProjectVariableView>` impls accordingly.

In `apps/server/aegis-server/src/transport/http/openapi.rs`: remove the override schema names from the `components(schemas(…))` list.

In `apps/server/aegis-server/src/transport/http/error.rs`: delete the four override arms from `sdtm_project_status` and `sdtm_project_code`.

- [ ] **Step 8: Update the server's null service and test stub**

In `apps/server/aegis-server/src/state.rs`: delete the eight override method implementations from `impl apis::sdtm_project::SdtmProjectService for NullSdtmProjectService`.

In `apps/server/aegis-server/tests/integration_auth.rs`: delete the eight override method implementations from the local `SdtmProjectService` stub.

- [ ] **Step 9: Run the full workspace build and tests**

Run: `cargo build -p apis -p sdtm-project -p aegis-server && cargo test -p apis -p sdtm-project -p aegis-server`
Expected: BUILD SUCCESS and all tests pass. The `sdtm-project` crate still compiles because its *internal* override machinery is untouched.

- [ ] **Step 10: Commit**

```bash
git add lib/crates/apis lib/crates/sdtm-project/src/adapter/facade apps/server/aegis-server
git commit -m "$(cat <<'EOF'
refactor(sdtm-project): drop the override sub-resource from the API

Removes the override endpoints, the apis port methods behind them and
the facade that implemented them. A domain and a variable now carry
their metadata themselves, so there is nothing left for a separate
override row to hold. The crate's internal override machinery is
removed in the following commit, once the new model is in place.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: The `sdtm-project` crate's standalone model

This task is **internally atomic**: the aggregate shapes, the repository ports, the Postgres DDL and the crates' own test suites all change together, because the DDL is rewritten in place (there is no intermediate schema to migrate through) and the persistence layer binds the aggregate's fields directly. The crate does not compile partway through this task; it compiles at the end of it. Plan the whole task before starting.

The `apis` views keep their old field names (`version_id`, `domain_id`) and do **not** yet carry the new metadata — the facade stops mapping the removed override fields and renames its two FK field reads, and nothing else. Task 4 carries the new metadata and the renamed fields across the `apis` boundary.

**Files:**
- Create: `lib/crates/sdtm-project/src/domain/conversion_rule.rs`
- Modify: `lib/crates/sdtm-project/src/domain/variable_type.rs`
- Modify: `lib/crates/sdtm-project/src/domain/sdtm_project_domain.rs`
- Modify: `lib/crates/sdtm-project/src/domain/sdtm_project_variable.rs`
- Delete: `lib/crates/sdtm-project/src/domain/sdtm_project_domain_override.rs`
- Delete: `lib/crates/sdtm-project/src/domain/sdtm_project_variable_override.rs`
- Modify: `lib/crates/sdtm-project/src/domain/repository.rs`
- Modify: `lib/crates/sdtm-project/src/domain/error.rs`
- Modify: `lib/crates/sdtm-project/src/domain.rs`
- Modify: `lib/crates/sdtm-project/src/domain/tests.rs`
- Modify: `lib/crates/sdtm-project/src/usecase/commands.rs`
- Modify: `lib/crates/sdtm-project/src/usecase/views.rs`
- Modify: `lib/crates/sdtm-project/src/usecase/sdtm_project_usecase.rs`
- Modify: `lib/crates/sdtm-project/src/usecase.rs`
- Modify: `lib/crates/sdtm-project/src/usecase/tests.rs`
- Modify: `lib/crates/sdtm-project/src/adapter/persistence/postgres/sdtm_project_domain_repo.rs`
- Modify: `lib/crates/sdtm-project/src/adapter/persistence/postgres/sdtm_project_variable_repo.rs`
- Delete: `lib/crates/sdtm-project/src/adapter/persistence/postgres/sdtm_project_domain_override_repo.rs`
- Delete: `lib/crates/sdtm-project/src/adapter/persistence/postgres/sdtm_project_variable_override_repo.rs`
- Modify: `lib/crates/sdtm-project/src/adapter/persistence/postgres.rs`
- Modify: `lib/crates/sdtm-project/src/adapter/facade/internal/service.rs`
- Modify: `lib/crates/sdtm-project/src/adapter/facade/internal/tests.rs`
- Modify: `lib/crates/sdtm-project/src/lib.rs`
- Modify: `lib/crates/sdtm-project/migrations/20261103135412_create_sdtm_project_domains.sql`
- Delete: `lib/crates/sdtm-project/migrations/20261103135413_create_sdtm_project_domain_overrides.sql`
- Modify: `lib/crates/sdtm-project/migrations/20261103135414_create_sdtm_project_variables.sql`
- Delete: `lib/crates/sdtm-project/migrations/20261103135416_create_sdtm_project_variable_overrides.sql`
- Modify: `lib/crates/sdtm-project/tests/public_api.rs`
- Modify: `lib/crates/sdtm-project/tests/integration_persistence.rs`

**Interfaces:**
- Consumes: nothing from Tasks 1–2 beyond the still-current `apis` trait.
- Produces (used by Task 4):
  - `sdtm_project::domain::VariableSource`, `FromCrf`, `ConversionRule`, `SdtmVariableControlled`.
  - `SdtmProjectDomain { id, sdtm_project_version_id, domain_code, category, descriptions, key_variables, created_at, updated_at }`.
  - `SdtmProjectVariable { id, sdtm_project_domain_id, variable_code, variable_controlled, variable_type, variable_core, variable_role, variable_sequence, descriptions, conversion_rules, created_at, updated_at }`.
  - `SdtmProjectUsecase<V, D, Va, P>` — three repositories plus the project lookup (was five plus).
  - `SdtmProjectDomainView` / `SdtmProjectVariableView` carrying the fields above.

- [ ] **Step 1: Write the failing domain-layer tests in `lib/crates/sdtm-project/src/domain/tests.rs`**

Replace the override imports and the override aggregate tests. Add:

```rust
#[test]
fn variable_source_serializes_to_the_agreed_json() {
    let cases: Vec<(VariableSource, &str)> = vec![
        (VariableSource::Protocol, r#"{"source":"Protocol"}"#),
        (VariableSource::Derived, r#"{"source":"Derived"}"#),
        (VariableSource::Edt, r#"{"source":"eDT"}"#),
        (
            VariableSource::Crf(Some(FromCrf {
                form_code: "AE".into(),
            })),
            r#"{"source":"CRF","from":{"form_code":"AE"}}"#,
        ),
        (
            VariableSource::Assigned(None),
            r#"{"source":"Assigned","from":null}"#,
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(serde_json::to_string(&source).unwrap(), expected);
    }
}

#[test]
fn sdtm_variable_controlled_omits_unset_members() {
    let json = serde_json::to_string(&SdtmVariableControlled {
        code: Some(vec!["C66731".into()]),
        format: None,
    })
    .unwrap();
    assert_eq!(json, r#"{"code":["C66731"]}"#);
}

#[test]
fn sdtm_project_variable_new_rejects_blank_variable_code() {
    let err = SdtmProjectVariable::new(
        1,
        "   ".into(),
        None,
        SdtmVariableType::Character,
        SdtmVariableCore::Req,
        None,
        1,
        Vec::new(),
        Vec::new(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyVariableCode));
}

#[test]
fn sdtm_project_variable_new_rejects_a_blank_conversion_rule() {
    let err = SdtmProjectVariable::new(
        1,
        "AETERM".into(),
        None,
        SdtmVariableType::Character,
        SdtmVariableCore::Req,
        None,
        1,
        Vec::new(),
        vec![ConversionRule {
            source: VariableSource::Protocol,
            rule: "   ".into(),
        }],
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyConversionRule));
}
```

Update `sdtm_project_domain_new_rejects_blank_domain_code` to the new constructor signature:

```rust
#[test]
fn sdtm_project_domain_new_rejects_blank_domain_code() {
    let err = SdtmProjectDomain::new(
        1,
        "  ".into(),
        DomainCategory::Events,
        Vec::new(),
        Vec::new(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyDomainCode));
}
```

`serde_json` is already a dependency of the `sdtm-project` crate.

- [ ] **Step 2: Write the failing usecase-layer tests in `lib/crates/sdtm-project/src/usecase/tests.rs`**

The in-memory fakes in this file must shrink to three repositories (version, domain, variable) plus the project lookup; delete the `domain_overrides` and `variable_overrides` collections and the two override repository impls.

Add these tests (the helper that builds a usecase over the in-memory store is named `usecase()` in this file — reuse whatever the existing tests use to construct one; if none exists, add `fn usecase() -> SdtmProjectUsecase<InMemoryVersionRepo, InMemoryDomainRepo, InMemoryVariableRepo, InMemoryProjectLookup>` returning the four in-memory implementations over a shared `Arc<InMemoryStore>`):

```rust
#[tokio::test]
async fn create_domain_with_key_variables_is_rejected() {
    let u = usecase();
    let version = u.create_version(CreateSdtmProjectVersion { project_code: "P1".into(), version_name: "v1".into() }).await.unwrap();
    let err = u.create_domain(CreateSdtmProjectDomain {
        sdtm_project_version_id: version.id,
        domain_code: "AE".into(),
        category: DomainCategory::Events,
        descriptions: Vec::new(),
        key_variables: vec!["AETERM".into()],
    }).await.unwrap_err();
    assert!(matches!(err, UsecaseError::Repository(DomainError::KeyVariableNotFound { variable_code }) if variable_code == "AETERM"));
}

#[tokio::test]
async fn update_domain_accepts_key_variable_that_names_an_existing_variable() {
    // create version → domain → variable "AETERM" → update domain with
    // key_variables ["AETERM"] → Ok, and the view carries the list.
}

#[tokio::test]
async fn update_domain_rejects_key_variable_that_names_no_variable() {
    // … domain with a variable "AETERM"; update with ["AESTDTC"] →
    // Err(KeyVariableNotFound { variable_code: "AESTDTC" }).
}

#[tokio::test]
async fn update_domain_rejects_key_variable_from_another_domain() {
    // Two domains under one version; domain A has "AETERM", domain B
    // does not. Updating domain B with key_variables ["AETERM"] →
    // Err(KeyVariableNotFound { variable_code: "AETERM" }). The check
    // is scoped to the domain being written, not the project.
}

#[tokio::test]
async fn update_domain_with_empty_key_variables_clears_the_list() {
    // Domain with a variable "AETERM"; set key_variables ["AETERM"];
    // then update with key_variables Some(vec![]) → the view's
    // key_variables is empty (not unchanged).
}

#[tokio::test]
async fn update_variable_rejects_rename_when_the_old_code_is_a_key_variable() {
    // Domain with variable "AETERM" and key_variables ["AETERM"];
    // update_variable { variable_code: Some("AETERMX") } →
    // Err(VariableReferencedByKeyVariables { variable_code: "AETERM" }),
    // and the variable still carries "AETERM" afterwards.
}

#[tokio::test]
async fn delete_variable_rejects_when_the_code_is_a_key_variable() {
    // … → Err(VariableReferencedByKeyVariables { variable_code: "AETERM" }),
    // and the variable still exists afterwards.
}

#[tokio::test]
async fn delete_variable_succeeds_when_the_code_is_not_a_key_variable() {
    // … → Ok(()).
}

#[tokio::test]
async fn create_variable_stores_conversion_rules() {
    // create_variable with one ConversionRule → the view's
    // conversion_rules round-trips the source and the rule text.
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p sdtm-project --lib`
Expected: COMPILE ERROR — `cannot find type VariableSource in module crate::domain` and the new constructor arities do not match.

- [ ] **Step 4: Create `lib/crates/sdtm-project/src/domain/conversion_rule.rs`**

```rust
use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Where a project variable's value comes from.
///
/// Adjacently tagged: the unit variants are flat strings, the two
/// payload variants carry an optional `from` object. The variant
/// spellings are pinned per-variant — no blanket `rename_all` can
/// produce both `CRF` and `eDT`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", content = "from")]
pub enum VariableSource {
    #[serde(rename = "Protocol")]
    Protocol,
    #[serde(rename = "Derived")]
    Derived,
    #[serde(rename = "Assigned")]
    Assigned(Option<FromCrf>),
    #[serde(rename = "CRF")]
    Crf(Option<FromCrf>),
    #[serde(rename = "eDT")]
    Edt,
}

/// The CRF form a source draws from.
///
/// `form_code` is the code field of the CRF form. It is **not**
/// validated against the `crf` crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FromCrf {
    pub form_code: String,
}

/// One rule that turns a source into a variable's value.
///
/// `rule` is opaque text: this crate neither parses nor evaluates it.
///
/// The source is **flattened** into the rule, so a rule serialises as
/// `{"source": "CRF", "from": {…}, "rule": "…"}` — the source's own
/// fields sit beside the rule text. Without the flatten the source
/// would nest under a second `source` key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversionRule {
    #[serde(flatten)]
    pub source: VariableSource,
    pub rule: String,
}

impl ConversionRule {
    /// Validating constructor used by the domain layer. Rejects a
    /// blank `rule` — a rule with no expression carries no information.
    pub fn new(source: VariableSource, rule: String) -> Result<Self, DomainError> {
        if rule.trim().is_empty() {
            return Err(DomainError::EmptyConversionRule);
        }
        Ok(Self { source, rule })
    }
}
```

- [ ] **Step 5: Add `SdtmVariableControlled` to `lib/crates/sdtm-project/src/domain/variable_type.rs`**

Append (copied from `domain-model/src/domain/variable_type.rs`):

```rust
/// Controlled-terminology metadata attached to an SDTM variable.
///
/// Carried on the `SdtmProjectVariable` aggregate and persisted as a
/// single JSONB column on `sdtm_project_variables`. Both lists are
/// optional and are omitted from the JSON when unset, so the stored
/// shape is e.g. `{"code": ["C66731"]}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableControlled {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Vec<String>>,
}
```

- [ ] **Step 6: Rewrite `lib/crates/sdtm-project/src/domain/sdtm_project_domain.rs`**

Replace the file's contents with these three items. Keep the file's existing `Debug` impl, updated with the new field names (`sdtm_project_version_id`, `category`, `descriptions`), and keep `normalize_key_variables` exactly as it is.

```rust
/// A domain a project declares inside one of its SDTM model versions.
///
/// The aggregate is a **complete copy** of the domain: it carries the
/// category and the descriptions itself, rather than pointing at a
/// standard domain and patching it.
///
/// `key_variables` is an **ordered** collection: the order the caller
/// supplies is meaningful and is preserved through the JSONB column
/// and out to the wire. It still has set semantics in one respect —
/// duplicates collapse to their first occurrence. Every code it names
/// must be a variable of this same domain; the usecase enforces that.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmProjectDomain {
    pub id: i64,
    pub sdtm_project_version_id: i64,
    pub domain_code: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub key_variables: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SdtmProjectDomain {
    /// Validating constructor used by the domain layer. Rejects an
    /// empty / whitespace `domain_code` and a blank key-variable code.
    pub fn new(
        sdtm_project_version_id: i64,
        domain_code: String,
        category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
        key_variables: Vec<String>,
    ) -> Result<Self, DomainError>;

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i64,
        sdtm_project_version_id: i64,
        domain_code: String,
        category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
        key_variables: Vec<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self;
}

/// Input DTO for `SdtmProjectDomainRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmProjectDomainNew {
    pub sdtm_project_version_id: i64,
    pub domain_code: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub key_variables: Vec<String>,
}

/// Input DTO for `SdtmProjectDomainRepository::update`.
/// `sdtm_project_version_id` is immutable — a domain always belongs to
/// the version it was created under.
#[derive(Debug, Clone, Default)]
pub struct SdtmProjectDomainUpdate {
    pub id: i64,
    pub domain_code: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
    pub key_variables: Option<Vec<String>>,
}
```

`new` rejects a blank `domain_code` with `DomainError::EmptyDomainCode` and normalises `key_variables` through `Self::normalize_key_variables` (unchanged). `for_repository` stores its arguments as given.

- [ ] **Step 7: Rewrite `lib/crates/sdtm-project/src/domain/sdtm_project_variable.rs`**

Replace the file's contents with these three items. Keep the file's existing `Debug` impl, updated with the new field names.

```rust
/// A variable a project declares inside one of its domains.
///
/// The aggregate is a **complete copy** of the variable: it carries the
/// type, the core, the role, the controlled terminology and the
/// descriptions itself, rather than pointing at a standard variable and
/// patching it.
///
/// `variable_sequence` is the column order within the parent domain.
/// `conversion_rules` says where the variable's value comes from and
/// how it is derived.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmProjectVariable {
    pub id: i64,
    pub sdtm_project_domain_id: i64,
    pub variable_code: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub conversion_rules: Vec<ConversionRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SdtmProjectVariable {
    /// Validating constructor used by the domain layer. Rejects an
    /// empty / whitespace `variable_code` and a blank conversion rule.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sdtm_project_domain_id: i64,
        variable_code: String,
        variable_controlled: Option<SdtmVariableControlled>,
        variable_type: SdtmVariableType,
        variable_core: SdtmVariableCore,
        variable_role: Option<SdtmRole>,
        variable_sequence: i64,
        descriptions: Vec<SdtmVariableDescription>,
        conversion_rules: Vec<ConversionRule>,
    ) -> Result<Self, DomainError>;

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i64,
        sdtm_project_domain_id: i64,
        variable_code: String,
        variable_controlled: Option<SdtmVariableControlled>,
        variable_type: SdtmVariableType,
        variable_core: SdtmVariableCore,
        variable_role: Option<SdtmRole>,
        variable_sequence: i64,
        descriptions: Vec<SdtmVariableDescription>,
        conversion_rules: Vec<ConversionRule>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self;
}

/// Input DTO for `SdtmProjectVariableRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmProjectVariableNew {
    pub sdtm_project_domain_id: i64,
    pub variable_code: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub conversion_rules: Vec<ConversionRule>,
}

/// Input DTO for `SdtmProjectVariableRepository::update`.
/// `sdtm_project_domain_id` is immutable — a variable always belongs
/// to the domain it was created under.
///
/// `variable_controlled` and `variable_role` use `Option<Option<T>>` so
/// the caller can distinguish "don't change" (outer `None`) from
/// "clear the field" (outer `Some(None)`); the other fields use flat
/// `Option<T>` where `None` means "don't change" and `Some(value)`
/// means "replace".
#[derive(Debug, Clone, Default)]
pub struct SdtmProjectVariableUpdate {
    pub id: i64,
    pub variable_code: Option<String>,
    pub variable_controlled: Option<Option<SdtmVariableControlled>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
    pub conversion_rules: Option<Vec<ConversionRule>>,
}
```

`new` rejects a blank `variable_code` with `DomainError::EmptyVariableCode` and a conversion rule whose `rule` is blank with `DomainError::EmptyConversionRule`. `for_repository` stores its arguments as given.

- [ ] **Step 8: Delete the override aggregate modules and rewrite the domain error**

Delete `lib/crates/sdtm-project/src/domain/sdtm_project_domain_override.rs` and `lib/crates/sdtm-project/src/domain/sdtm_project_variable_override.rs`.

In `lib/crates/sdtm-project/src/domain/error.rs`:
- delete the `SdtmProjectDomainOverrideNotFound`, `SdtmProjectVariableOverrideNotFound`, `DuplicateSdtmProjectDomainOverride` and `DuplicateSdtmProjectVariableOverride` variants;
- add to the validation group:
  ```rust
      #[error("empty conversion rule")]
      EmptyConversionRule,
  ```
- add to the existence group:
  ```rust
      #[error("key variable not found in domain: {variable_code}")]
      KeyVariableNotFound { variable_code: String },

      #[error("variable is referenced by key variables: {variable_code}")]
      VariableReferencedByKeyVariables { variable_code: String },
  ```

- [ ] **Step 9: Rewrite the repository ports**

In `lib/crates/sdtm-project/src/domain/repository.rs`: delete the `SdtmProjectDomainOverrideRepository` and `SdtmProjectVariableOverrideRepository` traits and their imports. Rewrite the two surviving traits' doc comments to drop the "the read methods hydrate the override" paragraph.

- [ ] **Step 10: Update the domain module declarations**

In `lib/crates/sdtm-project/src/domain.rs`: add `mod conversion_rule;`, delete `mod sdtm_project_domain_override;` and `mod sdtm_project_variable_override;`, and update the `pub use` block:

```rust
pub use conversion_rule::{ConversionRule, FromCrf, VariableSource};
pub use variable_type::{SdtmRole, SdtmVariableControlled, SdtmVariableCore, SdtmVariableType};
```
plus remove the override re-exports.

- [ ] **Step 11: Rewrite the usecase commands and views**

In `lib/crates/sdtm-project/src/usecase/commands.rs`, replace the domain and variable command structs and delete the four override command structs:

```rust
#[derive(Debug, Clone)]
pub struct CreateSdtmProjectDomain {
    pub sdtm_project_version_id: i64,
    pub domain_code: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub key_variables: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateSdtmProjectDomain {
    pub id: i64,
    pub domain_code: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
    pub key_variables: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct CreateSdtmProjectVariable {
    pub sdtm_project_domain_id: i64,
    pub variable_code: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub conversion_rules: Vec<ConversionRule>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateSdtmProjectVariable {
    pub id: i64,
    pub variable_code: Option<String>,
    pub variable_controlled: Option<Option<SdtmVariableControlled>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
    pub conversion_rules: Option<Vec<ConversionRule>>,
}
```

In `lib/crates/sdtm-project/src/usecase/views.rs`, replace the two view structs and delete the two override views:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectDomainView {
    pub id: i64,
    pub sdtm_project_version_id: i64,
    pub domain_code: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub key_variables: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmProjectDomain> for SdtmProjectDomainView {
    fn from(d: SdtmProjectDomain) -> Self {
        Self {
            id: d.id,
            sdtm_project_version_id: d.sdtm_project_version_id,
            domain_code: d.domain_code,
            category: d.category,
            descriptions: d.descriptions,
            key_variables: d.key_variables,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVariableView {
    pub id: i64,
    pub sdtm_project_domain_id: i64,
    pub variable_code: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub conversion_rules: Vec<ConversionRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

- [ ] **Step 12: Rewrite the usecase**

In `lib/crates/sdtm-project/src/usecase/sdtm_project_usecase.rs`:

- `SdtmProjectUsecaseConfig<V, D, Va, P>` and `SdtmProjectUsecase<V, D, Va, P>` lose the two override repositories. The struct fields become `version_repo`, `domain_repo`, `variable_repo`, `projects`.
- `create_domain` becomes:
  ```rust
      pub async fn create_domain(
          &self,
          cmd: CreateSdtmProjectDomain,
      ) -> Result<SdtmProjectDomainView, UsecaseError> {
          validate_create_domain(&cmd)?;
          let key_variables = SdtmProjectDomain::normalize_key_variables(&cmd.key_variables)
              .map_err(UsecaseError::Validation)?;
          ensure_version_exists(&self.version_repo, cmd.sdtm_project_version_id).await?;
          // A brand-new domain has no variables yet, so any key
          // variable it names is missing by definition. Checked here
          // rather than through `ensure_key_variables_exist` because
          // there is no domain id to scope the lookup to.
          if let Some(code) = key_variables.first() {
              return Err(UsecaseError::Repository(DomainError::KeyVariableNotFound {
                  variable_code: code.clone(),
              }));
          }
          let d = self
              .domain_repo
              .create(SdtmProjectDomainNew {
                  sdtm_project_version_id: cmd.sdtm_project_version_id,
                  domain_code: cmd.domain_code,
                  category: cmd.category,
                  descriptions: cmd.descriptions,
                  key_variables,
              })
              .await?;
          Ok(d.into())
      }
  ```
- `update_domain` gains the same key-variable check against `cmd.id`:
  ```rust
          let key_variables = match &cmd.key_variables {
              None => None,
              Some(kv) => {
                  let kv = SdtmProjectDomain::normalize_key_variables(kv)
                      .map_err(UsecaseError::Validation)?;
                  ensure_key_variables_exist(&self.variable_repo, cmd.id, &kv).await?;
                  Some(kv)
              }
          };
  ```
- `update_variable` gains the rename guard:
  ```rust
          validate_update_variable(&cmd)?;
          if let Some(ref new_code) = cmd.variable_code {
              let current = self.variable_repo.find_by_id(cmd.id).await?;
              if new_code != &current.variable_code {
                  ensure_not_a_key_variable(&self.domain_repo, &self.variable_repo, &current).await?;
              }
          }
  ```
- `delete_variable` gains the same guard before the delete:
  ```rust
      pub async fn delete_variable(&self, id: i64) -> Result<(), UsecaseError> {
          let current = self.variable_repo.find_by_id(id).await?;
          ensure_not_a_key_variable(&self.domain_repo, &self.variable_repo, &current).await?;
          self.variable_repo.delete(id).await?;
          Ok(())
      }
  ```
- Add the two helpers:
  ```rust
  /// Every code in `key_variables` must name a variable of `domain_id`.
  ///
  /// The check is scoped to the domain being written — a code that
  /// exists in a *sibling* domain is still missing here.
  async fn ensure_key_variables_exist<Va>(
      repo: &Va,
      domain_id: i64,
      key_variables: &[String],
  ) -> Result<(), UsecaseError>
  where
      Va: SdtmProjectVariableRepository,
  {
      if key_variables.is_empty() {
          return Ok(());
      }
      let vars = repo.list_by_domain(domain_id).await?;
      for code in key_variables {
          if !vars.iter().any(|v| v.variable_code == *code) {
              return Err(UsecaseError::Repository(DomainError::KeyVariableNotFound {
                  variable_code: code.clone(),
              }));
          }
      }
      Ok(())
  }

  /// A variable that a domain names as a key variable cannot be
  /// renamed or deleted out from under it. The caller must drop the
  /// code from `key_variables` first.
  async fn ensure_not_a_key_variable<D, Va>(
      domain_repo: &D,
      variable_repo: &Va,
      variable: &SdtmProjectVariable,
  ) -> Result<(), UsecaseError>
  where
      D: SdtmProjectDomainRepository,
      Va: SdtmProjectVariableRepository,
  {
      let domain = domain_repo.find_by_id(variable.sdtm_project_domain_id).await?;
      if domain
          .key_variables
          .iter()
          .any(|code| code == &variable.variable_code)
      {
          return Err(UsecaseError::Repository(
              DomainError::VariableReferencedByKeyVariables {
                  variable_code: variable.variable_code.clone(),
              },
          ));
      }
      Ok(())
  }
  ```
- `validate_create_variable` and `validate_update_variable` gain a blank-`rule` check over `conversion_rules`:
  ```rust
      for rule in rules {
          if rule.rule.trim().is_empty() {
              return Err(UsecaseError::Validation(DomainError::EmptyConversionRule));
          }
      }
  ```
- `ensure_domain_exists` is called with `cmd.sdtm_project_domain_id` in `create_variable`.
- Delete the four override usecase methods.

- [ ] **Step 13: Update the usecase module's re-exports**

In `lib/crates/sdtm-project/src/usecase.rs`: remove the four override command re-exports and the two override view re-exports; the remaining names are unchanged.

- [ ] **Step 14: Rewrite the two Postgres repositories**

In `lib/crates/sdtm-project/src/adapter/persistence/postgres/sdtm_project_domain_repo.rs`:

- the row struct becomes:
  ```rust
  #[derive(FromRow)]
  struct SdtmProjectDomainRow {
      id: i64,
      sdtm_project_version_id: i64,
      domain_code: String,
      category: String,
      descriptions: JsonValue,
      key_variables: JsonValue,
      created_at: DateTime<Utc>,
      updated_at: DateTime<Utc>,
  }
  ```
- `into_domain` parses `category` with `DomainCategory::try_from(self.category.as_str())?` and `descriptions` with `serde_json::from_value`, then calls `SdtmProjectDomain::for_repository` with the new argument order.
- `create` becomes a plain insert — the `LEFT JOIN` is gone:
  ```sql
  INSERT INTO sdtm_project_domains
      (sdtm_project_version_id, domain_code, category, descriptions, key_variables)
  VALUES ($1, $2, $3, $4, $5)
  RETURNING id, sdtm_project_version_id, domain_code, category, descriptions,
            key_variables, created_at, updated_at
  ```
- `find_by_id` and `list_by_version` select the same column list, with `WHERE sdtm_project_version_id = $1` for the list.
- `update` becomes:
  ```sql
  UPDATE sdtm_project_domains
  SET domain_code   = COALESCE($2, domain_code),
      category      = COALESCE($3, category),
      descriptions  = COALESCE($4, descriptions),
      key_variables = COALESCE($5, key_variables)
  WHERE id = $1
  RETURNING id, sdtm_project_version_id, domain_code, category, descriptions,
            key_variables, created_at, updated_at
  ```
  (bind `$3` as `input.category.map(|c| c.as_str())`.)
- `map_domain_err` keeps its shape; the unique-violation arm still reports `DuplicateSdtmProjectDomain`.

In `lib/crates/sdtm-project/src/adapter/persistence/postgres/sdtm_project_variable_repo.rs`:

- the row struct becomes:
  ```rust
  #[derive(FromRow)]
  struct SdtmProjectVariableRow {
      id: i64,
      sdtm_project_domain_id: i64,
      variable_code: String,
      variable_controlled: Option<JsonValue>,
      variable_type: String,
      variable_core: String,
      variable_role: Option<String>,
      variable_sequence: i64,
      descriptions: JsonValue,
      conversion_rules: JsonValue,
      created_at: DateTime<Utc>,
      updated_at: DateTime<Utc>,
  }
  ```
- `into_variable` parses `variable_type` / `variable_core` / `variable_role` with the existing `try_from` helpers, `variable_controlled` with `serde_json::from_value` (a SQL `NULL` arrives as `None` → `Ok(None)`), `descriptions` and `conversion_rules` with `serde_json::from_value`.
- `create` becomes a plain insert:
  ```sql
  INSERT INTO sdtm_project_variables
      (sdtm_project_domain_id, variable_code, variable_controlled, variable_type,
       variable_core, variable_role, variable_sequence, descriptions, conversion_rules)
  VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
  RETURNING id, sdtm_project_domain_id, variable_code, variable_controlled,
            variable_type, variable_core, variable_role, variable_sequence,
            descriptions, conversion_rules, created_at, updated_at
  ```
- `update` uses the three-state pattern for the two nullable columns:
  ```sql
  UPDATE sdtm_project_variables
  SET variable_code      = COALESCE($2, variable_code),
      variable_sequence  = COALESCE($3, variable_sequence),
      variable_type      = COALESCE($4, variable_type),
      variable_core      = COALESCE($5, variable_core),
      descriptions       = COALESCE($6, descriptions),
      conversion_rules   = COALESCE($7, conversion_rules),
      variable_role      = CASE WHEN $8 THEN $9::text ELSE variable_role END,
      variable_controlled = CASE WHEN $10 THEN $11::jsonb ELSE variable_controlled END
  WHERE id = $1
  RETURNING id, sdtm_project_domain_id, variable_code, variable_controlled,
            variable_type, variable_core, variable_role, variable_sequence,
            descriptions, conversion_rules, created_at, updated_at
  ```
  with the bindings: `$8` = `input.variable_role.is_some()`, `$9` = `input.variable_role.flatten().map(|r| r.as_str())`, `$10` = `input.variable_controlled.is_some()`, `$11` = `input.variable_controlled.flatten().map(|c| serde_json::to_value(c))` — bind `None` when the inner is `None`, so "clear" writes SQL `NULL` rather than JSON `null`.
- delete both override repository files and remove their `pub mod` lines from `adapter/persistence/postgres.rs`.

- [ ] **Step 15: Rewrite the two migration files and delete the two override migrations**

Rewrite `lib/crates/sdtm-project/migrations/20261103135412_create_sdtm_project_domains.sql` to:

```sql
-- sdtm_project_domains: one row per (version, domain code).
--
-- The domain is a complete copy: it carries its own `category` and
-- `descriptions` rather than pointing at a standard domain and
-- patching it. `key_variables` is a JSONB array of variable codes
-- kept in the order the caller supplied; every code it names must be
-- a variable of this same domain (enforced by the usecase, not by
-- the database).

CREATE TABLE IF NOT EXISTS sdtm_project_domains (
    id                       BIGSERIAL PRIMARY KEY,
    sdtm_project_version_id  BIGINT      NOT NULL REFERENCES sdtm_project_versions(id) ON DELETE CASCADE,
    domain_code              TEXT        NOT NULL,
    category                 TEXT        NOT NULL,
    descriptions             JSONB       NOT NULL DEFAULT '[]'::jsonb,
    key_variables            JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_domains_version_code_unique UNIQUE (sdtm_project_version_id, domain_code),
    CONSTRAINT sdtm_project_domains_category_check CHECK (
        category IN (
            'Special Purpose',
            'Interventions',
            'Events',
            'Findings',
            'Trial Design',
            'Relationships',
            'Study Reference'
        )
    ),
    CONSTRAINT sdtm_project_domains_descriptions_is_array CHECK (jsonb_typeof(descriptions) = 'array'),
    CONSTRAINT sdtm_project_domains_key_variables_is_array CHECK (jsonb_typeof(key_variables) = 'array')
);

CREATE OR REPLACE FUNCTION sdtm_project_domains_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_domains_updated_at
BEFORE UPDATE ON sdtm_project_domains
FOR EACH ROW EXECUTE FUNCTION sdtm_project_domains_set_updated_at();
```

Rewrite `lib/crates/sdtm-project/migrations/20261103135414_create_sdtm_project_variables.sql` to:

```sql
-- sdtm_project_variables: one row per (domain, variable code).
--
-- The variable is a complete copy: it carries its own type, core,
-- role, controlled terminology and descriptions rather than pointing
-- at a standard variable and patching it. `conversion_rules` is a
-- JSONB array of {source, rule} objects saying where the value comes
-- from and how it is derived.

CREATE TABLE IF NOT EXISTS sdtm_project_variables (
    id                       BIGSERIAL PRIMARY KEY,
    sdtm_project_domain_id  BIGINT      NOT NULL REFERENCES sdtm_project_domains(id) ON DELETE CASCADE,
    variable_code            TEXT        NOT NULL,
    variable_sequence        BIGINT      NOT NULL,
    variable_controlled      JSONB,
    variable_type            TEXT        NOT NULL,
    variable_core            TEXT        NOT NULL,
    variable_role            TEXT,
    descriptions             JSONB       NOT NULL DEFAULT '[]'::jsonb,
    conversion_rules         JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_variables_domain_code_unique UNIQUE (sdtm_project_domain_id, variable_code),
    CONSTRAINT sdtm_project_variables_type_check CHECK (variable_type IN ('Numeric', 'Character')),
    CONSTRAINT sdtm_project_variables_core_check CHECK (variable_core IN ('Req', 'Exp', 'Perm', 'Supp')),
    CONSTRAINT sdtm_project_variables_role_check CHECK (
        variable_role IS NULL OR variable_role IN (
            'Identifier',
            'Topic',
            'Timing',
            'Record Qualifier',
            'Synonym Qualifier',
            'Variable Qualifier',
            'Grouping Qualifier',
            'Rule'
        )
    ),
    CONSTRAINT sdtm_project_variables_descriptions_is_array CHECK (jsonb_typeof(descriptions) = 'array'),
    CONSTRAINT sdtm_project_variables_conversion_rules_is_array CHECK (jsonb_typeof(conversion_rules) = 'array')
);

CREATE OR REPLACE FUNCTION sdtm_project_variables_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_project_variables_updated_at
BEFORE UPDATE ON sdtm_project_variables
FOR EACH ROW EXECUTE FUNCTION sdtm_project_variables_set_updated_at();
```

Delete `20261103135413_create_sdtm_project_domain_overrides.sql` and `20261103135416_create_sdtm_project_variable_overrides.sql`.

- [ ] **Step 16: Update the facade's field reads**

In `lib/crates/sdtm-project/src/adapter/facade/internal/service.rs`:
- `domain_view_from_internal`: change `version_id: view.version_id` to `version_id: view.sdtm_project_version_id`;
- `variable_view_from_internal`: change `domain_id: view.domain_id` to `domain_id: view.sdtm_project_domain_id`;
- leave the rest of the mapping alone — the `apis` views do not carry the new metadata yet (Task 4 does that).

- [ ] **Step 17: Update the crate's re-exports**

In `lib/crates/sdtm-project/src/lib.rs`: remove the override re-exports (`SdtmProjectDomainOverride`, `SdtmProjectDomainOverrideNew`, `SdtmProjectDomainOverrideRepository`, `SdtmProjectDomainOverrideUpdate`, `SdtmProjectVariableOverride`, `SdtmProjectVariableOverrideNew`, `SdtmProjectVariableOverrideRepository`, `SdtmProjectVariableOverrideUpdate`, `SdtmProjectDomainOverrideRepoPg`, `SdtmProjectVariableOverrideRepoPg`, `SdtmProjectDomainOverrideView`, `SdtmProjectVariableOverrideView`, `CreateSdtmProjectDomainOverride`, `UpdateSdtmProjectDomainOverride`, `CreateSdtmProjectVariableOverride`, `UpdateSdtmProjectVariableOverride`), and add `ConversionRule`, `FromCrf`, `VariableSource`, `SdtmVariableControlled`.

- [ ] **Step 18: Update the crate's public-API test**

In `lib/crates/sdtm-project/tests/public_api.rs`: update the import list and the `assert_*` functions to the new surface — drop every override name, add `assert_conversion_rule`, `assert_from_crf`, `assert_variable_source`, `assert_var_controlled`, and update the two aggregate constructor calls to the new arities:

```rust
    assert_domain_aggregate(
        SdtmProjectDomain::new(
            1,
            "AE".into(),
            DomainCategory::Events,
            Vec::new(),
            Vec::new(),
        )
        .unwrap(),
    );
    assert_variable_aggregate(
        SdtmProjectVariable::new(
            1,
            "AETERM".into(),
            None,
            SdtmVariableType::Character,
            SdtmVariableCore::Req,
            None,
            1,
            Vec::new(),
            Vec::new(),
        )
        .unwrap(),
    );
```

Also update the `SdtmProjectUsecaseConfig` construction in this file to the three-repository shape.

- [ ] **Step 19: Update the live-DB integration test**

In `lib/crates/sdtm-project/tests/integration_persistence.rs`:
- remove the two override tables from the drop list;
- delete the override repository tests;
- update the domain and variable tests to the new DTO shapes;
- add these tests:

```rust
#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn domain_round_trips_category_and_descriptions() { /* create → find_by_id → assert category + descriptions match */ }

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn conversion_rules_round_trip_through_the_repository() {
    // create a variable with two rules — one `Protocol`, one
    // `Crf(Some(FromCrf { form_code }))` — then read it back and
    // assert both rules survive verbatim, including the `from` object.
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_without_controlled_terminology_round_trips_as_null() {
    // create a variable with `variable_controlled: None`, read it
    // back, assert `variable_controlled` is `None` (not `Some`).
}

#[tokio::test]
#[ignore = "requires AEGIS_TEST_DATABASE_URL"]
async fn variable_controlled_round_trips_through_the_repository() {
    // create with `Some(SdtmVariableControlled { code: Some(vec![…]), format: None })`
    // and read it back.
}
```

- [ ] **Step 20: Update the Postgres repos' migration-content unit tests**

In both repo files, rewrite the `#[cfg(test)] mod tests` unit test to assert the new DDL:

```rust
    #[test]
    fn migration_file_declares_domain_tables() {
        let sql =
            include_str!("../../../../migrations/20261103135412_create_sdtm_project_domains.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_project_domains"));
        assert!(sql.contains("sdtm_project_version_id  BIGINT"));
        assert!(sql.contains("CONSTRAINT sdtm_project_domains_version_code_unique UNIQUE (sdtm_project_version_id, domain_code)"));
        assert!(sql.contains("category                 TEXT        NOT NULL"));
        assert!(sql.contains("sdtm_project_domains_updated_at"));
        assert!(!sql.contains("ALTER TABLE"));
    }
```

and the variable equivalent, asserting `conversion_rules`, `variable_controlled`, the domain-code unique constraint, and `!sql.contains("ALTER TABLE")`.

- [ ] **Step 21: Run the crate's tests to verify they pass**

Run: `cargo test -p sdtm-project --lib && cargo test -p sdtm-project --test public_api`
Expected: PASS.

- [ ] **Step 22: Run the workspace build and the live-DB tests**

Run: `cargo build --workspace`
Expected: BUILD SUCCESS. The `apis` views still use the old field names, and the server still compiles against the old `apis` surface.

Run: `cargo test -p sdtm-project -- --ignored --test-threads=1`
Expected: PASS — requires `AEGIS_TEST_DATABASE_URL` pointing at a disposable database.

- [ ] **Step 23: Commit**

```bash
git add lib/crates/sdtm-project
git commit -m "$(cat <<'EOF'
refactor(sdtm-project): make project domains and variables standalone copies

A project domain now carries its own category and descriptions, and a
project variable its own type, core, role, controlled terminology and
descriptions, so the override tables and the reference-to-the-standard
design are gone. The DDL is rewritten in place rather than amended.

A variable gains conversion_rules, naming where its value comes from
and how it is derived. A domain's key_variables must always name
variables that exist in that domain: renaming or deleting a referenced
variable is rejected rather than silently rewriting the study's key.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Carry the new model across the `apis` boundary and onto the wire

The `apis` views and requests gain the metadata the crate now carries, and the two FK fields are renamed to match the crate. The facade maps the new fields. The server's DTOs, handlers and OpenAPI schemas follow, because a change to an `apis` view breaks the server's `From` impls and its handlers in the same breath — the two halves cannot be committed apart without leaving the workspace broken.

**Files:**
- Modify: `lib/crates/apis/src/sdtm_project.rs`
- Modify: `lib/crates/apis/tests/public_api.rs`
- Modify: `lib/crates/sdtm-project/src/adapter/facade/internal/service.rs`
- Modify: `lib/crates/sdtm-project/src/adapter/facade/internal/tests.rs`
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/sdtm_project/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`
- Modify: `apps/server/aegis-server/src/transport/http/router.rs` (the test only)

**Interfaces:**
- Consumes: the crate's new aggregates and views from Task 3; the `apis` value types from Task 1.
- Produces:
  - `apis::sdtm_project::SdtmProjectDomainView { id, sdtm_project_version_id, domain_code, category, descriptions, key_variables, created_at, updated_at }`
  - `apis::sdtm_project::SdtmProjectVariableView { id, sdtm_project_domain_id, variable_code, variable_controlled, variable_type, variable_core, variable_role, variable_sequence, descriptions, conversion_rules, created_at, updated_at }`
  - `apis::sdtm_project::CreateSdtmProjectDomainRequest { sdtm_project_version_id, domain_code, category, descriptions, key_variables }`
  - `apis::sdtm_project::UpdateSdtmProjectDomainRequest { id, domain_code, category, descriptions, key_variables }`
  - `apis::sdtm_project::CreateSdtmProjectVariableRequest { sdtm_project_domain_id, variable_code, variable_controlled, variable_type, variable_core, variable_role, variable_sequence, descriptions, conversion_rules }`
  - `apis::sdtm_project::UpdateSdtmProjectVariableRequest { id, variable_code, variable_controlled, variable_type, variable_core, variable_role, variable_sequence, descriptions, conversion_rules }`
  - The HTTP surface: `POST/PUT /api/sdtm-project/domains[/{id}]` carrying `sdtmProjectVersionId`, `domainCode`, `category`, `descriptions`, `keyVariables`; `POST/PUT /api/sdtm-project/variables[/{id}]` carrying `sdtmProjectDomainId`, `variableCode`, `variableControlled`, `variableType`, `variableCore`, `variableRole`, `variableSequence`, `descriptions`, `conversionRules`.

- [ ] **Step 1: Write the failing test in `lib/crates/apis/tests/public_api.rs`**

Update the sdtm-project section of this file so it constructs the new views and requests field-for-field:

```rust
    assert_view(SdtmProjectDomainView {
        id: 1,
        sdtm_project_version_id: 1,
        domain_code: "AE".into(),
        category: DomainCategory::Events,
        descriptions: Vec::new(),
        key_variables: vec!["AETERM".into()],
        created_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        updated_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
    });
    assert_view(SdtmProjectVariableView {
        id: 1,
        sdtm_project_domain_id: 1,
        variable_code: "AETERM".into(),
        variable_controlled: None,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: Some(SdtmRole::Topic),
        variable_sequence: 1,
        descriptions: Vec::new(),
        conversion_rules: vec![ConversionRule {
            source: VariableSource::Protocol,
            rule: "AESEV".into(),
        }],
        created_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        updated_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
    });
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apis sdtm_project`
Expected: COMPILE ERROR — `SdtmProjectDomainView` has no field named `sdtm_project_version_id`, and `SdtmProjectVariableView` has no field named `conversion_rules`.

- [ ] **Step 3: Rewrite the `apis` views and requests**

In `lib/crates/apis/src/sdtm_project.rs`:

```rust
/// Safe projection of an `SdtmProjectDomain` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectDomainView {
    pub id: i64,
    pub sdtm_project_version_id: i64,
    pub domain_code: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub key_variables: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of an `SdtmProjectVariable` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVariableView {
    pub id: i64,
    pub sdtm_project_domain_id: i64,
    pub variable_code: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub conversion_rules: Vec<ConversionRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

and the four request DTOs:

```rust
/// Input DTO for [`SdtmProjectService::create_domain`].
#[derive(Debug, Clone)]
pub struct CreateSdtmProjectDomainRequest {
    pub sdtm_project_version_id: i64,
    pub domain_code: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub key_variables: Vec<String>,
}

/// Input DTO for [`SdtmProjectService::update_domain`]. `key_variables`
/// preserves the caller's order; `Some(vec![])` clears the column.
#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmProjectDomainRequest {
    pub id: i64,
    pub domain_code: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
    pub key_variables: Option<Vec<String>>,
}

/// Input DTO for [`SdtmProjectService::create_variable`].
#[derive(Debug, Clone)]
pub struct CreateSdtmProjectVariableRequest {
    pub sdtm_project_domain_id: i64,
    pub variable_code: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub conversion_rules: Vec<ConversionRule>,
}

/// Input DTO for [`SdtmProjectService::update_variable`].
///
/// `variable_controlled` and `variable_role` use `Option<Option<T>>` so
/// the caller can distinguish "don't change" (outer `None`) from
/// "clear the field" (outer `Some(None)`).
#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmProjectVariableRequest {
    pub id: i64,
    pub variable_code: Option<String>,
    pub variable_controlled: Option<Option<SdtmVariableControlled>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
    pub conversion_rules: Option<Vec<ConversionRule>>,
}
```

- [ ] **Step 4: Map the new fields in the facade**

In `lib/crates/sdtm-project/src/adapter/facade/internal/service.rs`:

- extend the import lists: add `ConversionRule`, `FromCrf`, `SdtmVariableControlled` and `VariableSource` to the `crate::domain::{…}` import, and `ConversionRule as ApiConversionRule`, `FromCrf as ApiFromCrf`, `SdtmVariableControlled as ApiSdtmVariableControlled`, `VariableSource as ApiVariableSource` to the `apis::sdtm_project::{…}` import;

- add mappers mirroring the existing description mappers:
  ```rust
  fn variable_controlled_from_api(v: Option<ApiSdtmVariableControlled>) -> Option<SdtmVariableControlled> {
      v.map(|c| SdtmVariableControlled {
          code: c.code,
          format: c.format,
      })
  }

  fn variable_controlled_to_api(v: Option<SdtmVariableControlled>) -> Option<ApiSdtmVariableControlled> {
      v.map(|c| ApiSdtmVariableControlled {
          code: c.code,
          format: c.format,
      })
  }

  fn conversion_rules_from_api(v: Vec<ApiConversionRule>) -> Vec<ConversionRule> {
      v.into_iter()
          .map(|r| ConversionRule {
              source: variable_source_from_api(r.source),
              rule: r.rule,
          })
          .collect()
  }

  fn conversion_rules_to_api(v: Vec<ConversionRule>) -> Vec<ApiConversionRule> {
      v.into_iter()
          .map(|r| ApiConversionRule {
              source: variable_source_to_api(r.source),
              rule: r.rule,
          })
          .collect()
  }

  fn variable_source_from_api(s: ApiVariableSource) -> VariableSource {
      match s {
          ApiVariableSource::Protocol => VariableSource::Protocol,
          ApiVariableSource::Derived => VariableSource::Derived,
          ApiVariableSource::Assigned(f) => VariableSource::Assigned(f.map(|f| FromCrf { form_code: f.form_code })),
          ApiVariableSource::Crf(f) => VariableSource::Crf(f.map(|f| FromCrf { form_code: f.form_code })),
          ApiVariableSource::Edt => VariableSource::Edt,
      }
  }

  fn variable_source_to_api(s: VariableSource) -> ApiVariableSource {
      match s {
          VariableSource::Protocol => ApiVariableSource::Protocol,
          VariableSource::Derived => ApiVariableSource::Derived,
          VariableSource::Assigned(f) => ApiVariableSource::Assigned(f.map(|f| ApiFromCrf { form_code: f.form_code })),
          VariableSource::Crf(f) => ApiVariableSource::Crf(f.map(|f| ApiFromCrf { form_code: f.form_code })),
          VariableSource::Edt => ApiVariableSource::Edt,
      }
  }
  ```
- `domain_view_from_internal` gains `category: from_internal_category(view.category)` and `descriptions: domain_descriptions_to_api(view.descriptions)`, and renames `version_id` to `sdtm_project_version_id`.
- `variable_view_from_internal` gains the controlled/type/core/role/descriptions/conversion_rules fields and renames `domain_id` to `sdtm_project_domain_id`.
- `create_domain` / `update_domain` / `create_variable` / `update_variable` pass the new request fields through, mapping the enums and the descriptions.
- the `From<UsecaseError> for SdtmProjectApiError` impl gains:
  ```rust
                DomainError::KeyVariableNotFound { variable_code } => {
                    SdtmProjectApiError::KeyVariableNotFound { variable_code }
                }
                DomainError::VariableReferencedByKeyVariables { variable_code } => {
                    SdtmProjectApiError::VariableReferencedByKeyVariables { variable_code }
                }
  ```
  and `DomainError::EmptyConversionRule` joins the catch-all list of validation-only variants that map to `SdtmProjectApiError::Repository`.

- [ ] **Step 5: Update the facade's tests**

In `lib/crates/sdtm-project/src/adapter/facade/internal/tests.rs`: update every command and view literal to the new shapes, and add a test that a `conversion_rules` list survives the round trip through the facade (source spelling and rule text intact).

- [ ] **Step 6: Write the failing test in the server's router tests**

In `apps/server/aegis-server/src/transport/http/router.rs`, inside `mod tests`, add a new test after `openapi_json_advertises_the_sdtm_project_namespace`:

```rust
    /// The sdtm-project namespace advertises the standalone model: the
    /// domain and variable schemas carry the metadata the aggregates
    /// now hold.
    #[tokio::test]
    async fn openapi_json_advertises_the_standalone_sdtm_project_model() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let schemas = &doc["components"]["schemas"];
        for (schema, fields) in [
            (
                "SdtmProjectDomainViewResponse",
                &[
                    "sdtmProjectVersionId",
                    "domainCode",
                    "category",
                    "descriptions",
                    "keyVariables",
                ][..],
            ),
            (
                "SdtmProjectVariableViewResponse",
                &[
                    "sdtmProjectDomainId",
                    "variableCode",
                    "variableControlled",
                    "variableType",
                    "variableCore",
                    "variableRole",
                    "variableSequence",
                    "descriptions",
                    "conversionRules",
                ][..],
            ),
        ] {
            let properties = &schemas[schema]["properties"];
            for field in fields {
                assert!(
                    properties.get(*field).is_some(),
                    "{schema} must carry `{field}`: {properties}",
                );
            }
        }
    }
```

- [ ] **Step 7: Run the test to verify it fails**

Run: `cargo test -p aegis-server --lib transport::http::router::tests::openapi_json_advertises_the_standalone_sdtm_project_model`
Expected: FAIL — the server's DTOs still carry the old field names, so `SdtmProjectDomainViewResponse must carry 'category'` (or a compile error in the DTO layer).

- [ ] **Step 8: Rewrite the server's sdtm-project DTOs**

In `apps/server/aegis-server/src/transport/http/dto.rs`:

- add the new value DTOs:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "camelCase")]
  pub struct SdtmProjectVariableControlled {
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub code: Option<Vec<String>>,
      #[serde(default, skip_serializing_if = "Option::is_none")]
      pub format: Option<Vec<String>>,
  }

  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "camelCase")]
  pub struct SdtmProjectFromCrf {
      pub form_code: String,
  }

  /// Adjacently tagged exactly like the crate's `VariableSource`, so
  /// the bytes on the wire are identical to the ones the crate
  /// serialises: a unit variant omits `from`, a payload variant
  /// carries it (as `null` when the form is absent).
  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  #[serde(tag = "source", content = "from")]
  pub enum SdtmProjectVariableSource {
      #[serde(rename = "Protocol")]
      Protocol,
      #[serde(rename = "Derived")]
      Derived,
      #[serde(rename = "Assigned")]
      Assigned(Option<SdtmProjectFromCrf>),
      #[serde(rename = "CRF")]
      Crf(Option<SdtmProjectFromCrf>),
      #[serde(rename = "eDT")]
      Edt,
  }

  #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
  pub struct SdtmProjectConversionRule {
      #[serde(flatten)]
      pub source: SdtmProjectVariableSource,
      pub rule: String,
  }
  ```
- rewrite the request DTOs:
  ```rust
  #[derive(Debug, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "camelCase")]
  pub struct CreateSdtmProjectDomainRequest {
      pub sdtm_project_version_id: i64,
      pub domain_code: String,
      pub category: SdtmProjectDomainCategory,
      pub descriptions: Vec<SdtmProjectDomainDescription>,
      pub key_variables: Vec<String>,
  }

  #[derive(Debug, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "camelCase")]
  pub struct UpdateSdtmProjectDomainRequest {
      pub domain_code: Option<String>,
      pub category: Option<SdtmProjectDomainCategory>,
      pub descriptions: Option<Vec<SdtmProjectDomainDescription>>,
      pub key_variables: Option<Vec<String>>,
  }

  #[derive(Debug, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "camelCase")]
  pub struct CreateSdtmProjectVariableRequest {
      pub sdtm_project_domain_id: i64,
      pub variable_code: String,
      pub variable_controlled: Option<SdtmProjectVariableControlled>,
      pub variable_type: SdtmProjectVariableType,
      pub variable_core: SdtmProjectVariableCore,
      pub variable_role: Option<SdtmProjectRole>,
      pub variable_sequence: i64,
      pub descriptions: Vec<SdtmProjectVariableDescription>,
      pub conversion_rules: Vec<SdtmProjectConversionRule>,
  }

  #[derive(Debug, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "camelCase")]
  pub struct UpdateSdtmProjectVariableRequest {
      pub variable_code: Option<String>,
      /// Three-state: absent = unchanged, `null` = clear, value =
      /// replace. See [`double_option`].
      #[serde(default, deserialize_with = "double_option")]
      pub variable_controlled: Option<Option<SdtmProjectVariableControlled>>,
      pub variable_type: Option<SdtmProjectVariableType>,
      pub variable_core: Option<SdtmProjectVariableCore>,
      /// Three-state: absent = unchanged, `null` = clear, value =
      /// replace. See [`double_option`].
      #[serde(default, deserialize_with = "double_option")]
      pub variable_role: Option<Option<SdtmProjectRole>>,
      pub variable_sequence: Option<i64>,
      pub descriptions: Option<Vec<SdtmProjectVariableDescription>>,
      pub conversion_rules: Option<Vec<SdtmProjectConversionRule>>,
  }
  ```
- rewrite the two view-response DTOs:
  ```rust
  #[derive(Debug, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "camelCase")]
  pub struct SdtmProjectDomainViewResponse {
      pub id: i64,
      pub sdtm_project_version_id: i64,
      pub domain_code: String,
      pub category: SdtmProjectDomainCategory,
      pub descriptions: Vec<SdtmProjectDomainDescription>,
      /// The variable codes, in the order the caller supplied them.
      pub key_variables: Vec<String>,
      pub created_at: chrono::DateTime<chrono::Utc>,
      pub updated_at: chrono::DateTime<chrono::Utc>,
  }

  #[derive(Debug, Serialize, Deserialize, ToSchema)]
  #[serde(rename_all = "camelCase")]
  pub struct SdtmProjectVariableViewResponse {
      pub id: i64,
      pub sdtm_project_domain_id: i64,
      pub variable_code: String,
      pub variable_controlled: Option<SdtmProjectVariableControlled>,
      pub variable_type: SdtmProjectVariableType,
      pub variable_core: SdtmProjectVariableCore,
      pub variable_role: Option<SdtmProjectRole>,
      pub variable_sequence: i64,
      pub descriptions: Vec<SdtmProjectVariableDescription>,
      pub conversion_rules: Vec<SdtmProjectConversionRule>,
      pub created_at: chrono::DateTime<chrono::Utc>,
      pub updated_at: chrono::DateTime<chrono::Utc>,
  }
  ```
- update the two `From<apis::sdtm_project::…>` impls to carry the new fields, mapping the enums and the controlled-terms structure;
- add a unit test in `dto.rs` asserting the exact JSON of a conversion rule:
  ```rust
  #[test]
  fn conversion_rule_serializes_with_the_agreed_source_spelling() {
      let rule = SdtmProjectConversionRule {
          source: SdtmProjectVariableSource::Crf(Some(SdtmProjectFromCrf {
              form_code: "AE".into(),
          })),
          rule: "AESEV".into(),
      };
      let json = serde_json::to_value(&rule).unwrap();
      assert_eq!(json["source"], "CRF");
      assert_eq!(json["from"]["formCode"], "AE");
  }
  ```

- [ ] **Step 9: Update the server's handlers**

In `apps/server/aegis-server/src/transport/http/sdtm_project/handlers.rs`:
- `create_domain` passes `sdtm_project_version_id: req.sdtm_project_version_id`, `category`, `descriptions`;
- `update_domain` passes `category` and `descriptions`;
- `create_variable` passes `sdtm_project_domain_id: req.sdtm_project_domain_id`, `variable_controlled`, `variable_type`, `variable_core`, `variable_role`, `descriptions`, `conversion_rules`;
- `update_variable` passes the same, mapping `variable_controlled` and `conversion_rules` into the `apis` types;
- add the new documented responses: `400` on `create_domain` / `update_domain` / `create_variable` / `update_variable` (a key variable that names no variable) and `409` on `update_variable` / `delete_variable` (a variable a domain names as a key).

- [ ] **Step 10: Register the new schemas in the OpenAPI document**

In `apps/server/aegis-server/src/transport/http/openapi.rs`, add to the `components(schemas(…))` list, in the sdtm-project block:

```
        dto::SdtmProjectVariableControlled,
        dto::SdtmProjectFromCrf,
        dto::SdtmProjectVariableSource,
        dto::SdtmProjectConversionRule,
```

- [ ] **Step 11: Run the whole workspace**

Run: `cargo build -p apis -p sdtm-project -p aegis-server && cargo test -p apis -p sdtm-project -p aegis-server && cargo clippy -p apis -p sdtm-project -p aegis-server --all-targets --all-features -- -D warnings && cargo fmt --all -- --check`
Expected: all green — the `apis` views, the facade and the server now agree.

- [ ] **Step 12: Commit**

```bash
git add lib/crates/apis lib/crates/sdtm-project/src/adapter/facade apps/server/aegis-server
git commit -m "$(cat <<'EOF'
feat(apis): expose sdtm project domain and variable metadata

The apis port now carries what the standalone model holds: a domain's
category and descriptions, a variable's type, core, role, controlled
terminology, descriptions and conversion rules. The two foreign-key
fields are renamed to sdtm_project_version_id / sdtm_project_domain_id
so the target is explicit on the wire.

The server's DTOs, handlers and OpenAPI schemas follow in the same
commit: a change to an apis view breaks the server's From impls and
its handlers, so the two halves cannot land apart.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Documentation

**Files:**
- Modify: `lib/crates/sdtm-project/README.md`

**Interfaces:**
- Consumes: the final shape from Task 4.
- Produces: nothing but documentation.

- [ ] **Step 1: Update the crate README**

In `lib/crates/sdtm-project/README.md`:
- the layout tree: replace `sdtm_project_domain_override.rs` and `sdtm_project_variable_override.rs` with `conversion_rule.rs`, and drop the "the five repository ports" / "ten Create/Update command DTOs" / "five view DTOs" counts (it is now three ports, six command DTOs, three views);
- the data-model table: three rows instead of five — `SdtmProjectVersion` / `sdtm_project_versions` `(project_code, version_name)`, `SdtmProjectDomain` / `sdtm_project_domains` `(sdtm_project_version_id, domain_code)`, `SdtmProjectVariable` / `sdtm_project_variables` `(sdtm_project_domain_id, variable_code)`;
- add a short section describing the conversion rules and the key-variable rule, including the "a domain is created with empty key_variables" consequence.

- [ ] **Step 2: Verify the whole workspace once more**

Run: `cargo build -p apis -p sdtm-project -p aegis-server && cargo test -p apis -p sdtm-project -p aegis-server && cargo fmt --all -- --check`
Expected: all green.

- [ ] **Step 3: Commit**

```bash
git add lib/crates/sdtm-project/README.md
git commit -m "$(cat <<'EOF'
docs(sdtm-project): update the README for the standalone model

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
```

---

## Task dependency map

```
Task 1 (apis value types + errors)
   ↓
Task 2 (remove the override surface)
   ↓
Task 3 (the crate's standalone model)   ← internally atomic
   ↓
Task 4 (apis views + facade + server wire layer)
   ↓
Task 5 (docs)
```

Every task leaves `cargo build --workspace` green at its boundary. Task 3 does not compile partway through — plan it in one sitting.

## Out of scope

- `aegis-desktop` — no TS, no Rust shell changes.
- Validating `FromCrf.form_code` against the `crf` crate.
- Parsing or evaluating `ConversionRule.rule`.
- An "import from standard" action that copies a `domain-model` domain into a project domain.
