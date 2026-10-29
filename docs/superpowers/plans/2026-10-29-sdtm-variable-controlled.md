# SDTM Variable Controlled-Terminology Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace `SdtmVariable.variable_controlled: Option<String>` with `Option<SdtmVariableControlled>` — a structured value carrying an optional `code` list and an optional `format` list — through the domain model, the API port, the HTTP wire, and the database column.

**Architecture:** The value is modelled as a small struct at every layer boundary, exactly as `SdtmVariableDescription` already is: a domain type in `domain-model`, a mirror in `apis`, and a wire DTO in `aegis-server`, each converted with `From` impls. The database column changes from `TEXT` to `JSONB` via a new migration, and the postgres adapter follows the existing `descriptions` JSONB pattern for decode and bind.

**Tech Stack:** Rust 2024, Cargo workspace, SQLx 0.9 (postgres + `json` feature), serde / serde_json, utoipa (OpenAPI schemas), axum.

**Spec:** `docs/superpowers/specs/2026-10-29-sdtm-variable-controlled-design.md` — read it before starting; this plan implements it and does not restate its reasoning.

## Global Constraints

- `SdtmVariableControlled` lives in `lib/crates/domain-model/src/domain/variable_type.rs` and is re-exported from `lib/crates/domain-model/src/domain.rs`.
- `lib/crates/apis` declares ports and DTOs only — it mirrors domain types, it does not import them.
- The desktop crate `apps/desktop/aegis-desktop` is **out of scope** — do not modify it.
- The server accepts **only** the object shape for `variableControlled`. A JSON string is a deserialization error; do not add an untagged/legacy variant.
- The migration drops existing `variable_controlled` TEXT values to `NULL`. Do not backfill them.
- The migration and the deploy are one operation — the old and new servers are not compatible with each other's column type in either order. See the spec's "Rollout" note.
- `UpdateSdtmVariableRequest.variable_controlled` keeps its plain `Option<Option<T>>` derive. Do **not** adopt the `double_option` helper: that would change today's behaviour (making "clear" reachable over HTTP) rather than preserve it, and would raise the same question for `variable_role` on the same DTO. The current behaviour is pinned by a test in Task 4.
- The JSON shape omits absent lists (`{"code": ["C66731"]}`, never `{"code": [...], "format": null}`), on the wire and in the database alike.
- Every task ends green for its own crate: `cargo test -p <crate>` and `cargo clippy -p <crate> --all-targets --all-features -- -D warnings`.
- Live-DB tests are `#[ignore]`-gated and drop tables on every run. Point them at the test database, never at `aegis`:
  `AEGIS_DOMAIN_MODEL_DATABASE_URL=<url of aegis_sdtm_project_test> cargo test -p domain-model -- --ignored --test-threads=1`

## Review Focus

These are the input classes the spec implies but does not spell out. Each is pinned by a test in the task that owns the code.

1. **A JSONB column holding JSON `null`** (not SQL `NULL`) must decode as *absent*, not as an error. Only reachable by hand-editing the DB, but a decode error there would break every read of that row.
2. **A JSONB column holding a malformed document** (e.g. `{"code": "C66731"}` — a string where an array belongs) must surface as `DomainError::Repository`, not a panic and not a silent `None`.
3. **A list that is present but empty** (`{"code": []}`) must round-trip as *present with an empty list*, not collapse to absent. `Some(vec![])` and `None` are different values.
4. **An explicit `null` for a list** (`{"code": null}`) must read back as absent — same as a missing key.
5. **A legacy string in a request body** (`{"variableControlled": "AETERM"}`) must be rejected by the server's deserializer, not coerced into `{"code": ["AETERM"]}`.

---

### Task 1: The `SdtmVariableControlled` value type

**Files:**
- Modify: `lib/crates/domain-model/src/domain/variable_type.rs` (append after the `SdtmRole` `TryFrom` impl, i.e. after line 121)
- Modify: `lib/crates/domain-model/src/domain.rs:23`
- Modify: `lib/crates/domain-model/src/lib.rs:22-26`
- Test: `lib/crates/domain-model/src/domain/tests.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `domain_model::SdtmVariableControlled` — a struct with `pub code: Option<Vec<String>>` and `pub format: Option<Vec<String>>`, deriving `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`. Also reachable as `domain_model::domain::SdtmVariableControlled`.

This task adds the type without wiring it into any field, so nothing else in the workspace changes. The field migration is Task 3.

- [ ] **Step 1: Write the failing tests**

Append to `lib/crates/domain-model/src/domain/tests.rs`. Extend the existing `use super::{…}` import list to include `SdtmVariableControlled`.

```rust
// ---- SdtmVariableControlled -----------------------------------------------

