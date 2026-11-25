# Add `version_id` to `CodeItemListQuery` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional `version_id: Option<i64>` field to `CodeItemListQuery` (both domain and API-contract variants) so callers can scope the listing to a single terminology version, and propagate the field through the usecase, repository trait/impls, service trait/impl, and HTTP route without affecting the `aegis-desktop` crate or its frontend.

**Architecture:** Mirror the existing `codelist_id: Option<i64>` pattern verbatim — `Option<i64>` in both struct copies, `skip_serializing_if = "Option::is_none"` on the wire DTO, repository filters via `Option::map_or(true, |v| …)`. The usecase already uses `..query` syntax so it picks the new field up automatically. Field is `Option` (not required) so the existing "list everything" path keeps working unchanged for callers that don't supply it.

**Tech Stack:** Rust (edition 2021), `sqlx` for Postgres, `utoipa` for OpenAPI, `serde_json` for DTO tests, `axum` for HTTP handlers.

## Global Constraints

- **No changes to `apps/desktop/aegis-desktop/`** — desktop has its own internal mirror of `CodeItemListQuery` and its own consumer surface. The backend change is additive and must compile-against without desktop edits.
- **No change to `CodeItemRepository::search_or_list` trait signature** — only the input struct gains a field.
- **Field order convention:** in both struct definitions, place `version_id` immediately after `codelist_id` (or before — pick **before**, mirroring `CodeListListQuery` which has `version_id` first). All existing construction sites get `version_id: None` prepended so the codebase continues to compile.
- **Serde convention:** copy the `codelist_id` serde attribute pair exactly: `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- **Frequent commits:** one commit per task.

## Touch Points Summary

| # | File | Change |
|---|---|---|
| 1 | `lib/crates/apis/src/terminology.rs` | Add `version_id: Option<i64>` to `CodeItemListQuery` (lines 125–131) |
| 2 | `lib/crates/terminology/src/domain/code_item.rs` | Add `version_id: Option<i64>` to `CodeItemListQuery` (lines 115–121) |
| 3 | All `CodeItemListQuery { … }` construction sites | Prepend `version_id: None` (or specific value) |
| 4 | `lib/crates/terminology/src/adapter/facade/in_memory/service.rs` | Field-forward `version_id` (lines 327–342) |
| 5 | `lib/crates/terminology/src/adapter/facade/in_memory/tests.rs` | Filter in in-memory repo + regression test |
| 6 | `lib/crates/terminology/src/adapter/persistence/postgres/code_item_repo.rs` | Extend SQL builder (lines 116–133) |
| 7 | `lib/crates/terminology/tests/integration_persistence.rs` | Postgres-backed regression test |
| 8 | `apps/server/aegis-server/src/transport/http/dto.rs` | Wire DTO + serde round-trip test |
| 9 | `apps/server/aegis-server/src/transport/http/terminology/handlers.rs` | Destructure + forward + `utoipa::path` param |
| 10 | `apps/server/aegis-server/src/transport/http/router.rs` | Router-stub echo (lines ~988–1021) |

---

## Task 1: Add `version_id` field to both `CodeItemListQuery` structs

**Files:**
- Modify: `lib/crates/apis/src/terminology.rs:125-131`
- Modify: `lib/crates/terminology/src/domain/code_item.rs:115-121`
- Modify: every existing construction site for either struct (see Step 4)

**Interfaces:**
- Consumes: nothing (struct definition only)
- Produces: `CodeItemListQuery { version_id: Option<i64>, … }` in both crates

- [ ] **Step 1: Edit `lib/crates/apis/src/terminology.rs`**

Replace lines 125–131:

```rust
#[derive(Debug, Clone)]
pub struct CodeItemListQuery {
    pub codelist_id: Option<i64>,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}
```

with:

```rust
#[derive(Debug, Clone)]
pub struct CodeItemListQuery {
    /// Optional owning version id. `Some(_)` restricts the
    /// result set to code items whose `version_id` matches;
    /// `None` lists every code item the backend knows about
    /// (optionally further narrowed by `codelist_id`).
    pub version_id: Option<i64>,
    pub codelist_id: Option<i64>,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}
