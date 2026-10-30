# SDTM variable controlled-terminology model — design

- **Date:** 2026-10-29
- **Branch:** `feat/sdtm-model_variable_control`
- **Status:** approved
- **Scope:** `lib/crates/domain-model`, `lib/crates/apis`, `apps/server/aegis-server`
- **Out of scope:** `apps/desktop/aegis-desktop` (explicitly excluded for this turn)

## Intent

`SdtmVariable.variable_controlled` currently carries a single free-text
string. That is too coarse for SDTM controlled terminology: a variable
references one or more codelists, and separately declares the value
*format* it accepts. The field becomes a structured value so the
controlled-terminology metadata can be stored, transported, and edited
as two independent lists instead of one opaque string.

The new shape:

```
SdtmVariableControlled {
    code:   Option<Vec<String>>,
    format: Option<Vec<String>>,
}
```

`code` holds the controlled-terminology codelist references; `format`
holds the declared value formats. Both are optional and independent —
a variable may declare a codelist without declaring a format, or the
reverse.

## Goals

- Model `variable_controlled` as a structured value through every layer
  of the backend, from the domain aggregate to the HTTP wire.
- Store the value as JSONB so both lists survive a round-trip and the
  column can be queried with JSONB operators later.
- Keep the existing three-state update semantics
  (`don't change` / `clear` / `replace`) working unchanged for the new
  type.
- Change the shape of the wire contract in one place, so the desktop
  client can be updated against a single documented contract.

## Non-goals

- No new endpoints, no new query parameters, no pagination changes.
- No validation of codelist names against the `terminology` crate.
  The value is stored as given.
- No reordering, renaming, or reshaping of any other SDTM variable
  field (`variable_type`, `variable_core`, `variable_role`,
  `variable_sequence`, `descriptions`).
- No UI work — the desktop drawer that edits this field is a separate
  follow-up change.

## Anti-goals

- **No legacy string acceptance on the server.** The server accepts
  only the object shape. A client still sending the old string gets a
  deserialization error rather than a silently coerced value. A
  dual-shape (`#[serde(untagged)]`) wire type was considered and
  rejected: it keeps a shape we are deliberately retiring alive in
  three crates, and the desktop is being updated in the same release.
- **No backfill of the old strings.** Existing TEXT values are not
  convertible — the old string does not say whether it was a codelist
  or a format. They are dropped to `NULL` rather than guessed at.

## Constraints

- `lib/crates/apis` declares ports and DTOs only. It mirrors domain
  types rather than importing them, so the new struct is declared
  there separately.
- The desktop crate hand-duplicates the server's wire DTOs; it does
  **not** depend on `apis`. Changing the server's wire shape
  therefore does not break the desktop's compilation, only its runtime
  behaviour against a new server.
- `sqlx` is configured with the `json` feature, and the crate already
  stores `descriptions` as a JSONB blob. The new column follows that
  existing pattern rather than introducing a new one.
- Live-DB tests are `#[ignore]`-gated and destructive by design: they
  drop `sdtm_variables` and `_sqlx_migrations` before each run.

## Design

### 1. The domain type

`SdtmVariableControlled` lives in
`lib/crates/domain-model/src/domain/variable_type.rs`, alongside the
other controlled-terminology vocabulary types (`SdtmVariableType`,
`SdtmVariableCore`, `SdtmRole`) that describe the same concern — what
a variable's controlled terminology is.

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

`skip_serializing_if` keeps absent lists out of the JSON entirely
rather than writing `"code": null`, so the stored document is
`{"code": ["C66731"]}` rather than
`{"code": ["C66731"], "format": null}`. `#[serde(default)]` on the way
in means an absent key and an explicit `null` both deserialize to
`None`. The JSON shape is a contract with the desktop client, so it is
pinned by a unit test.

`Eq` is derived because every other field on the aggregate is `Eq` and
the aggregate derives `Eq` — `Option<Vec<String>>` supports it.

The type is re-exported from `lib/crates/domain-model/src/domain.rs`
next to the other `variable_type` re-exports.

### 2. Field propagation

`variable_controlled` changes type at every site that carries it. The
usecase, facade, and handler layers are pass-throughs: the value flows
across unchanged, so only the field type and the type-specific
conversions change.