#[test]
fn variable_controlled_serializes_only_the_lists_that_are_set() {
    let vc = SdtmVariableControlled {
        code: Some(vec!["C66731".into()]),
        format: None,
    };
    let json = serde_json::to_value(&vc).unwrap();
    assert_eq!(json, serde_json::json!({ "code": ["C66731"] }));
}

#[test]
fn variable_controlled_reads_absent_lists_as_none() {
    let vc: SdtmVariableControlled = serde_json::from_value(serde_json::json!({})).unwrap();
    assert_eq!(vc.code, None);
    assert_eq!(vc.format, None);
}

#[test]
fn variable_controlled_reads_explicit_null_lists_as_none() {
    let vc: SdtmVariableControlled =
        serde_json::from_value(serde_json::json!({ "code": null })).unwrap();
    assert_eq!(vc.code, None);
    assert_eq!(vc.format, None);
}

#[test]
fn variable_controlled_round_trips_both_lists() {
    let vc = SdtmVariableControlled {
        code: Some(vec!["C66731".into()]),
        format: Some(vec!["Codelist".into()]),
    };
    let json = serde_json::to_value(&vc).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "code": ["C66731"], "format": ["Codelist"] })
    );
    let back: SdtmVariableControlled = serde_json::from_value(json).unwrap();
    assert_eq!(back, vc);
}

#[test]
fn variable_controlled_keeps_present_but_empty_lists() {
    let vc = SdtmVariableControlled {
        code: Some(vec![]),
        format: None,
    };
    let json = serde_json::to_value(&vc).unwrap();
    assert_eq!(json, serde_json::json!({ "code": [] }));
    let back: SdtmVariableControlled = serde_json::from_value(json).unwrap();
    assert_eq!(back.code, Some(vec![]));
    assert_eq!(back.format, None);
}
```

This covers Review Focus items 3 and 4.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p domain-model --lib domain::tests`

Expected: compile error — `no SdtmVariableControlled in domain` (E0432). The type does not exist yet.

- [ ] **Step 3: Add the type**

Append to `lib/crates/domain-model/src/domain/variable_type.rs`:

```rust
/// Controlled-terminology metadata attached to an SDTM variable.
///
/// Carried on the `SdtmVariable` aggregate and persisted as a single
/// JSONB column on `sdtm_variables`. Both lists are optional and are
/// omitted from the JSON when unset, so the stored shape is e.g.
/// `{"code": ["C66731"]}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableControlled {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Vec<String>>,
}
```

The file already imports `serde::{Deserialize, Serialize}` (line 3) — no import change needed.

- [ ] **Step 4: Re-export the type**

In `lib/crates/domain-model/src/domain.rs`, change line 23 to:

```rust
pub use variable_type::{SdtmRole, SdtmVariableControlled, SdtmVariableCore, SdtmVariableType};
```

In `lib/crates/domain-model/src/lib.rs`, add `SdtmVariableControlled` to the `pub use domain::{…}` list (lines 22-26), alphabetically between `SdtmVariableCore` and `SdtmVariableDescription`:

```rust
    SdtmVariableCore, SdtmVariableControlled, SdtmVariableDescription,
```

(Re-wrap the whole `pub use domain::{…}` block with `cargo fmt` afterwards.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p domain-model --lib domain::tests`

Expected: PASS — 5 new tests plus the pre-existing domain tests.

- [ ] **Step 6: Verify the crate is still clean**

Run: `cargo clippy -p domain-model --all-targets --all-features -- -D warnings`

Expected: no warnings. The new type is publicly reachable, so it is not dead code.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/domain-model/src/domain/variable_type.rs lib/crates/domain-model/src/domain.rs lib/crates/domain-model/src/lib.rs lib/crates/domain-model/src/domain/tests.rs
git commit -m "feat(domain-model): add the SdtmVariableControlled value type"
```

---

### Task 2: The `apis` port contract

**Files:**
- Modify: `lib/crates/apis/src/domain_model.rs`
- Test: `lib/crates/apis/tests/public_api.rs`

**Interfaces:**
- Consumes: nothing (the `apis` crate does not depend on `domain-model`).
- Produces: `apis::domain_model::SdtmVariableControlled` — a struct with `pub code: Option<Vec<String>>` and `pub format: Option<Vec<String>>`, deriving `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`. `SdtmVariableView.variable_controlled`, `CreateSdtmVariableRequest.variable_controlled` become `Option<SdtmVariableControlled>`; `UpdateSdtmVariableRequest.variable_controlled` becomes `Option<Option<SdtmVariableControlled>>`.

**Note:** this task deliberately leaves `domain-model` non-compiling (its in-memory facade converts between the `apis` view and the domain view). Task 3 resolves that. Scope this task's verification to the `apis` crate.

- [ ] **Step 1: Write the failing test**

Append to `lib/crates/apis/tests/public_api.rs`. The file has no `apis::domain_model` section yet — add one:

```rust
// ---- domain_model --------------------------------------------------------