```

Also update the doc comment above the struct (lines 120–124) so it mentions `version_id`:

```rust
/// Query for [`TerminologyService::list_code_items`]. Mirrors
/// [`CodeListListQuery`] but scopes to a `codelist_id`. Both
/// `version_id` and `codelist_id` are optional: `Some(_)`
/// restricts to a single owning version / codelist, `None`
/// lists every code item known to the backend.
```

- [ ] **Step 2: Edit `lib/crates/terminology/src/domain/code_item.rs`**

Replace lines 115–121:

```rust
#[derive(Debug, Clone)]
pub struct CodeItemListQuery {
    pub codelist_id: Option<i64>,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}
```

with:

```rust
#[derive(Debug, Clone)]
pub struct CodeItemListQuery {
    /// Optional owning version id. `Some(_)` restricts the
    /// result set to code items whose `version_id` matches;
    /// `None` lists every code item (optionally further
    /// narrowed by `codelist_id`).
    pub version_id: Option<i64>,
    pub codelist_id: Option<i64>,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}
```

Update the doc comment at lines 108–114 to mention `version_id`:

```rust
/// Query for `CodeItemRepository::search_or_list`. Mirrors
/// [`CodeListListQuery`](super::code_list::CodeListListQuery)
/// but scopes to a `codelist_id` instead of a version. Both
/// `version_id` and `codelist_id` are optional: `Some(_)`
/// restricts to a single owning version / codelist (the
/// typical per-version or per-codelist browse path); `None`
/// returns every code item across every codelist known to
/// the backend.
```

- [ ] **Step 3: Find every existing construction site**

Run:
```bash
grep -rn "CodeItemListQuery {" /root/coding/project/aegis
```

Expected hits include (non-exhaustive — confirm by grep):
- `lib/crates/terminology/src/usecase/tests.rs:~820-845`
- `lib/crates/terminology/src/adapter/facade/in_memory/tests.rs` (multiple sites around lines 994–1246)
- `lib/crates/terminology/tests/integration_persistence.rs:~438-521`
- `apps/server/aegis-server/tests/integration_auth.rs:~318-326`
- `apps/server/aegis-server/src/transport/http/terminology/handlers.rs:~460-480`
- `apps/server/aegis-server/src/transport/http/router.rs:~988-1021` (only if it constructs the struct)

For the apis-crate variant, also grep for the **struct literal body** pattern after the existing edits:
```bash
grep -rn "codelist_id:" /root/coding/project/aegis
```

- [ ] **Step 4: Update each construction site**

For every `CodeItemListQuery { … }` literal, prepend `version_id: None,` as the first field. Example — given an existing site:

```rust
CodeItemListQuery {
    codelist_id: None,
    fragment: None,
    offset: 0,
    limit: 50,
}
```

change it to:

```rust
CodeItemListQuery {
    version_id: None,
    codelist_id: None,
    fragment: None,
    offset: 0,
    limit: 50,
}
```

For `terminology::CodeItemListQuery` sites (usecase tests, integration_persistence), do the same.

For the handler at `apps/server/aegis-server/src/transport/http/terminology/handlers.rs:~460-480` (which destructures `Query(CodeItemListQuery { codelist_id, fragment, offset, limit })`), Task 5 will fully rewrite it. For now just add `version_id: None` to any *struct-literal* construction in that file.

**Important:** do NOT change construction sites inside the in-memory or postgres repositories — they don't construct `CodeItemListQuery` themselves; they consume one parameter from the trait method and build the SQL on its fields.

- [ ] **Step 5: Verify the workspace compiles**

Run:
```bash
cargo check --workspace --tests
```

Expected: PASS. If you see "missing field `version_id`" errors, you've missed a construction site — go back to Step 3 and re-grep.

- [ ] **Step 6: Commit**

```bash
git add lib/crates/apis/src/terminology.rs \
        lib/crates/terminology/src/domain/code_item.rs \
        $(git diff --name-only --diff-filter=M | grep -E '(CodeItemListQuery|tests\.rs|handlers\.rs|integration_persistence\.rs|integration_auth\.rs)')