| Layer | File | Change |
|---|---|---|
| Domain aggregate | `domain/sdtm_variable.rs` | `SdtmVariable.variable_controlled`, `SdtmVariable::new`, `SdtmVariable::for_repository`, `SdtmVariableNew.variable_controlled` → `Option<SdtmVariableControlled>`; `SdtmVariableUpdate.variable_controlled` → `Option<Option<SdtmVariableControlled>>` |
| Domain re-export | `domain.rs` | export `SdtmVariableControlled` |
| Crate-root re-export | `lib.rs` | export `SdtmVariableControlled` — `tests/integration_persistence.rs` imports domain types from the crate root, so the type must be reachable there |
| Usecase commands | `usecase/commands.rs` | `CreateSdtmVariable.variable_controlled` → `Option<SdtmVariableControlled>`; `UpdateSdtmVariable.variable_controlled` → `Option<Option<SdtmVariableControlled>>` |
| Usecase views | `usecase/views.rs` | `SdtmVariableView.variable_controlled` → `Option<SdtmVariableControlled>` |
| Postgres adapter | `adapter/persistence/postgres/sdtm_variable_repo.rs` | row decode, create bind, update bind — see below |
| In-memory facade | `adapter/facade/in_memory/service.rs` | `apis::domain_model::SdtmVariableControlled` ↔ `domain_model::domain::SdtmVariableControlled` conversion helpers, mirroring the existing `variable_descriptions_to_api` / `_from_api` pair |
| API port | `apis/src/domain_model.rs` | new `SdtmVariableControlled` mirror struct; `SdtmVariableView`, `CreateSdtmVariableRequest`, `UpdateSdtmVariableRequest` field types |
| Server wire | `apps/server/aegis-server/src/transport/http/dto.rs` | new `SdtmVariableControlled` wire struct + `From` impls both ways; `SdtmVariableViewResponse`, `CreateSdtmVariableRequest`, `UpdateSdtmVariableRequest` field types; the `From<CreateSdtmVariableRequest> for apis::domain_model::CreateSdtmVariableRequest` impl converts the create path |
| Server handler | `apps/server/aegis-server/src/transport/http/domain_model/handlers.rs` | only the **update** path is built inline: `variable_controlled: req.variable_controlled.map(\|inner\| inner.map(Into::into))`. The **create** path needs no handler change — it goes through the `From` impl in `dto.rs` |
| OpenAPI document | `apps/server/aegis-server/src/transport/http/openapi.rs` | register `dto::SdtmVariableControlled` in the explicit `components(schemas(…))` list, and in the expected-schema-name list in its `#[cfg(test)]` module — the list is hand-maintained, and an unregistered nested schema leaves a dangling `$ref` on `variableControlled` in the generated document |

### 3. Persistence

The postgres adapter follows the pattern already used for the
`descriptions` JSONB column.

**Decode.** The row struct holds `variable_controlled: Option<JsonValue>`
and parses it with `serde_json::from_value` into
`Option<SdtmVariableControlled>`. A parse failure maps to
`DomainError::Repository`, matching how `descriptions` handles it.

**Create.** `Option<SdtmVariableControlled>` is serialized to
`Option<JsonValue>`. `None` binds SQL `NULL`; `Some(v)` binds the JSON
document. JSON `null` and SQL `NULL` stay distinct — a stored value
never becomes ambiguous between "absent" and "empty".

**Update.** The three-state semantics are preserved exactly as they are
today for `variable_role`, using the same dynamic-SQL-fragment
approach:

| `UpdateSdtmVariable.variable_controlled` | SQL fragment | Effect |
|---|---|---|
| outer `None` | `COALESCE($7, variable_controlled)` | column unchanged |
| outer `Some(None)` | `variable_controlled = $7` (bound `NULL`) | column cleared to `NULL` |
| outer `Some(Some(v))` | `variable_controlled = $7` (bound JSON) | column replaced |

Only the bound type changes — from `Option<&str>` to `Option<JsonValue>`.

**"Clear" is reachable over HTTP.** The three states above are the
*repository* contract, and the domain, usecase, and adapter layers
implement all three. At the HTTP boundary,
`UpdateSdtmVariableRequest.variable_controlled` uses the repo's
`double_option` deserializer (the helper at
`apps/server/aegis-server/src/transport/http/dto.rs`, which the
sdtm-project DTOs already use) so all three survive deserialization: a
missing key is "leave the column alone", an explicit `null` is "clear
the column", and a value is "replace it". A plain `Option<Option<T>>`
derive would collapse the first two — serde maps `null` to the outer
`None` — and would leave "clear" unreachable from a client.

This closes a pre-existing gap: before this change the field's wire
type was `Option<Option<String>>` with the plain derive, so a client
could not clear it and the desktop's "clear the field" action was a
silent no-op. The change to the wire's *shape* is what made the gap
worth closing: a client that now sends `null` to clear a controlled
value must actually be able to clear it.

`variable_role` on the same DTO keeps the plain derive and therefore
keeps its existing behaviour — for that field both an absent key and an
explicit `null` mean "don't change". Aligning it is a separate,
deliberate decision, because it would change a second field's behaviour
and would make this DTO consistent with the sdtm-project endpoints in a
way that is not otherwise part of this change.

### 4. Migration

New file:
`lib/crates/domain-model/migrations/0004_alter_sdtm_variables_variable_controlled_to_jsonb.sql`

```sql
-- sdtm_variables.variable_controlled changes from TEXT to JSONB
-- carrying a SdtmVariableControlled object ({"code": [...], "format": [...]}).
-- The previous TEXT value did not record whether it was a codelist or a
-- format, so it cannot be converted and is dropped to NULL.

ALTER TABLE sdtm_variables
    ALTER COLUMN variable_controlled TYPE JSONB
    USING NULL::jsonb;
```

The column stays nullable, so no default and no `NOT NULL` constraint
is added. Existing rows lose their `variable_controlled` value; every
other column is untouched.