use apis::domain_model::{
    CreateSdtmVariableRequest, SdtmVariableControlled, SdtmVariableView, UpdateSdtmVariableRequest,
};

/// Every public type in `apis::domain_model` is nameable from the test.
#[test]
fn domain_model_public_types_are_nameable() {
    fn assert_controlled(_: SdtmVariableControlled) {}
    fn assert_view(_: SdtmVariableView) {}
    fn assert_create(_: CreateSdtmVariableRequest) {}
    fn assert_update(_: UpdateSdtmVariableRequest) {}

    assert_controlled(SdtmVariableControlled {
        code: Some(vec!["C66731".into()]),
        format: None,
    });
    assert_view(SdtmVariableView {
        id: 1,
        domain_id: 2,
        name: "AESEV".into(),
        variable_controlled: Some(SdtmVariableControlled {
            code: Some(vec!["C66731".into()]),
            format: Some(vec!["Codelist".into()]),
        }),
        variable_type: apis::domain_model::SdtmVariableType::Character,
        variable_core: apis::domain_model::SdtmVariableCore::Req,
        variable_role: Some(apis::domain_model::SdtmRole::Topic),
        variable_sequence: 11,
        descriptions: vec![],
        created_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        updated_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
    });
    assert_create(CreateSdtmVariableRequest {
        domain_id: 2,
        name: "AESEV".into(),
        variable_controlled: None,
        variable_type: apis::domain_model::SdtmVariableType::Character,
        variable_core: apis::domain_model::SdtmVariableCore::Req,
        variable_role: None,
        variable_sequence: 11,
        descriptions: vec![],
    });
    assert_update(UpdateSdtmVariableRequest::default());
}

/// The wire shape of the controlled-terminology value omits the lists
/// that are not set, so a client sees `{"code": [...]}` rather than
/// `{"code": [...], "format": null}`.
#[test]
fn domain_model_variable_controlled_serialises_only_the_lists_that_are_set() {
    let vc = SdtmVariableControlled {
        code: Some(vec!["C66731".into()]),
        format: None,
    };
    let json = serde_json::to_value(&vc).unwrap();
    assert_eq!(json, serde_json::json!({ "code": ["C66731"] }));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apis`

Expected: compile error — `unresolved import apis::domain_model::SdtmVariableControlled`, and the `SdtmVariableView` literal mismatches on `variable_controlled` (expected `Option<String>`, found `Option<SdtmVariableControlled>`).

- [ ] **Step 3: Add the mirror struct**

In `lib/crates/apis/src/domain_model.rs`, add after `SdtmVariableDescriptionDetail` (line 151):

```rust
/// Controlled-terminology metadata attached to an SDTM variable.
///
/// Mirrors `domain_model::SdtmVariableControlled`. The two structs
/// are kept in sync layer by layer — the in-memory facade in
/// `domain-model` converts between them field by field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableControlled {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Vec<String>>,
}
```

- [ ] **Step 4: Change the three DTO field types**

In `lib/crates/apis/src/domain_model.rs`:

- line 183, `SdtmVariableView`:
  `pub variable_controlled: Option<SdtmVariableControlled>,`
- line 259, `CreateSdtmVariableRequest`:
  `pub variable_controlled: Option<SdtmVariableControlled>,`
- line 279, `UpdateSdtmVariableRequest`:
  `pub variable_controlled: Option<Option<SdtmVariableControlled>>,`

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p apis`

Expected: PASS — the two new tests plus the pre-existing `public_api` tests.

- [ ] **Step 6: Verify the crate is clean**

Run: `cargo clippy -p apis --all-targets --all-features -- -D warnings`

Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/apis/src/domain_model.rs lib/crates/apis/tests/public_api.rs
git commit -m "feat(apis): model variable_controlled as a structured value"
```

---

### Task 3: Migrate the `domain-model` crate

**Files:**
- Modify: `lib/crates/domain-model/src/domain/sdtm_variable.rs` (lines 30, 65, 97, 127, 146)
- Modify: `lib/crates/domain-model/src/usecase/commands.rs` (lines 40, 53)
- Modify: `lib/crates/domain-model/src/usecase/views.rs` (line 57)
- Modify: `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_variable_repo.rs`
- Modify: `lib/crates/domain-model/src/adapter/facade/in_memory/service.rs`
- Create: `lib/crates/domain-model/migrations/0004_alter_sdtm_variables_variable_controlled_to_jsonb.sql`
- Test: `lib/crates/domain-model/src/usecase/tests.rs`
- Test: `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_variable_repo.rs` (the existing `#[cfg(test)] mod tests`)
- Test: `lib/crates/domain-model/tests/integration_persistence.rs`