git commit -m "feat(terminology): add optional version_id to CodeItemListQuery"
```

---

## Task 2: Field-forward `version_id` in the in-memory service

**Files:**
- Modify: `lib/crates/terminology/src/adapter/facade/in_memory/service.rs:327-342`

**Interfaces:**
- Consumes: `apis::terminology::CodeItemListQuery` (now with `version_id`)
- Produces: `terminology::CodeItemListQuery` (now with `version_id`)

- [ ] **Step 1: Read the current `list_code_items` method**

Confirm lines 327–342 in `service.rs`:

```rust
async fn list_code_items(
    &self,
    query: ApiCodeItemListQuery,
) -> Result<ApiPage<CodeItemView>, TerminologyApiError> {
    let internal_q = CodeItemListQuery {
        codelist_id: query.codelist_id,
        fragment: query.fragment,
        offset: query.offset,
        limit: query.limit,
    };
    let page = self.usecase.list_code_items(internal_q).await?;
    Ok(ApiPage { … })
}
```

- [ ] **Step 2: Add `version_id` to the field-forward**

Replace the struct literal with:

```rust
    let internal_q = CodeItemListQuery {
        version_id: query.version_id,
        codelist_id: query.codelist_id,
        fragment: query.fragment,
        offset: query.offset,
        limit: query.limit,
    };
```

- [ ] **Step 3: Verify it compiles**

Run:
```bash
cargo check -p terminology --tests
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add lib/crates/terminology/src/adapter/facade/in_memory/service.rs
git commit -m "feat(terminology): forward version_id in in-memory service adapter"
```

---

## Task 3: Add `version_id` filter to the in-memory repo + regression test (TDD)

**Files:**
- Modify: `lib/crates/terminology/src/adapter/facade/in_memory/tests.rs:352-388` (in-memory `search_or_list`)
- Modify: `lib/crates/terminology/src/adapter/facade/in_memory/tests.rs` (add regression test, mirror lines 1217–1252)

**Interfaces:**
- Consumes: `CodeItemListQuery { version_id: Option<i64>, … }`
- Produces: `Page<CodeItem>` filtered by `version_id` when `Some(_)`, unfiltered when `None`

- [ ] **Step 1: Write the failing regression test**

Append to `lib/crates/terminology/src/adapter/facade/in_memory/tests.rs` (after the `list_code_items_without_codelist_id_returns_all_codelists` test at line 1252):

```rust
#[tokio::test]
async fn list_code_items_with_version_id_filters_to_that_version() {
    // Regression for `CodeItemListQuery::version_id: Option<i64>`:
    // when the caller supplies `version_id`, only items whose
    // `version_id` matches must come back, even when `codelist_id`
    // is omitted (the global-list path).
    let svc = service();
    let v1 = svc.create_version(create_version_req("v1")).await.unwrap();
    let v2 = svc.create_version(create_version_req("v2")).await.unwrap();
    let cl1 = svc.create_code_list(create_code_list_req(v1.id, "C1")).await.unwrap();
    let cl2 = svc.create_code_list(create_code_list_req(v2.id, "C2")).await.unwrap();
    svc.create_code_item(create_code_item_req(cl1.id, v1.id, "ALPHA")).await.unwrap();
    svc.create_code_item(create_code_item_req(cl2.id, v2.id, "BETA")).await.unwrap();

    let page = svc
        .list_code_items(apis::terminology::CodeItemListQuery {
            version_id: Some(v1.id),
            codelist_id: None,
            fragment: None,
            offset: 0,
            limit: 50,
        })
        .await
        .unwrap();
    let mut codes: Vec<String> = page.items.iter().map(|i| i.code.clone()).collect();
    codes.sort();
    assert_eq!(codes, vec!["ALPHA".to_string()]);
}