**Rollout.** The server does not auto-migrate — operators apply
`lib/crates/domain-model/migrations` by hand. The old and new servers
are not compatible with each other's column type in either order: after
the `ALTER`, the old server still binds `TEXT` into a `JSONB` column and
Postgres rejects every variable write; before the `ALTER`, the new
server binds `JSONB` into a `TEXT` column and the same happens. There
is no zero-downtime path, so the migration and the deploy must be
treated as one operation, and a write outage for SDTM variables during
the switch is expected. Reads are not safe to rely on either: the old
server decodes the column into a `String`, and a `JSONB` value is not
in the wire format a `TEXT` column produces, so the old server cannot
be trusted to read the migrated column correctly even though it will
not refuse to compile against it.

### 5. Wire format

The top-level field keeps its camelCase rename (`variableControlled`);
`code` and `format` are single words and are unaffected by the
camelCase rename.

```jsonc
// GET /api/domain-model/domains/{domain_id}/variables
{
  "id": 1,
  "name": "AESEV",
  "variableControlled": { "code": ["C66731"], "format": ["Codelist"] },
  "variableType": "Character",
  "variableCore": "Req",
  "variableRole": "Topic",
  "variableSequence": 1,
  "descriptions": []
}
```

A variable with no controlled terminology serialises as
`"variableControlled": null`.

### 6. Tests

**New — JSON shape (unit, `domain-model`).** Pins the serialized form
of `SdtmVariableControlled`: only the lists that are set appear; an
absent key and an explicit `null` both read back as `None`; a value
with both lists round-trips unchanged. This is a contract with the
desktop client, so it is pinned rather than left implicit.

**New — migration content (unit, `domain-model`).** Asserts that
migration `0004` exists and converts the column to JSONB, matching the
existing `migration_file_is_present_and_idempotent` test in
`adapter/persistence/postgres/sdtm_variable_repo.rs`.

**Updated — live-DB persistence (integration, `#[ignore]`).** The
existing variable tests in
`lib/crates/domain-model/tests/integration_persistence.rs` switch from
`Some("AETERM".into())` to
`Some(SdtmVariableControlled { code: Some(vec!["AETERM".into()]), format: None })`,
and the `.as_deref()` string assertions become struct equality
assertions. The three-state update tests keep covering "outer `None`
does not change", "outer `Some(None)` clears", and "outer
`Some(Some(v))` replaces" against the new type.

**Updated — in-memory fakes.** The fake repository in
`usecase/tests.rs` and the facade test in
`adapter/facade/in_memory/tests.rs` follow the new type; they are
pass-throughs, so only the literal values change.

## Known consequence: desktop version skew

`apps/desktop/aegis-desktop` is not modified in this change.

The desktop does not depend on `apis`; it hand-duplicates the server's
wire DTOs in `src-tauri/src/http/domain_model/variable.rs` and
`src/shared/api/types.ts`. It therefore still **compiles** after this
change. At runtime, against a server built from this branch, the skew
is a crash rather than a type mismatch:

- **The desktop's own write path throws.** `VariableEditDrawer.tsx`
  reads `row.variableControlled` — now an object — into its state and
  calls `.trim()` on it. `.trim` is `undefined` on an object, so
  submitting the drawer raises a `TypeError`, and the drawer renders
  `[object Object]` where the old string used to be.
- **A desktop write is rejected with a plain-text 422.** The server has
  no `Json` extractor rejection handler, so a body whose
  `variableControlled` is a string is refused by axum with its default
  `422 Unprocessable Entity` and a plain-text body — not the typed
  `ErrorBody { code, message }` the desktop's error mapper expects.

  This is **server-side debt, not desktop-side**. `CLAUDE.md` states the
  invariant plainly — "Errors are always rendered as a typed
  `ErrorBody { code, message }` so the desktop client can dispatch on
  the stable `code`" — and the server does not honour it for extractor
  rejections on any endpoint. The desktop cannot fix this from its
  side: it cannot parse an error body that is not the documented shape.
  The fix is a `Json` rejection handler mapping `JsonRejection` to
  `ErrorBody` with a stable code, and it belongs in a change of its
  own — it is a server-wide error-contract change, not a detail of
  this type migration. Until then, a client that sends the legacy
  string sees a plain-text 422 and cannot distinguish "your body is
  malformed" from "the server is broken".

This skew is intentional and temporary: the desktop is updated in the
follow-up change that adopts the new wire contract. The follow-up
should send `null` to clear `variableControlled` (the server now
honours that — see the note in §3) and `{"code": [...], "format": [...]}`
to replace it.

## What's left to the builder

- The exact `From` impl bodies for the `apis` ↔ wire and `apis` ↔
  domain conversions are mechanical and should follow the existing
  `SdtmVariableDescription` conversions in the same files.
- The `SdtmVariableControlled` mirror in `apis` should derive the same
  set as its sibling DTOs there (`Debug, Clone, PartialEq, Eq,
  Serialize, Deserialize`).
- The server wire struct needs a `ToSchema` derive, matching the other
  wire DTOs in `dto.rs`, so the OpenAPI document stays complete.