**Interfaces:**
- Consumes: `domain_model::SdtmVariableControlled` (Task 1), `apis::domain_model::SdtmVariableControlled` (Task 2).
- Produces: `SdtmVariable.variable_controlled: Option<SdtmVariableControlled>`; `SdtmVariableNew.variable_controlled: Option<SdtmVariableControlled>`; `SdtmVariableUpdate.variable_controlled: Option<Option<SdtmVariableControlled>>`; `CreateSdtmVariable.variable_controlled: Option<SdtmVariableControlled>`; `UpdateSdtmVariable.variable_controlled: Option<Option<SdtmVariableControlled>>`; `usecase::views::SdtmVariableView.variable_controlled: Option<SdtmVariableControlled>`. The `sdtm_variables.variable_controlled` column is `JSONB` after migration `0004`.

**Note:** this task changes one crate but touches six files in it. They cannot be split: the aggregate's type change breaks the postgres adapter's `Encode` bound, and the facade's `apis` ↔ domain conversion breaks as soon as either side moves. The crate compiles only at the end of this task.

- [ ] **Step 1: Write the failing usecase test**

Append to `lib/crates/domain-model/src/usecase/tests.rs`. Model it on the existing `variable_crud_round_trips` test (around line 380) and the `build_usecase()` helper (line 249). Add `SdtmVariableControlled` to the `use crate::domain::{…}` import list at the top of the file.

```rust
#[tokio::test]
async fn variable_controlled_three_state_update_is_honoured() {
    let uc = build_usecase();
    let v = uc.create_version(CreateSdtmVersion { name: "2024-09-27".into() }).await.unwrap();
    let d = uc
        .create_domain(CreateSdtmDomain {
            version_id: v.id,
            name: "AE".into(),
            category: DomainCategory::Events,
            descriptions: vec![],
        })
        .await
        .unwrap();

    let controlled = SdtmVariableControlled {
        code: Some(vec!["C66731".into()]),
        format: Some(vec!["Codelist".into()]),
    };
    let var = uc
        .create_variable(CreateSdtmVariable {
            domain_id: d.id,
            name: "AESEV".into(),
            variable_controlled: Some(controlled.clone()),
            variable_type: SdtmVariableType::Character,
            variable_core: SdtmVariableCore::Req,
            variable_role: Some(SdtmRole::Topic),
            variable_sequence: 1,
            descriptions: vec![],
        })
        .await
        .unwrap();
    assert_eq!(var.variable_controlled, Some(controlled.clone()));

    // Outer None: leave the field alone.
    let unchanged = uc
        .update_variable(UpdateSdtmVariable {
            id: var.id,
            name: None,
            variable_controlled: None,
            variable_type: None,
            variable_core: None,
            variable_role: None,
            variable_sequence: None,
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(unchanged.variable_controlled, Some(controlled.clone()));

    // Outer Some(inner Some): replace the field.
    let replacement = SdtmVariableControlled {
        code: Some(vec!["C66988".into()]),
        format: None,
    };
    let replaced = uc
        .update_variable(UpdateSdtmVariable {
            id: var.id,
            name: None,
            variable_controlled: Some(Some(replacement.clone())),
            variable_type: None,
            variable_core: None,
            variable_role: None,
            variable_sequence: None,
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(replaced.variable_controlled, Some(replacement));

    // Outer Some(inner None): clear the field.
    let cleared = uc
        .update_variable(UpdateSdtmVariable {
            id: var.id,
            name: None,
            variable_controlled: Some(None),
            variable_type: None,
            variable_core: None,
            variable_role: None,
            variable_sequence: None,
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(cleared.variable_controlled, None);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p domain-model --lib usecase::tests`

Expected: compile error — `expected Option<SdtmVariableControlled>, found Option<String>` on `CreateSdtmVariable.variable_controlled` and `UpdateSdtmVariable.variable_controlled`.

- [ ] **Step 3: Change the aggregate and usecase field types**

In `lib/crates/domain-model/src/domain/sdtm_variable.rs`:

- line 30, `SdtmVariable`:
  `pub variable_controlled: Option<SdtmVariableControlled>,`