#[tokio::test]
async fn list_code_items_with_version_id_combined_with_codelist_id() {
    // When both `version_id` and `codelist_id` are supplied,
    // the result must satisfy BOTH predicates.
    let svc = service();
    let v = svc.create_version(create_version_req("v")).await.unwrap();
    let cl1 = svc.create_code_list(create_code_list_req(v.id, "C1")).await.unwrap();
    let cl2 = svc.create_code_list(create_code_list_req(v.id, "C2")).await.unwrap();
    svc.create_code_item(create_code_item_req(cl1.id, v.id, "IN_CL1")).await.unwrap();
    svc.create_code_item(create_code_item_req(cl2.id, v.id, "IN_CL2")).await.unwrap();

    let page = svc
        .list_code_items(apis::terminology::CodeItemListQuery {
            version_id: Some(v.id),
            codelist_id: Some(cl1.id),
            fragment: None,
            offset: 0,
            limit: 50,
        })
        .await
        .unwrap();
    let mut codes: Vec<String> = page.items.iter().map(|i| i.code.clone()).collect();
    codes.sort();
    assert_eq!(codes, vec!["IN_CL1".to_string()]);
}
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run:
```bash
cargo test -p terminology --lib list_code_items_with_version_id_filters_to_that_version list_code_items_with_version_id_combined_with_codelist_id
```

Expected: FAIL — `list_code_items` currently doesn't filter by `version_id`, so the first test will return both `ALPHA` and `BETA`, and the second test will return both `IN_CL1` and `IN_CL2`.

- [ ] **Step 3: Add the `version_id` filter to the in-memory repo**

Edit `lib/crates/terminology/src/adapter/facade/in_memory/tests.rs:352-388` — find the existing `search_or_list` implementation:

```rust
async fn search_or_list(
    &self,
    q: CodeItemListQuery,
) -> Result<Page<CodeItem>, DomainError> {
    let mut all: Vec<CodeItem> = self
        .state
        .lock()
        .unwrap()
        .by_id
        .values()
        .filter(|i| q.codelist_id.map_or(true, |cid| i.codelist_id == cid))
        .cloned()
        .collect();
    …
```