- line 65, `SdtmVariable::new` parameter:
  `variable_controlled: Option<SdtmVariableControlled>,`
- line 97, `SdtmVariable::for_repository` parameter:
  `variable_controlled: Option<SdtmVariableControlled>,`
- line 127, `SdtmVariableNew`:
  `pub variable_controlled: Option<SdtmVariableControlled>,`
- line 146, `SdtmVariableUpdate`:
  `pub variable_controlled: Option<Option<SdtmVariableControlled>>,`

The file's `use super::variable_type::{SdtmRole, SdtmVariableCore, SdtmVariableType};` (line 5) becomes:

```rust
use super::variable_type::{SdtmRole, SdtmVariableControlled, SdtmVariableCore, SdtmVariableType};
```

In `lib/crates/domain-model/src/usecase/commands.rs`:

- line 40, `CreateSdtmVariable`: `pub variable_controlled: Option<SdtmVariableControlled>,`
- line 53, `UpdateSdtmVariable`: `pub variable_controlled: Option<Option<SdtmVariableControlled>>,`
- extend the `use crate::domain::{…}` import list (lines 1-4) with `SdtmVariableControlled`.

In `lib/crates/domain-model/src/usecase/views.rs`:

- line 57, `SdtmVariableView`: `pub variable_controlled: Option<SdtmVariableControlled>,`
- extend the `use crate::domain::{…}` import list (lines 3-6) with `SdtmVariableControlled`.

- [ ] **Step 4: Run the test again — it still fails, now in the adapter**

Run: `cargo test -p domain-model --lib usecase::tests`

Expected: compile errors in `adapter/persistence/postgres/sdtm_variable_repo.rs` — `Option<SdtmVariableControlled>` does not implement `sqlx::Encode`, and `no method named as_str found for reference &SdtmVariableControlled` at the update bind (line 146).

- [ ] **Step 5: Add the migration**

Create `lib/crates/domain-model/migrations/0004_alter_sdtm_variables_variable_controlled_to_jsonb.sql`:

```sql
-- sdtm_variables.variable_controlled changes from TEXT to JSONB
-- carrying a SdtmVariableControlled object ({"code": [...], "format": [...]}).
-- The previous TEXT value did not record whether it was a codelist or a
-- format, so it cannot be converted and is dropped to NULL.

ALTER TABLE sdtm_variables
    ALTER COLUMN variable_controlled TYPE JSONB
    USING NULL::jsonb;
```

- [ ] **Step 6: Write the failing adapter unit tests**

In `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_variable_repo.rs`, extend the existing `#[cfg(test)] mod tests` (line 242). These cover Review Focus items 1 and 2.

```rust
#[test]
fn variable_controlled_migration_converts_the_column_to_jsonb() {
    let sql = include_str!(
        "../../../../migrations/0004_alter_sdtm_variables_variable_controlled_to_jsonb.sql"
    );
    assert!(sql.contains("ALTER TABLE sdtm_variables"));
    assert!(sql.contains("TYPE JSONB"));
}

#[test]
fn row_with_json_null_variable_controlled_decodes_as_absent() {
    let row = SdtmVariableRow {
        id: 1,
        domain_id: 1,
        name: "AESEV".into(),
        variable_controlled: Some(serde_json::Value::Null),
        variable_type: "Character".into(),
        variable_core: "Req".into(),
        variable_role: Some("Topic".into()),
        variable_sequence: 1,
        descriptions: serde_json::json!([]),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let var = row.into_var().unwrap();
    assert_eq!(var.variable_controlled, None);
}

#[test]
fn row_with_malformed_variable_controlled_is_a_repository_error() {
    let row = SdtmVariableRow {
        id: 1,
        domain_id: 1,
        name: "AESEV".into(),
        variable_controlled: Some(serde_json::json!({ "code": "C66731" })),
        variable_type: "Character".into(),
        variable_core: "Req".into(),
        variable_role: Some("Topic".into()),
        variable_sequence: 1,
        descriptions: serde_json::json!([]),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let err = row.into_var().unwrap_err();
    assert!(matches!(err, DomainError::Repository(_)), "got {err:?}");
}
```

- [ ] **Step 7: Run the adapter tests to verify they fail**

Run: `cargo test -p domain-model --lib adapter::persistence`

Expected: compile errors — the `SdtmVariableRow` literal has the wrong field type (`Option<String>` expected), and `into_var` does not yet handle `Option<JsonValue>`.

- [ ] **Step 8: Migrate the postgres adapter**

In `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_variable_repo.rs`:

(a) The `use crate::domain::{…}` import (lines 6-9) gains `SdtmVariableControlled`:

```rust
use crate::domain::{
    DomainError, SdtmRole, SdtmVariable, SdtmVariableControlled, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableNew, SdtmVariableRepository, SdtmVariableType,
    SdtmVariableUpdate,
};
```

(b) The `SdtmVariableRow` struct field (line 16):

```rust
    variable_controlled: Option<JsonValue>,
```

(c) The decode in `SdtmVariableRow::into_var` — insert before the `descriptions` parse:

```rust
        let variable_controlled: Option<SdtmVariableControlled> = match self.variable_controlled {
            None => None,
            // A JSON `null` in the column is not a valid
            // SdtmVariableControlled, but it means "no controlled
            // terminology" just as SQL NULL does.
            Some(v) if v.is_null() => None,
            Some(v) => Some(
                serde_json::from_value(v).map_err(|e| DomainError::Repository(e.to_string()))?,
            ),
        };
```

and change the `for_repository` call's third-from-last argument from `self.variable_controlled` to `variable_controlled`.

(d) The create bind — in `create`, after the `descriptions_json` binding (line 66-67), add:

```rust
        let variable_controlled_json: Option<JsonValue> = match &input.variable_controlled {
            None => None,
            Some(v) => {
                Some(serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };
```

and replace `.bind(&input.variable_controlled)` (line 84) with `.bind(variable_controlled_json)`.

(e) The update bind — in `update`, replace the `variable_controlled_bound` block (lines 143-148) with:

```rust
        let variable_controlled_bound: Option<JsonValue> = match &input.variable_controlled {
            None => None,
            Some(None) => None,
            Some(Some(v)) => Some(
                serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?,
            ),
        };
        let clear_controlled = input.variable_controlled.is_some();
```

The `ctrl_expr` / `role_expr` fragments (lines 164-174) and the SQL body stay exactly as they are — only the bound type changed. `COALESCE($7, variable_controlled)` and `variable_controlled = $7` both work against a `JSONB` column.

- [ ] **Step 9: Run the adapter tests to verify they pass**

Run: `cargo test -p domain-model --lib adapter::persistence`

Expected: PASS — the three new tests plus the pre-existing migration test.

- [ ] **Step 10: Migrate the in-memory facade**

In `lib/crates/domain-model/src/adapter/facade/in_memory/service.rs`:

(a) Add `SdtmVariableControlled as ApiSdtmVariableControlled` to the `use apis::domain_model::{…}` import list (lines 14-22), and `SdtmVariableControlled` to the `use crate::domain::{…}` import list (lines 27-30).

(b) Add the conversion helpers next to the existing `variable_descriptions_to_api` / `_from_api` pair (after line 214):

```rust
fn variable_controlled_from_api(v: ApiSdtmVariableControlled) -> SdtmVariableControlled {
    SdtmVariableControlled {
        code: v.code,
        format: v.format,
    }
}

fn variable_controlled_to_api(v: SdtmVariableControlled) -> ApiSdtmVariableControlled {
    ApiSdtmVariableControlled {
        code: v.code,
        format: v.format,
    }
}
```

(c) `variable_view_from_internal` (line 244): `variable_controlled: view.variable_controlled.map(variable_controlled_to_api),`

(d) `create_variable` (line 406): `variable_controlled: req.variable_controlled.map(variable_controlled_from_api),`

(e) `update_variable` (line 439): `variable_controlled: req.variable_controlled.map(|inner| inner.map(variable_controlled_from_api)),`

- [ ] **Step 11: Run the usecase test to verify it passes**

Run: `cargo test -p domain-model --lib usecase::tests`

Expected: PASS — the new three-state test plus the pre-existing usecase tests.

- [ ] **Step 12: Update the ignore-gated live-DB tests**

In `lib/crates/domain-model/tests/integration_persistence.rs`, replace every `variable_controlled: Some("AETERM".into())` with:

```rust
                variable_controlled: Some(SdtmVariableControlled {
                    code: Some(vec!["AETERM".into()]),
                    format: None,
                }),
```

and `variable_controlled: Some(Some("AETERMCD".into()))` with:

```rust
            variable_controlled: Some(Some(SdtmVariableControlled {
                code: Some(vec!["AETERMCD".into()]),
                format: None,
            })),
```

Replace the string assertions:

- line 420: `assert_eq!(var.variable_controlled.as_deref(), Some("AETERM"));` →
  ```rust
        assert_eq!(
            var.variable_controlled,
            Some(SdtmVariableControlled {
                code: Some(vec!["AETERM".into()]),
                format: None,
            })
        );
  ```