Add the `version_id` filter. Chain it AFTER the `codelist_id` filter (or before — order doesn't matter for correctness):

```rust
        .filter(|i| q.codelist_id.map_or(true, |cid| i.codelist_id == cid))
        .filter(|i| q.version_id.map_or(true, |vid| i.version_id == vid))
```

- [ ] **Step 4: Run the new tests to verify they pass**

Run:
```bash
cargo test -p terminology --lib list_code_items_with_version_id_filters_to_that_version list_code_items_with_version_id_combined_with_codelist_id
```

Expected: PASS.

- [ ] **Step 5: Run the whole in-memory test module to confirm no regression**

Run:
```bash
cargo test -p terminology --lib
```

Expected: PASS — every existing `CodeItemListQuery` test (which now passes `version_id: None` after Task 1) must still pass.

- [ ] **Step 6: Commit**

```bash
git add lib/crates/terminology/src/adapter/facade/in_memory/tests.rs
git commit -m "feat(terminology): filter code items by version_id in in-memory repo"
```

---

## Task 4: Extend Postgres `search_or_list` SQL builder (no test for this task — covered by Task 5)

**Files:**
- Modify: `lib/crates/terminology/src/adapter/persistence/postgres/code_item_repo.rs:116-133`

**Interfaces:**
- Consumes: `CodeItemListQuery { version_id: Option<i64>, … }`
- Produces: SQL `SELECT … WHERE … [AND] version_id = $X` (omitted when `None`)

- [ ] **Step 1: Read the current SQL builder**

Confirm lines 116–133:

```rust
let mut has_where = false;
if let Some(codelist_id) = q.codelist_id {
    qb.push(" WHERE codelist_id = ");
    qb.push_bind(codelist_id);
    has_where = true;
}
let frag_filter = q.fragment.as_deref().filter(|s| !s.trim().is_empty());

if let Some(frag) = frag_filter {
    qb.push(if has_where { " AND " } else { " WHERE " });
    …
```

- [ ] **Step 2: Add the `version_id` branch BEFORE the `codelist_id` branch**

Insert above the existing `if let Some(codelist_id) = q.codelist_id { … }` block:

```rust
if let Some(version_id) = q.version_id {
    qb.push(" WHERE version_id = ");
    qb.push_bind(version_id);
    has_where = true;
}
```

(Placing it before `codelist_id` matches the natural hierarchy: version → codelist → item.)

- [ ] **Step 3: Verify it compiles**

Run:
```bash
cargo check -p terminology
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add lib/crates/terminology/src/adapter/persistence/postgres/code_item_repo.rs
git commit -m "feat(terminology): add version_id WHERE clause to Postgres code_item repo"
```

---

## Task 5: Postgres-backed regression test for `version_id` (TDD)

**Files:**
- Modify: `lib/crates/terminology/tests/integration_persistence.rs` (append test)

**Interfaces:**
- Consumes: real Postgres connection, `CodeItemRepo::search_or_list`
- Produces: filtered `Page<CodeItem>` matching the supplied `version_id`

**Requires:** A running Postgres (the integration test harness sets one up via `#[sqlx::test]` — confirm by reading the file's `mod` boilerplate before writing).

- [ ] **Step 1: Read the harness**

Read the first ~60 lines of `lib/crates/terminology/tests/integration_persistence.rs` to learn:
- The fixture macro used (`#[sqlx::test]` vs custom)
- How other `search_or_list` tests construct their fixtures (e.g. `list_code_items_paginates_across_multiple_pages` at ~438–521)
- The helper functions (`setup_code_item`, `setup_code_list`, `setup_version`)

If the harness is non-`#[sqlx::test]` (i.e. it requires manual `DATABASE_URL`), skip Task 5 — fall back to "covered by Task 4 + integration test in a follow-up PR" and add a `// TODO` comment in `code_item_repo.rs`.

- [ ] **Step 2: Write the failing integration test**

Append to `integration_persistence.rs` (mirror the fixture shape of the existing pagination test):

```rust
#[sqlx::test]
async fn list_code_items_filters_by_version_id(pool: sqlx::PgPool) {
    let repo = CodeItemRepo::new(pool.clone());

    let v1 = repo
        .create_version(terminology::domain::NewTerminologyVersion {
            kind: terminology::domain::TerminologyKind::Sdtm,
            name: "v1".into(),
        })
        .await
        .unwrap();
    let v2 = repo
        .create_version(terminology::domain::NewTerminologyVersion {
            kind: terminology::domain::TerminologyKind::Sdtm,
            name: "v2".into(),
        })
        .await
        .unwrap();

    let cl1 = repo
        .create_code_list(terminology::domain::CodeListNew {
            version_id: v1.id,
            code: "C1".into(),
            extensible: false,
            name: "n".into(),
            submission_value: "sv".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .unwrap();
    let cl2 = repo
        .create_code_list(terminology::domain::CodeListNew {
            version_id: v2.id,
            code: "C2".into(),
            extensible: false,
            name: "n".into(),
            submission_value: "sv".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .unwrap();

    repo.create_code_item(terminology::domain::CodeItemNew {
        codelist_id: cl1.id,
        version_id: v1.id,
        code: "ONLY_V1".into(),
        submission_value: "sv".into(),
        synonym: "".into(),
        definition: "".into(),
        nci_preferred_term: "".into(),
    })
    .await
    .unwrap();
    repo.create_code_item(terminology::domain::CodeItemNew {
        codelist_id: cl2.id,
        version_id: v2.id,
        code: "ONLY_V2".into(),
        submission_value: "sv".into(),
        synonym: "".into(),
        definition: "".into(),
        nci_preferred_term: "".into(),
    })
    .await
    .unwrap();

    let page = repo
        .search_or_list(terminology::domain::CodeItemListQuery {
            version_id: Some(v1.id),
            codelist_id: None,
            fragment: None,
            offset: 0,
            limit: 50,
        })
        .await
        .unwrap();
    let mut codes: Vec<String> = page.items.iter().map(|i| i.code.clone()).collect();
    codes.sort();
    assert_eq!(codes, vec!["ONLY_V1".to_string()]);
}
```

Adjust field names (`NewTerminologyVersion`, `CodeListNew`, `CodeItemNew`) to whatever the existing integration tests use — copy/paste from the pagination test fixture.

- [ ] **Step 3: Run the test to verify it fails**

Run:
```bash
DATABASE_URL=… cargo test -p terminology --test integration_persistence list_code_items_filters_by_version_id
```

(Use the same `DATABASE_URL` form as the other integration tests in the file — often the test harness spins up a per-test DB automatically and no env var is needed.)

Expected: FAIL — the SQL currently doesn't filter by `version_id`, so both items come back.

- [ ] **Step 4: Confirm Task 4's implementation already passes this test**

Re-run the same command. Expected: PASS (Task 4's WHERE clause addition makes the assertion hold).

- [ ] **Step 5: Commit**

```bash
git add lib/crates/terminology/tests/integration_persistence.rs
git commit -m "test(terminology): integration test for version_id filter on code items"
```

---

## Task 6: Add `version_id` to the HTTP wire DTO + serde round-trip test (TDD)

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs:771-788`
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs:1493-1521` (test module)

- [ ] **Step 1: Read the current DTO and the existing serde round-trip test**

Confirm the DTO at `apps/server/aegis-server/src/transport/http/dto.rs:771-788`:

```rust
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemListQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codelist_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
}
```

Also read the existing serde test block around line 1493–1521 — it round-trips `codelist_id: None` and `codelist_id: Some(7)`.

- [ ] **Step 2: Write the failing serde test**

Inside the existing test module (mirror the existing tests):

```rust
#[test]
fn code_item_list_query_version_id_round_trips() {
    let q = CodeItemListQuery {
        version_id: Some(42),
        codelist_id: None,
        fragment: None,
        offset: 0,
        limit: 25,
    };
    let json = serde_json::to_string(&q).unwrap();
    assert!(json.contains("\"versionId\":42"), "missing versionId, got: {json}");

    let back: CodeItemListQuery = serde_json::from_str(&json).unwrap();
    assert_eq!(back.version_id, Some(42));

    // `None` must be skipped on serialise.
    let q_none = CodeItemListQuery {
        version_id: None,
        codelist_id: None,
        fragment: None,
        offset: 0,
        limit: 25,
    };
    let json_none = serde_json::to_string(&q_none).unwrap();
    assert!(!json_none.contains("versionId"), "versionId should be skipped when None, got: {json_none}");
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run:
```bash
cargo test -p aegis-server --lib code_item_list_query_version_id_round_trips
```

Expected: FAIL — `CodeItemListQuery` has no field named `version_id` yet.

- [ ] **Step 4: Add `version_id` to the DTO**

Replace the DTO at `apps/server/aegis-server/src/transport/http/dto.rs:771-788` with:

```rust
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
```

- [ ] **Step 5: Run the test to verify it passes**

Run:
```bash
cargo test -p aegis-server --lib code_item_list_query_version_id_round_trips
```

Expected: PASS.

- [ ] **Step 6: Run all DTO serde tests to confirm no regression**

Run:
```bash
cargo test -p aegis-server --lib transport::http::dto
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs
git commit -m "feat(server): add version_id to wire DTO CodeItemListQuery"
```

---

## Task 7: Forward `version_id` in the HTTP handler + update `utoipa::path` params

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/terminology/handlers.rs:440-480`

- [ ] **Step 1: Read the current handler**

Confirm `handlers.rs:440-480`. The current code:

```rust
#[utoipa::path(
    get,
    path = "/api/terminology/code-items",
    params(
        ("codelistId" = Option<i64>, Query, description = "…"),
        ("fragment" = Option<String>, Query, description = "…"),
        ("offset" = u32, Query, default = 0),
        ("limit" = u32, Query, default = 50),
    ),
    …,
    responses(…)
)]
pub async fn list_code_items(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Query(CodeItemListQuery {
        codelist_id,
        fragment,
        offset,
        limit,
    }): Query<CodeItemListQuery>,
) -> Result<Json<dto::PagedCodeItemListResponse>, ApiError> {
    let page = state
        .terminology
        .list_code_items(apis::terminology::CodeItemListQuery {
            codelist_id,
            fragment,
            offset,
            limit,
        })
        .await?;
    Ok(Json(page.into()))
}
```

- [ ] **Step 2: Add `versionId` to the `utoipa::path` params**

Insert a new entry in the `params(…)` list, **before** the `"codelistId"` entry (mirroring the DTO field order):

```rust
        ("versionId" = Option<i64>, Query, description = "Optional owning version id; restricts results to items whose `version_id` matches."),
        ("codelistId" = Option<i64>, Query, description = "…"),
```

- [ ] **Step 3: Add `version_id` to the destructure and the struct literal**

Replace the handler body:

```rust
pub async fn list_code_items(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Query(CodeItemListQuery {
        version_id,
        codelist_id,
        fragment,
        offset,
        limit,
    }): Query<CodeItemListQuery>,
) -> Result<Json<dto::PagedCodeItemListResponse>, ApiError> {
    let page = state
        .terminology
        .list_code_items(apis::terminology::CodeItemListQuery {
            version_id,
            codelist_id,
            fragment,
            offset,
            limit,
        })
        .await?;
    Ok(Json(page.into()))
}
```

- [ ] **Step 4: Verify it compiles**

Run:
```bash
cargo check -p aegis-server
```

Expected: PASS.

- [ ] **Step 5: Run any existing handler integration tests**

Run:
```bash
cargo test -p aegis-server --test integration_auth
```

Expected: PASS. (Existing test sites already updated in Task 1's Step 4.)

- [ ] **Step 6: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/terminology/handlers.rs
git commit -m "feat(server): forward version_id through code-items HTTP handler"
```

---

## Task 8: Echo `version_id` in the router-test stub

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/router.rs:~988-1021`

- [ ] **Step 1: Read the stub**

Locate the `list_code_items` stub in `router.rs` (around line 988–1021). Current shape is something like:

```rust
async fn list_code_items(
    &self,
    query: apis::terminology::CodeItemListQuery,
) -> Result<…, …> {
    let codelist_id = query.codelist_id.unwrap_or(0);
    …
}
```

- [ ] **Step 2: Add the `version_id` echo**

Mirror the `codelist_id.unwrap_or(0)` line:

```rust
    let version_id = query.version_id.unwrap_or(0);
    let codelist_id = query.codelist_id.unwrap_or(0);
    …
```

Wire `version_id` into whatever response shape the stub returns — match what it already does for `codelist_id`.

- [ ] **Step 3: Verify it compiles and router tests pass**

Run:
```bash
cargo test -p aegis-server --lib router
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/router.rs
git commit -m "test(server): echo version_id in code-items router stub"
```

---

## Task 9: Full workspace verification

- [ ] **Step 1: Type-check the entire workspace**

Run:
```bash
cargo check --workspace --all-targets
```

Expected: PASS. **Confirm `aegis-desktop` still compiles** — if it does not, the desktop crate has an unexpected coupling and the change must be reverted (the user's constraint).

- [ ] **Step 2: Run every terminology unit + integration test**

Run:
```bash
cargo test -p terminology
```

Expected: PASS (covers the in-memory repo + regression tests from Task 3 and the Postgres integration test from Task 5).

- [ ] **Step 3: Run every server test**

Run:
```bash
cargo test -p aegis-server
```

Expected: PASS (covers DTO serde tests from Task 6 and handler/router tests from Tasks 7–8).

- [ ] **Step 4: Confirm `aegis-desktop` is untouched**

Run:
```bash
git diff --stat main -- apps/desktop/aegis-desktop
```

Expected: empty diff (the constraint forbids changes there).

- [ ] **Step 5: Final commit (only if Step 1–4 surfaced any fix-ups)**

If any of the above surfaced a missing construction site, fix it and commit with:
```bash
git commit -am "fix: address review findings from full-workspace check"
```

(No commit needed if Steps 1–4 all passed cleanly.)

---

## Self-Review Notes

- **Spec coverage:**
  - Optional `version_id` in domain `CodeItemListQuery` → Task 1
  - Usecase layer → already covered by `..query` (Task 1 enables it; no separate task needed)
  - Repository trait → no signature change (Task 1)
  - Repository impl (in-memory) → Task 3
  - Repository impl (Postgres) → Tasks 4–5
  - Service trait → no signature change (Task 1)
  - Service impl (in-memory) → Task 2
  - HTTP route DTO → Task 6
  - HTTP handler → Task 7
  - HTTP router stub → Task 8
  - Desktop untouched → Task 9 verifies
- **Placeholder scan:** none — every step shows actual code or an exact command.
- **Type consistency:** `version_id: Option<i64>` is used identically in `apis::terminology::CodeItemListQuery`, `terminology::CodeItemListQuery`, and the wire DTO `dto::CodeItemListQuery`. Field order is `version_id, codelist_id, fragment, offset, limit` everywhere.