- line 479: `assert_eq!(updated.variable_controlled.as_deref(), Some("AETERM"));` → the same struct comparison against `updated.variable_controlled`.
- line 533: `assert_eq!(updated.variable_controlled.as_deref(), Some("AETERMCD"));` → the same struct comparison against `updated.variable_controlled`, with `code: Some(vec!["AETERMCD".into()])`.

Add `SdtmVariableControlled` to the file's `use domain_model::{…}` import list.

- [ ] **Step 13: Run the live-DB tests**

Run:

```bash
AEGIS_DOMAIN_MODEL_DATABASE_URL=<url of aegis_sdtm_project_test> \
  cargo test -p domain-model -- --ignored --test-threads=1
```

Expected: PASS — every `#[ignore]` test, including the three three-state variable tests, now round-tripping through the `JSONB` column.

- [ ] **Step 14: Run the full crate test suite**

Run: `cargo test -p domain-model`

Expected: PASS — unit, doc, and public-API tests. The `#[ignore]` live-DB tests are skipped here.

- [ ] **Step 15: Verify the crate is clean**

Run: `cargo clippy -p domain-model --all-targets --all-features -- -D warnings`

Expected: no warnings.

- [ ] **Step 16: Commit**

```bash
git add lib/crates/domain-model
git commit -m "feat(domain-model): store variable_controlled as JSONB"
```

---

### Task 4: The `aegis-server` wire

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/domain_model/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`
- Test: `apps/server/aegis-server/src/transport/http/dto.rs` (the existing `#[cfg(test)] mod tests` at line 2893)

**Interfaces:**
- Consumes: `apis::domain_model::SdtmVariableControlled` (Task 2).
- Produces: `dto::SdtmVariableControlled` — the wire struct with `pub code: Option<Vec<String>>` and `pub format: Option<Vec<String>>`, deriving `Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema`, with `From` impls in both directions against `apis::domain_model::SdtmVariableControlled`.

- [ ] **Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` block in `apps/server/aegis-server/src/transport/http/dto.rs` (line 2893). This covers Review Focus item 5.

```rust
    /// The server accepts only the structured shape for
    /// `variableControlled`. The retired string shape must be a
    /// deserialization error rather than a silently coerced value.
    #[test]
    fn sdtm_variable_controlled_rejects_the_legacy_string_shape() {
        let legacy = r#"{"domainId":1,"name":"AESEV","variableControlled":"C66731","variableType":"Character","variableCore":"Req","variableSequence":1,"descriptions":[]}"#;
        let err = serde_json::from_str::<CreateSdtmVariableRequest>(legacy);
        assert!(
            err.is_err(),
            "the legacy string shape must not deserialize into the object type"
        );
    }

    /// The structured shape deserializes, and both lists are optional.
    #[test]
    fn sdtm_variable_controlled_accepts_the_object_shape() {
        let body = r#"{"domainId":1,"name":"AESEV","variableControlled":{"code":["C66731"]},"variableType":"Character","variableCore":"Req","variableSequence":1,"descriptions":[]}"#;
        let req: CreateSdtmVariableRequest = serde_json::from_str(body).unwrap();
        assert_eq!(
            req.variable_controlled,
            Some(SdtmVariableControlled {
                code: Some(vec!["C66731".into()]),
                format: None,
            })
        );
    }

    /// Over HTTP, an explicit `null` on the update field means "leave
    /// the column alone", not "clear it". A plain `Option<Option<T>>`
    /// derive maps `null` to the outer `None`, so the "clear" state is
    /// unreachable from a client. This is pre-existing behaviour that
    /// this change preserves — the repo's `double_option` helper is the
    /// remedy, and adopting it here would be a separate decision
    /// (it would change behaviour rather than preserve it, and would
    /// raise the same question for `variable_role` on this DTO).
    #[test]
    fn sdtm_variable_update_null_means_unchanged_not_cleared() {
        let null: UpdateSdtmVariableRequest =
            serde_json::from_str(r#"{"variableControlled":null}"#).unwrap();
        assert_eq!(
            null.variable_controlled, None,
            "an explicit null must mean 'unchanged', as it does today"
        );

        let absent: UpdateSdtmVariableRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(absent.variable_controlled, None);
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p aegis-server --lib dto::tests`

Expected: compile error — `cannot find value SdtmVariableControlled in this scope`, and `CreateSdtmVariableRequest.variable_controlled` still has type `Option<String>` so the object-shape assertion does not compile.

- [ ] **Step 3: Add the wire struct**

In `apps/server/aegis-server/src/transport/http/dto.rs`, add after the `SdtmVariableDescriptionDetail` `From` impls (after line 1207):

```rust
/// Wire projection of [`apis::domain_model::SdtmVariableControlled`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableControlled {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Vec<String>>,
}

impl From<apis::domain_model::SdtmVariableControlled> for SdtmVariableControlled {
    fn from(v: apis::domain_model::SdtmVariableControlled) -> Self {
        Self {
            code: v.code,
            format: v.format,
        }
    }
}

impl From<SdtmVariableControlled> for apis::domain_model::SdtmVariableControlled {
    fn from(v: SdtmVariableControlled) -> Self {
        Self {
            code: v.code,
            format: v.format,
        }
    }
}
```

- [ ] **Step 4: Change the wire DTO field types**

In `apps/server/aegis-server/src/transport/http/dto.rs`:

- line 1264, `SdtmVariableViewResponse`:
  `pub variable_controlled: Option<SdtmVariableControlled>,`
- line 1280, the `From<apis::domain_model::SdtmVariableView>` impl:
  `variable_controlled: v.variable_controlled.map(Into::into),`
- line 1382, `CreateSdtmVariableRequest`:
  `pub variable_controlled: Option<SdtmVariableControlled>,`
- line 1397, `UpdateSdtmVariableRequest`:
  `pub variable_controlled: Option<Option<SdtmVariableControlled>>,`
- line 1433, the `From<CreateSdtmVariableRequest>` impl:
  `variable_controlled: r.variable_controlled.map(Into::into),`

- [ ] **Step 5: Register the schema in the OpenAPI document**

In `apps/server/aegis-server/src/transport/http/openapi.rs`:

- add `dto::SdtmVariableControlled,` to the `components(schemas(…))` list, immediately after `dto::SdtmVariableDescriptionDetail,` (line 101).
- add `"SdtmVariableControlled",` to the expected schema-name list in the `#[cfg(test)]` module, immediately after `"SdtmVariableDescriptionDetail",` (line 319).

- [ ] **Step 6: Update the update handler**

In `apps/server/aegis-server/src/transport/http/domain_model/handlers.rs`, line 379 in `update_variable`:

```rust
            variable_controlled: req.variable_controlled.map(|inner| inner.map(Into::into)),
```

The `create_variable` handler (line 295) needs no change — it goes through the `From<CreateSdtmVariableRequest> for apis::domain_model::CreateSdtmVariableRequest` impl, which Step 4 already updated.

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test -p aegis-server --lib`

Expected: PASS — the two new wire tests, the OpenAPI schema-list test, and the pre-existing server unit tests.

- [ ] **Step 8: Verify the whole workspace compiles and is clean**

Run:

```bash
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
```

Expected: no errors, no warnings, no formatting diffs. The workspace compiles again — Task 2 left it broken by design, and this task closes that gap.

- [ ] **Step 9: Run the workspace test suite**

Run: `cargo test --workspace`

Expected: PASS. The `#[ignore]` live-DB tests are skipped.

- [ ] **Step 10: Commit**

```bash
git add apps/server/aegis-server
git commit -m "feat(aegis-server): expose variable_controlled as a structured value"
```

---

## Self-Review

**Spec coverage.** The spec's six sections map onto the four tasks: the domain type (§1) → Task 1; field propagation (§2) → Tasks 2–4; persistence (§3) → Task 3 Steps 5, 8; the migration (§4) → Task 3 Step 5; the wire format (§5) → Task 4 Steps 3–6; tests (§6) → Task 1 Steps 1, Task 2 Step 1, Task 3 Steps 1/6/12, Task 4 Step 1. The spec's "known consequence" (desktop version skew) is covered by the Global Constraints — the desktop is not modified.

**Step scan.** Every step names a file, a signature or literal value, and the command that verifies it. No step says "handle edge cases" or "add appropriate validation". The bodies that appear in code blocks are the ones the tests do not fully determine: the serde attributes, the JSON-null decode arm, and the two `From` impls.

**Type consistency.** `SdtmVariableControlled` is spelled identically in all four tasks. The field types are checked once and consumed by name: `Option<SdtmVariableControlled>` on create paths, `Option<Option<SdtmVariableControlled>>` on update paths — the update path keeps the three-state semantics in every layer, and the server handler's `.map(|inner| inner.map(Into::into))` preserves the outer/inner distinction.

**Review Focus.** All five items are pinned: items 3 and 4 in Task 1 Step 1, items 1 and 2 in Task 3 Step 6, item 5 in Task 4 Step 1.

**Proportion.** The plan is roughly the length of the spec. It names files, types, and assertions; it does not transcribe the finished files.
