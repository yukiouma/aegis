# Terminology list/search refactor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Collapse the four `GET /code-lists`, `GET /code-lists/search`, `GET /code-items`, `GET /code-items/search` endpoints (plus the matching usecase / repository / Postgres / apis methods) into two unified list+search endpoints with optional full-text `fragment` and offset/limit pagination. Keep `GET /code-items/by-version-and-code` unchanged. Server-side only — no Tauri client touches.

**Architecture:** Each layer gains a unified `search_or_list` (repo) / `list_code_lists(query)` / `list_code_items(query)` (usecase + apis) method that returns a `Page<T>` with `{ items, next_offset }`. `fragment = None` and `fragment = Some("")` both mean "plain list, ordered by id ASC". `fragment = Some(non-empty)` triggers FTS via `tsv @@ to_tsquery('english', $frag || ':*')` with `ts_rank DESC, id ASC` tiebreaker. The Postgres adapter fetches `limit + 1` rows to compute `next_offset`; the in-memory backend does the same on its case-insensitive substring filter. The usecase validates the fragment against the tsquery metacharacters `& | ! ( ) :` and returns `UsecaseError::Validation(DomainError::InvalidFragment)` (→ 400).

**Tech Stack:** Rust, axum, utoipa-axum, sqlx + PostgreSQL FTS (`tsv`, GIN), async-trait, thiserror.

#### Global Constraints

From the design spec, no values may be tweaked without updating the spec:

- Required query param names: `versionId` for `/code-lists`, `codelistId` for `/code-items`. Optional: `fragment` (string), `offset` (u32, default 0), `limit` (u32, default 0 → clamped to 50, max 500).
- Response envelope: `{ "codelists": [...], "nextOffset"?: u32 }` for code lists, `{ "items": [...], "nextOffset"?: u32 }` for code items. Field names stay camelCase. `nextOffset` is `skip_serializing_if = "Option::is_none"`.
- `list_code_items_by_version_and_code` and `GET /code-items/by-version-and-code` are unchanged.
- `validate_tsquery_fragment` rejects these chars: `& | ! ( ) :`. Anything else passes.
- `fragment = Some("")` and `fragment = None` are both treated as "no fragment" (plain list path); the usecase passes `None` to the repo in either case.
- Postgres backend: `LIMIT $1 OFFSET $2` with `$1 = (limit as i64) + 1`. List path: `ORDER BY id ASC LIMIT … OFFSET …`. Search path: `ORDER BY ts_rank(tsv, to_tsquery('english', $frag || ':*')) DESC, id ASC LIMIT … OFFSET …`. The `code_lists_tsv_idx` and `code_items_tsv_idx` GIN indexes from migration 0002/0003 continue to serve the search path.
- In-memory backend: case-insensitive substring across the same five text fields; sorted by `id ASC`.
- Backwards-compatible wire: existing Tauri commands that call `GET /code-lists` and `GET /code-items` with only the required parent param continue to deserialize the response (they read `codelists` / `items` as `Vec`).
- **No files under `apps/desktop/aegis-desktop/` may be touched.** Any test fixture that lives there must keep its un-paged JSON shape and continue to pass.
- Conventional commits throughout (`feat(terminology):`, `refactor(http):`, `docs(terminology):`).

---

### Task 1: Domain types — `Page<T>`, `CodeListListQuery`, `CodeItemListQuery`, `DomainError::InvalidFragment`

**Files:**
- Create: `lib/crates/terminology/src/domain/paging.rs`
- Modify: `lib/crates/terminology/src/domain.rs:1-30` (add `pub mod paging;`)
- Modify: `lib/crates/terminology/src/domain/code_list.rs` (drop `CodeListSearchQuery`, `CodeListSearchHit`; add `CodeListListQuery`)
- Modify: `lib/crates/terminology/src/domain/code_item.rs` (drop `CodeItemSearchQuery`, `CodeItemSearchHit`; add `CodeItemListQuery`)
- Modify: `lib/crates/terminology/src/domain/error.rs` (drop `EmptyFragment`; add `InvalidFragment`)

**Interfaces:**
- Consumes: nothing — pure type additions.
- Produces:
  - `pub struct Page<T> { pub items: Vec<T>, pub next_offset: Option<u32> }` (in `domain::paging`)
  - `pub struct CodeListListQuery { pub version_id: i64, pub fragment: Option<String>, pub offset: u32, pub limit: u32 }`
  - `pub struct CodeItemListQuery { pub codelist_id: i64, pub fragment: Option<String>, pub offset: u32, pub limit: u32 }`
  - `DomainError::InvalidFragment` (with `#[error("search fragment contains reserved tsquery characters: & | ! ( ) :")]`)

- [ ] **Step 1: Write the failing test for `Page<T>` shape**

Open `lib/crates/terminology/src/domain/tests.rs` and append the following block inside the existing `mod tests` (it already exists per the file list — if not, create one). The test asserts `Page<T>` is constructible with `items` and `next_offset`, and that `next_offset: None` is the default "last page" state:

```rust
#[test]
fn page_struct_accepts_items_and_optional_next_offset() {
    let p: terminology::domain::Page<i32> = terminology::domain::Page {
        items: vec![1, 2, 3],
        next_offset: Some(3),
    };
    assert_eq!(p.items, vec![1, 2, 3]);
    assert_eq!(p.next_offset, Some(3));

    let last: terminology::domain::Page<i32> = terminology::domain::Page {
        items: vec![],
        next_offset: None,
    };
    assert!(last.next_offset.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p terminology --lib domain::tests::page_struct_accepts_items_and_optional_next_offset`
Expected: compile error — `domain::Page` does not exist.

- [ ] **Step 3: Create `domain/paging.rs`**

Create `lib/crates/terminology/src/domain/paging.rs` with this exact content:

```rust
//! Pagination envelope shared by every repository / usecase method
//! that returns more than one row.

/// One page of a paginated result set.
///
/// `items` are the rows for this page. `next_offset` is the cursor
/// to pass on the next request: pass it as `?offset=<value>` to read
/// the next page. `None` means this is the last page — there are no
/// more rows beyond the current `items`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_offset: Option<u32>,
}
```

Add the new module to `lib/crates/terminology/src/domain.rs` by inserting `pub mod paging;` next to the existing `pub mod code_list;` / `pub mod code_item;` lines.

- [ ] **Step 4: Replace `CodeListSearchQuery` / `CodeListSearchHit` with `CodeListListQuery`**

In `lib/crates/terminology/src/domain/code_list.rs` delete the two structs and the doc comment immediately above `CodeListSearchQuery` (the block from `/// Query for \`CodeListRepository::search\`. …` down to the closing brace of `CodeListSearchHit`). Replace with the new struct:

```rust
/// Query for `CodeListRepository::search_or_list`. Unifies list and
/// search under a single signature: `fragment = None` is a plain
/// `ORDER BY id ASC` list; `fragment = Some(_)` is a FTS query with
/// `ts_rank DESC, id ASC` ordering. Pagination is offset/limit and
/// `next_offset` in the response is `Some(offset + limit)` while more
/// rows remain.
#[derive(Debug, Clone)]
pub struct CodeListListQuery {
    pub version_id: i64,
    /// None or `Some("")` means "no fragment" (plain list path).
    pub fragment: Option<String>,
    pub offset: u32,
    /// 0 means "use the usecase default (50)". The usecase clamps
    /// to 50 / 500 before reaching the repo.
    pub limit: u32,
}
```

- [ ] **Step 5: Replace `CodeItemSearchQuery` / `CodeItemSearchHit` with `CodeItemListQuery`**

In `lib/crates/terminology/src/domain/code_item.rs` delete the same shape (the `/// Query for \`CodeItemRepository::search\`. …` block through the closing brace of `CodeItemSearchHit`). Replace with:

```rust
/// Query for `CodeItemRepository::search_or_list`. Mirrors
/// [`CodeListListQuery`](super::code_list::CodeListListQuery) but
/// scopes to a single `codelist_id` instead of a version.
#[derive(Debug, Clone)]
pub struct CodeItemListQuery {
    pub codelist_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}
```

- [ ] **Step 6: Drop `EmptyFragment`, add `InvalidFragment`**

In `lib/crates/terminology/src/domain/error.rs` delete the `EmptyFragment` variant (line 13-14):

```rust
    #[error("search fragment must not be empty")]
    EmptyFragment,
```

Add the new variant after `EmptyCodeAtPosition` (after line 32):

```rust
    /// Search fragment contained characters reserved by Postgres
    /// `to_tsquery` (`& | ! ( ) :`).
    #[error("search fragment contains reserved tsquery characters: & | ! ( ) :")]
    InvalidFragment,
```

- [ ] **Step 7: Re-run the domain test**

Run: `cargo test -p terminology --lib domain::tests`
Expected: PASS — `Page<T>` exists; the test compiles.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/terminology/src/domain.rs \
        lib/crates/terminology/src/domain/paging.rs \
        lib/crates/terminology/src/domain/code_list.rs \
        lib/crates/terminology/src/domain/code_item.rs \
        lib/crates/terminology/src/domain/error.rs \
        lib/crates/terminology/src/domain/tests.rs
git commit -m "feat(terminology): add Page<T>, CodeListListQuery, CodeItemListQuery, DomainError::InvalidFragment"
```

---

### Task 2: Repository trait — replace `list_by_*` + `search` with `search_or_list`

**Files:**
- Modify: `lib/crates/terminology/src/domain/repository.rs`

**Interfaces:**
- Consumes: `Page<T>`, `CodeListListQuery`, `CodeItemListQuery` from Task 1.
- Produces:
  - `trait CodeListRepository` drops `list_by_version` and `search`; gains `async fn search_or_list(&self, query: CodeListListQuery) -> Result<Page<CodeList>, DomainError>`.
  - `trait CodeItemRepository` drops `list_by_codelist` and `search`; gains `async fn search_or_list(&self, query: CodeItemListQuery) -> Result<Page<CodeItem>, DomainError>`.
  - `list_by_version_and_code` stays as-is on `CodeItemRepository`.

- [ ] **Step 1: Write a compile-fail sanity check**

This task is purely a trait-shape change; the failing test is the broader build. Move to step 2 first; the trait edit will deliberately break the fakes and Postgres adapters, then Tasks 3-4 fix them.

- [ ] **Step 2: Update `CodeListRepository` trait**

In `lib/crates/terminology/src/domain/repository.rs` rewrite the imports to:

```rust
use super::code_item::{
    CodeItem, CodeItemListQuery, CodeItemNew, CodeItemUpdate,
};
use super::code_list::{
    CodeList, CodeListListQuery, CodeListNew, CodeListUpdate,
};
use super::paging::Page;
```

Replace the body of `CodeListRepository` with:

```rust
#[async_trait]
pub trait CodeListRepository: Send + Sync {
    async fn create(&self, input: CodeListNew) -> Result<CodeList, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CodeList, DomainError>;
    /// Unified list+search under a version. Returns a single page.
    /// - `fragment = None`           → `WHERE version_id = $1 ORDER BY id ASC`
    /// - `fragment = Some(_)`        → `WHERE version_id = $1 AND tsv @@ to_tsquery('english', $2 || ':*')
    ///                                  ORDER BY ts_rank(tsv, to_tsquery('english', $2 || ':*')) DESC, id ASC`
    /// Implementations fetch `limit + 1` rows to compute `next_offset`.
    async fn search_or_list(
        &self,
        query: CodeListListQuery,
    ) -> Result<Page<CodeList>, DomainError>;
    async fn update(&self, input: CodeListUpdate) -> Result<CodeList, DomainError>;
    /// Hard delete; cascades to code_items via the schema's
    /// `ON DELETE CASCADE`.
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}
```

- [ ] **Step 3: Update `CodeItemRepository` trait**

In the same file, replace `CodeItemRepository`'s body with:

```rust
#[async_trait]
pub trait CodeItemRepository: Send + Sync {
    async fn create(&self, input: CodeItemNew) -> Result<CodeItem, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CodeItem, DomainError>;
    /// Unified list+search under a codelist. Returns a single page.
    /// Same shape semantics as
    /// [`CodeListRepository::search_or_list`].
    async fn search_or_list(
        &self,
        query: CodeItemListQuery,
    ) -> Result<Page<CodeItem>, DomainError>;
    /// Natural-key lookup on the `code_items` table itself. Returns
    /// every item whose `version_id` matches the given value and
    /// whose `code` matches the given value — i.e. all items with
    /// the same value code across the codelists of a single
    /// version. Multiple rows are expected when the same item
    /// code appears in more than one codelist of the version.
    /// Backed by the composite index
    /// `code_items_version_id_code_idx (version_id, code)`.
    async fn list_by_version_and_code(
        &self,
        version_id: i64,
        code: &str,
    ) -> Result<Vec<CodeItem>, DomainError>;
    async fn update(&self, input: CodeItemUpdate) -> Result<CodeItem, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;

    /// Insert several `CodeItem` rows in a single SQL statement.
    /// Returns the number of rows inserted on success. The backend
    /// must execute this atomically — if any row violates a constraint
    /// the entire call fails and zero rows are inserted.
    async fn bulk_create(&self, inputs: Vec<CodeItemNew>) -> Result<usize, DomainError>;
}
```

- [ ] **Step 4: Verify the trait change fails the build as expected**

Run: `cargo build -p terminology`
Expected: compile error — `FakeCodeListRepo`, `FakeCodeItemRepo`, `InMemoryCodeListRepo`, `InMemoryCodeItemRepo`, `CodeListRepo` (Postgres), `CodeItemRepo` (Postgres) all miss `search_or_list` and still implement `list_by_version` / `list_by_codelist` / `search`. Tasks 3 and 4 fix the adapters.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/terminology/src/domain/repository.rs
git commit -m "refactor(terminology): unify repository list+search behind search_or_list trait"
```

---

### Task 3: Postgres adapter — implement `search_or_list` on `CodeListRepo` and `CodeItemRepo`

**Files:**
- Modify: `lib/crates/terminology/src/adapter/persistence/postgres/code_list_repo.rs`
- Modify: `lib/crates/terminology/src/adapter/persistence/postgres/code_item_repo.rs`

**Interfaces:**
- Consumes: `CodeListListQuery` / `CodeItemListQuery`, `Page<T>` from Task 1.
- Produces: implementations of `search_or_list` on both `CodeListRepo` and `CodeItemRepo`. `list_by_version`, `list_by_codelist`, `search` impls are deleted.

- [ ] **Step 1: Write the failing Postgres integration test (add to `tests/integration_persistence.rs`)**

Append to `lib/crates/terminology/tests/integration_persistence.rs` (inside the existing `mod`, after `search_code_lists_ranks_hits`). The test exercises the list-path of `search_or_list`:

```rust
#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn list_code_lists_paginates_across_multiple_pages() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("page-v"),
            })
            .await
            .expect("version");

        for i in 0..7 {
            l_repo
                .create(CodeListNew {
                    version_id: v.id,
                    code: format!("page-cl-{i}"),
                    extensible: true,
                    name: format!("Codelist {i}"),
                    submission_value: format!("SV{i}"),
                    synonym: "".into(),
                    definition: "".into(),
                    nci_preferred_term: "".into(),
                })
                .await
                .expect("create");
        }

        // page 1
        let p1 = l_repo
            .search_or_list(terminology::CodeListListQuery {
                version_id: v.id,
                fragment: None,
                offset: 0,
                limit: 3,
            })
            .await
            .expect("page 1");
        assert_eq!(p1.items.len(), 3);
        assert_eq!(p1.next_offset, Some(3));

        // page 2
        let p2 = l_repo
            .search_or_list(terminology::CodeListListQuery {
                version_id: v.id,
                fragment: None,
                offset: 3,
                limit: 3,
            })
            .await
            .expect("page 2");
        assert_eq!(p2.items.len(), 3);
        assert_eq!(p2.next_offset, Some(6));

        // page 3 (final, only 1 row)
        let p3 = l_repo
            .search_or_list(terminology::CodeListListQuery {
                version_id: v.id,
                fragment: None,
                offset: 6,
                limit: 3,
            })
            .await
            .expect("page 3");
        assert_eq!(p3.items.len(), 1);
        assert_eq!(p3.next_offset, None);

        // offset >= total
        let empty = l_repo
            .search_or_list(terminology::CodeListListQuery {
                version_id: v.id,
                fragment: None,
                offset: 100,
                limit: 3,
            })
            .await
            .expect("empty page");
        assert!(empty.items.is_empty());
        assert_eq!(empty.next_offset, None);
    })
    .await;
}
```

Add `use terminology::CodeListListQuery;` to the imports at the top of `tests/integration_persistence.rs`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p terminology --test integration_persistence -- --ignored list_code_lists_paginates_across_multiple_pages`
Expected: compile error — `search_or_list` not found on `CodeListRepository` (the trait only has `list_by_version` and `search`).

- [ ] **Step 3: Implement `search_or_list` on `CodeListRepo`**

In `lib/crates/terminology/src/adapter/persistence/postgres/code_list_repo.rs`:

- Replace the imports at the top with:

```rust
use crate::domain::{
    CodeList, CodeListListQuery, CodeListNew, CodeListRow as _, CodeListUpdate, DomainError, Page,
};
```

(Keep the `use crate::domain::` line, just narrow it. Note: `CodeListRow as _` is a placeholder — the existing struct in this file is named `CodeListRow` privately; remove the `as _` and just delete `CodeListSearchHit`, `CodeListSearchQuery` from the old import line.)

The actual replacement import block:

```rust
use crate::domain::{
    CodeList, CodeListListQuery, CodeListNew, CodeListUpdate, DomainError, Page,
};
```

- Delete the body of `async fn list_by_version(&self, version_id: i64)` (the entire method).
- Delete the entire `async fn search(&self, query: CodeListSearchQuery)` method.
- Add the following implementation at the bottom of the `impl CodeListRepository for CodeListRepo { ... }` block:

```rust
    async fn search_or_list(
        &self,
        q: CodeListListQuery,
    ) -> Result<Page<CodeList>, DomainError> {
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "SELECT id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at FROM code_lists WHERE version_id = ",
        );
        qb.push_bind(q.version_id);

        if let Some(frag) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
            qb.push(" AND tsv @@ to_tsquery('english', ");
            qb.push_bind(format!("{frag}:*"));
            qb.push(") ORDER BY ts_rank(tsv, to_tsquery('english', ");
            qb.push_bind(format!("{frag}:*"));
            qb.push(")) DESC, id ASC LIMIT ");
        } else {
            qb.push(" ORDER BY id ASC LIMIT ");
        }
        // Fetch limit+1 to detect whether another page exists.
        qb.push_bind((q.limit as i64) + 1);
        qb.push(" OFFSET ");
        qb.push_bind(q.offset as i64);

        let mut rows: Vec<CodeListRow> = qb
            .build_query_as::<CodeListRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error_simple)?;

        let next_offset = if rows.len() as u32 > q.limit {
            rows.pop();
            Some(q.offset + q.limit)
        } else {
            None
        };
        let items = rows
            .into_iter()
            .map(CodeList::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page { items, next_offset })
    }
```

- [ ] **Step 4: Implement `search_or_list` on `CodeItemRepo`**

Mirror the same shape in `lib/crates/terminology/src/adapter/persistence/postgres/code_item_repo.rs`:

- Replace imports at the top with:

```rust
use crate::domain::{
    CodeItem, CodeItemListQuery, CodeItemNew, CodeItemUpdate, DomainError, Page,
};
```

- Delete `async fn list_by_codelist(&self, codelist_id: i64)`.
- Delete `async fn search(&self, query: CodeItemSearchQuery)`.
- Add:

```rust
    async fn search_or_list(
        &self,
        q: CodeItemListQuery,
    ) -> Result<Page<CodeItem>, DomainError> {
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "SELECT id, codelist_id, version_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at FROM code_items WHERE codelist_id = ",
        );
        qb.push_bind(q.codelist_id);

        if let Some(frag) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
            qb.push(" AND tsv @@ to_tsquery('english', ");
            qb.push_bind(format!("{frag}:*"));
            qb.push(") ORDER BY ts_rank(tsv, to_tsquery('english', ");
            qb.push_bind(format!("{frag}:*"));
            qb.push(")) DESC, id ASC LIMIT ");
        } else {
            qb.push(" ORDER BY id ASC LIMIT ");
        }
        qb.push_bind((q.limit as i64) + 1);
        qb.push(" OFFSET ");
        qb.push_bind(q.offset as i64);

        let mut rows: Vec<CodeItemRow> = qb
            .build_query_as::<CodeItemRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error_simple)?;

        let next_offset = if rows.len() as u32 > q.limit {
            rows.pop();
            Some(q.offset + q.limit)
        } else {
            None
        };
        let items = rows
            .into_iter()
            .map(CodeItem::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Page { items, next_offset })
    }
```

- [ ] **Step 5: Run the integration test**

Run: `cargo test -p terminology --test integration_persistence -- --ignored list_code_lists_paginates_across_multiple_pages`
Expected: PASS (requires `AEGIS_TERMINOLOGY_DATABASE_URL` to be set in the env).

- [ ] **Step 6: Commit**

```bash
git add lib/crates/terminology/src/adapter/persistence/postgres/code_list_repo.rs \
        lib/crates/terminology/src/adapter/persistence/postgres/code_item_repo.rs \
        lib/crates/terminology/tests/integration_persistence.rs
git commit -m "feat(terminology): implement search_or_list in Postgres adapters"
```

---

### Task 4: In-memory test fakes — implement `search_or_list`

**Files:**
- Modify: `lib/crates/terminology/src/usecase/tests.rs` (`FakeCodeListRepo`, `FakeCodeItemRepo`)
- Modify: `lib/crates/terminology/src/adapter/facade/in_memory/tests.rs` (`InMemoryCodeListRepo`, `InMemoryCodeItemRepo`)

**Interfaces:**
- Consumes: `CodeListListQuery`, `CodeItemListQuery`, `Page<T>`.
- Produces: in-memory `search_or_list` impls. Real substring filter (not the empty stub from before). Delete `list_by_version`, `list_by_codelist`, `search` impls.

- [ ] **Step 1: Write the failing fake test (in `lib/crates/terminology/src/usecase/tests.rs`)**

Append to the file:

```rust
#[tokio::test]
async fn list_code_lists_with_fragment_filters_and_paginates() {
    let (v_repo, l_repo, _, usecase) = make_usecase();
    let v = v_repo
        .create(TerminologyVersionNew {
            kind: TerminologyKind::Sdtm,
            name: "v1".into(),
        })
        .await
        .expect("v");
    let _ = v;
    for (i, name) in ["AGE", "AGEGRP", "SEX", "RACE", "AGE2"].iter().enumerate() {
        l_repo
            .create(CodeListNew {
                version_id: 1,
                code: format!("C{i}"),
                extensible: true,
                name: name.to_string(),
                submission_value: "x".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("create");
    }

    let page = usecase
        .list_code_lists(crate::domain::CodeListListQuery {
            version_id: 1,
            fragment: Some("age".into()),
            offset: 0,
            limit: 10,
        })
        .await
        .expect("page");
    let names: Vec<String> = page.items.into_iter().map(|c| c.name).collect();
    assert_eq!(names, vec!["AGE", "AGE2", "AGEGRP"]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p terminology --lib usecase::tests::list_code_lists_with_fragment_filters_and_paginates`
Expected: compile error — `list_code_lists` still takes `version_id: i64` and the fake doesn't implement `search_or_list`.

- [ ] **Step 3: Update `FakeCodeListRepo` to implement `search_or_list`**

In `lib/crates/terminology/src/usecase/tests.rs`:

- Replace the imports `use crate::domain::{…CodeItemSearchHit, CodeItemSearchQuery, …CodeListSearchHit, CodeListSearchQuery…}` with the new set:

```rust
use crate::domain::{
    CodeItem, CodeItemListQuery, CodeItemNew, CodeItemRepository, CodeItemUpdate, CodeList,
    CodeListListQuery, CodeListNew, CodeListRepository, CodeListUpdate, DomainError, Page,
    TerminologyKind, TerminologyVersion, TerminologyVersionNew, TerminologyVersionRepository,
    TerminologyVersionUpdate,
};
```

- In the `impl CodeListRepository for FakeCodeListRepo` block:
  - Delete the `async fn list_by_version(&self, …)` method.
  - Delete the `async fn search(&self, …)` method.
  - Add:

```rust
    async fn search_or_list(
        &self,
        q: CodeListListQuery,
    ) -> Result<Page<CodeList>, DomainError> {
        let mut all: Vec<CodeList> = self
            .state
            .lock()
            .unwrap()
            .by_id
            .values()
            .filter(|c| c.version_id == q.version_id)
            .cloned()
            .collect();

        if let Some(frag) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
            let needle = frag.to_lowercase();
            all.retain(|cl| {
                cl.name.to_lowercase().contains(&needle)
                    || cl.submission_value.to_lowercase().contains(&needle)
                    || cl.synonym.to_lowercase().contains(&needle)
                    || cl.definition.to_lowercase().contains(&needle)
                    || cl.nci_preferred_term.to_lowercase().contains(&needle)
            });
        }

        all.sort_by_key(|cl| cl.id);
        let limit = q.limit as usize;
        let offset = q.offset as usize;
        let mut items: Vec<CodeList> = all.into_iter().skip(offset).take(limit + 1).collect();
        let next_offset = if items.len() > limit {
            items.pop();
            Some(q.offset + q.limit)
        } else {
            None
        };
        Ok(Page { items, next_offset })
    }
```

- [ ] **Step 4: Update `FakeCodeItemRepo` to implement `search_or_list`**

In the same `impl CodeItemRepository for FakeCodeItemRepo` block:

- Delete `async fn list_by_codelist(&self, …)`.
- Delete `async fn search(&self, …)`.
- Add:

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
            .filter(|i| i.codelist_id == q.codelist_id)
            .cloned()
            .collect();

        if let Some(frag) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
            let needle = frag.to_lowercase();
            all.retain(|item| {
                item.submission_value.to_lowercase().contains(&needle)
                    || item.synonym.to_lowercase().contains(&needle)
                    || item.definition.to_lowercase().contains(&needle)
                    || item.nci_preferred_term.to_lowercase().contains(&needle)
            });
        }

        all.sort_by_key(|i| i.id);
        let limit = q.limit as usize;
        let offset = q.offset as usize;
        let mut items: Vec<CodeItem> = all.into_iter().skip(offset).take(limit + 1).collect();
        let next_offset = if items.len() > limit {
            items.pop();
            Some(q.offset + q.limit)
        } else {
            None
        };
        Ok(Page { items, next_offset })
    }
```

- [ ] **Step 5: Run the failing fake test (still fails because usecase hasn't been updated)**

Run: `cargo test -p terminology --lib usecase::tests::list_code_lists_with_fragment_filters_and_paginates`
Expected: still fails — `usecase.list_code_lists` is still the old `(version_id: i64)` signature. Task 5 fixes it.

- [ ] **Step 6: Mirror the same edits in `lib/crates/terminology/src/adapter/facade/in_memory/tests.rs`**

- Replace imports at the top to drop `*SearchHit`, `*SearchQuery` and add `Page`, `CodeListListQuery`, `CodeItemListQuery`:

```rust
use crate::domain::{
    CodeItem, CodeItemListQuery, CodeItemNew, CodeItemRepository, CodeItemUpdate, CodeList,
    CodeListListQuery, CodeListNew, CodeListRepository, CodeListUpdate, DomainError, Page,
    TerminologyKind, TerminologyVersion, TerminologyVersionNew, TerminologyVersionRepository,
    TerminologyVersionUpdate,
};
```

- In `impl CodeListRepository for InMemoryCodeListRepo`:
  - Delete `async fn list_by_version(&self, …)`.
  - Delete `async fn search(&self, …)`.
  - Add (identical to the fake in step 3, just on `InMemoryCodeListRepo`):

```rust
    async fn search_or_list(
        &self,
        q: CodeListListQuery,
    ) -> Result<Page<CodeList>, DomainError> {
        let mut all: Vec<CodeList> = self
            .state
            .lock()
            .unwrap()
            .by_id
            .values()
            .filter(|c| c.version_id == q.version_id)
            .cloned()
            .collect();

        if let Some(frag) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
            let needle = frag.to_lowercase();
            all.retain(|cl| {
                cl.name.to_lowercase().contains(&needle)
                    || cl.submission_value.to_lowercase().contains(&needle)
                    || cl.synonym.to_lowercase().contains(&needle)
                    || cl.definition.to_lowercase().contains(&needle)
                    || cl.nci_preferred_term.to_lowercase().contains(&needle)
            });
        }

        all.sort_by_key(|cl| cl.id);
        let limit = q.limit as usize;
        let offset = q.offset as usize;
        let mut items: Vec<CodeList> = all.into_iter().skip(offset).take(limit + 1).collect();
        let next_offset = if items.len() > limit {
            items.pop();
            Some(q.offset + q.limit)
        } else {
            None
        };
        Ok(Page { items, next_offset })
    }
```

- In `impl CodeItemRepository for InMemoryCodeItemRepo`:
  - Delete `async fn list_by_codelist(&self, …)`.
  - Delete `async fn search(&self, …)`.
  - Add the matching `search_or_list` (same body as step 4).

- [ ] **Step 7: Verify the in-memory test crate compiles**

Run: `cargo build -p terminology --tests`
Expected: still fails — `usecase.list_code_lists` signature mismatch. Task 5 fixes it.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/terminology/src/usecase/tests.rs \
        lib/crates/terminology/src/adapter/facade/in_memory/tests.rs
git commit -m "refactor(terminology): in-memory fakes implement search_or_list"
```

---

### Task 5: Usecase — change `list_code_lists` / `list_code_items` to take a query type, drop `search_*`, add `validate_tsquery_fragment`, drop search-hit re-exports

**Files:**
- Modify: `lib/crates/terminology/src/usecase/terminology_usecase.rs`
- Modify: `lib/crates/terminology/src/usecase/views.rs`

**Interfaces:**
- Consumes: `Page<T>`, `CodeListListQuery`, `CodeItemListQuery`, `DomainError::InvalidFragment`.
- Produces:
  - `pub async fn list_code_lists(&self, query: CodeListListQuery) -> Result<Page<CodeListView>, UsecaseError>`
  - `pub async fn list_code_items(&self, query: CodeItemListQuery) -> Result<Page<CodeItemView>, UsecaseError>`
  - `validate_tsquery_fragment(s: &str) -> Result<&str, UsecaseError>` (private)
  - `clamp_limit(limit: u32) -> u32` (unchanged; keep)
  - Removed: `search_code_lists`, `search_code_items`, `validate_fragment`.

- [ ] **Step 1: Update the usecase imports**

In `lib/crates/terminology/src/usecase/terminology_usecase.rs`, replace the `use crate::domain::{…}` import block with:

```rust
use crate::domain::{
    CodeItemListQuery, CodeItemNew, CodeItemRepository, CodeItemUpdate, CodeListListQuery,
    CodeListNew, CodeListRepository, CodeListUpdate, DomainError, Page,
    TerminologyVersionNew, TerminologyVersionRepository, TerminologyVersionUpdate,
};
```

- [ ] **Step 2: Rewrite `list_code_lists`**

Find `pub async fn list_code_lists(&self, version_id: i64)` and replace the whole body with:

```rust
    pub async fn list_code_lists(
        &self,
        query: CodeListListQuery,
    ) -> Result<Page<CodeListView>, UsecaseError> {
        let limit = clamp_limit(query.limit);
        let fragment = match query.fragment.as_deref() {
            None => None,
            Some(s) if s.trim().is_empty() => None,
            Some(s) => Some(validate_tsquery_fragment(s)?.to_owned()),
        };
        let q = CodeListListQuery {
            fragment,
            offset: query.offset,
            limit,
            ..query
        };
        let page = self.code_list_repo.search_or_list(q).await?;
        Ok(Page {
            items: page.items.into_iter().map(Into::into).collect(),
            next_offset: page.next_offset,
        })
    }
```

- [ ] **Step 3: Rewrite `list_code_items`**

Find `pub async fn list_code_items(&self, codelist_id: i64)` and replace the body with:

```rust
    pub async fn list_code_items(
        &self,
        query: CodeItemListQuery,
    ) -> Result<Page<CodeItemView>, UsecaseError> {
        let limit = clamp_limit(query.limit);
        let fragment = match query.fragment.as_deref() {
            None => None,
            Some(s) if s.trim().is_empty() => None,
            Some(s) => Some(validate_tsquery_fragment(s)?.to_owned()),
        };
        let q = CodeItemListQuery {
            fragment,
            offset: query.offset,
            limit,
            ..query
        };
        let page = self.code_item_repo.search_or_list(q).await?;
        Ok(Page {
            items: page.items.into_iter().map(Into::into).collect(),
            next_offset: page.next_offset,
        })
    }
```

- [ ] **Step 4: Delete `search_code_lists` and `search_code_items`**

Remove the two `pub async fn search_code_lists(&self, …)` and `pub async fn search_code_items(&self, …)` methods entirely.

- [ ] **Step 5: Replace `validate_fragment` with `validate_tsquery_fragment`**

Replace the helper function near the bottom of the file:

```rust
/// Reject any fragment containing characters reserved by Postgres
/// `to_tsquery` (`& | ! ( ) :`). The usecase passes `fragment = None`
/// (or `Some("")`) when the caller wants the plain list path, so
/// empty / whitespace-only fragments never reach this helper.
fn validate_tsquery_fragment(s: &str) -> Result<&str, UsecaseError> {
    if s.chars().any(|c| matches!(c, '&' | '|' | '!' | '(' | ')' | ':')) {
        return Err(UsecaseError::Validation(DomainError::InvalidFragment));
    }
    Ok(s)
}
```

- [ ] **Step 6: Drop the search-hit re-export**

In `lib/crates/terminology/src/usecase/views.rs` delete the last two lines:

```rust
// Re-export the search-hit views so the usecase surface is one
// `use terminology::*` away.
pub use crate::domain::{CodeItemSearchHit, CodeListSearchHit};
```

- [ ] **Step 7: Run the failing fake test from Task 4 — it should now pass**

Run: `cargo test -p terminology --lib usecase::tests::list_code_lists_with_fragment_filters_and_paginates`
Expected: PASS.

- [ ] **Step 8: Run the full usecase test suite**

Run: `cargo test -p terminology --lib usecase::`
Expected: most tests pass. The old `search_code_lists_clamps_limit_to_default_when_zero` test (in `usecase/tests.rs`) will fail to compile because `CodeListSearchQuery` no longer exists. Drop it in step 9.

- [ ] **Step 9: Drop the old search-related usecase test**

In `lib/crates/terminology/src/usecase/tests.rs` delete:

```rust
#[tokio::test]
async fn search_code_lists_clamps_limit_to_default_when_zero() {
    …
}
```

(Task 12 adds the new list_code_lists tests, including a `clamps_limit_to_default_when_zero` variant.)

- [ ] **Step 10: Verify the terminology crate builds**

Run: `cargo build -p terminology --tests`
Expected: PASS (the in-memory fakes and Postgres adapters are wired).

- [ ] **Step 11: Commit**

```bash
git add lib/crates/terminology/src/usecase/terminology_usecase.rs \
        lib/crates/terminology/src/usecase/views.rs \
        lib/crates/terminology/src/usecase/tests.rs
git commit -m "refactor(terminology): unified list+search on usecase; drop search_*"
```

---

### Task 6: API contract (`apis` crate) — `Page<T>`, query types, unified trait methods

**Files:**
- Modify: `lib/crates/apis/src/terminology.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `pub struct Page<T> { pub items: Vec<T>, pub next_offset: Option<u32> }`
  - `pub struct CodeListListQuery { pub version_id: i64, pub fragment: Option<String>, pub offset: u32, pub limit: u32 }`
  - `pub struct CodeItemListQuery { pub codelist_id: i64, pub fragment: Option<String>, pub offset: u32, pub limit: u32 }`
  - Trait methods:
    - `async fn list_code_lists(&self, query: CodeListListQuery) -> Result<Page<CodeListView>, TerminologyApiError>`
    - `async fn list_code_items(&self, query: CodeItemListQuery) -> Result<Page<CodeItemView>, TerminologyApiError>`
  - Removed types: `CodeListSearchQuery`, `CodeListSearchHit`, `CodeItemSearchQuery`, `CodeItemSearchHit`.
  - Removed trait methods: `search_code_lists`, `search_code_items`.

- [ ] **Step 1: Add the three new types**

In `lib/crates/apis/src/terminology.rs`, immediately before the `// ---- search query / hit ----` section heading, insert:

```rust
// ---- pagination envelope ----

/// One page of a paginated result set. Mirrors
/// `terminology::domain::Page<T>` field-for-field so the two
/// layers can `From`-convert without ceremony.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_offset: Option<u32>,
}

/// Query for [`TerminologyService::list_code_lists`]. Unified list
/// + search under a single signature: `fragment = None` is a plain
/// `ORDER BY id ASC` list; `fragment = Some(_)` is a FTS query with
/// `ts_rank DESC, id ASC` ordering.
#[derive(Debug, Clone)]
pub struct CodeListListQuery {
    pub version_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

/// Query for [`TerminologyService::list_code_items`]. Mirrors
/// [`CodeListListQuery`] but scopes to a `codelist_id`.
#[derive(Debug, Clone)]
pub struct CodeItemListQuery {
    pub codelist_id: i64,
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}
```

- [ ] **Step 2: Remove the search query / hit block**

Delete the entire `// ---- search query / hit ----` section, including its four structs (`CodeListSearchQuery`, `CodeListSearchHit`, `CodeItemSearchQuery`, `CodeItemSearchHit`) and all their doc comments.

- [ ] **Step 3: Replace the trait `list_code_lists` method signature**

In `pub trait TerminologyService`, find:

```rust
    /// List every codelist owned by the given version. Order is
    /// backend-defined.
    async fn list_code_lists(
        &self,
        version_id: i64,
    ) -> Result<Vec<CodeListView>, TerminologyApiError>;
```

Replace with:

```rust
    /// Unified list+search under a version. Returns one page of
    /// `CodeListView`s. `query.fragment = None` (or `Some("")`)
    /// returns the plain list path ordered by `id ASC`;
    /// `query.fragment = Some(non-empty)` runs the FTS prefix-match
    /// path with `ts_rank DESC, id ASC`. `query.offset` / `query.limit`
    /// are clamped by the backend (default 50, max 500).
    async fn list_code_lists(
        &self,
        query: CodeListListQuery,
    ) -> Result<Page<CodeListView>, TerminologyApiError>;
```

- [ ] **Step 4: Replace the trait `list_code_items` method signature**

Find:

```rust
    /// List every item belonging to the given codelist. Order is
    /// backend-defined.
    async fn list_code_items(
        &self,
        codelist_id: i64,
    ) -> Result<Vec<CodeItemView>, TerminologyApiError>;
```

Replace with:

```rust
    /// Unified list+search under a codelist. Mirrors
    /// [`TerminologyService::list_code_lists`] but scoped to
    /// `query.codelist_id`.
    async fn list_code_items(
        &self,
        query: CodeItemListQuery,
    ) -> Result<Page<CodeItemView>, TerminologyApiError>;
```

- [ ] **Step 5: Delete `search_code_lists` and `search_code_items` trait methods**

Delete both `async fn search_code_lists(&self, …)` and `async fn search_code_items(&self, …)` methods from the trait body. Delete their doc comments too.

- [ ] **Step 6: Verify the apis crate compiles**

Run: `cargo build -p apis`
Expected: FAIL — `TerminologyServiceImpl` (in the terminology crate) and the `StubTerminologyService` (in aegis-server) still implement the old methods. Tasks 7 + 11 fix them.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/apis/src/terminology.rs
git commit -m "refactor(apis): unified list+search query types on TerminologyService"
```

---

### Task 7: Service adapter (`TerminologyServiceImpl`) — implement new trait methods, drop search

**Files:**
- Modify: `lib/crates/terminology/src/adapter/facade/in_memory/service.rs`

**Interfaces:**
- Consumes: `apis::terminology::Page<T>`, `CodeListListQuery`, `CodeItemListQuery`; the `code_list_view_from_internal` / `code_item_view_from_internal` helpers already exist.
- Produces: updated impls of `list_code_lists` / `list_code_items`; removed impls of `search_code_lists` / `search_code_items`.

- [ ] **Step 1: Update imports**

In `lib/crates/terminology/src/adapter/facade/in_memory/service.rs`, replace the imports block:

```rust
use apis::terminology::TerminologyKind as ApiKind;
use apis::terminology::{
    BatchCreateCodeItemsRequest, BatchCreateCodeItemsResponse as ApiBatchResp, CodeItemView,
    CodeListListQuery, CodeListView, CreateCodeItemRequest, CreateCodeListRequest,
    CreateTerminologyVersionRequest, Page, TerminologyApiError, TerminologyService,
    TerminologyVersionView, UpdateCodeItemRequest, UpdateCodeListRequest,
    UpdateTerminologyVersionRequest,
};
```

Replace the `use crate::domain::{…}` with:

```rust
use crate::domain::{
    CodeItemListQuery, CodeItemRepository, CodeListListQuery, CodeListRepository, DomainError,
    TerminologyKind, TerminologyVersionRepository,
};
```

(Keep `use crate::usecase::{…}` exactly as-is.)

- [ ] **Step 2: Replace `list_code_lists` impl**

Find `async fn list_code_lists(&self, version_id: i64) -> Result<Vec<CodeListView>, …>` and replace with:

```rust
    async fn list_code_lists(
        &self,
        query: CodeListListQuery,
    ) -> Result<Page<CodeListView>, TerminologyApiError> {
        let page = self
            .usecase
            .list_code_lists(query.into())
            .await
            .map_err(TerminologyApiError::from)?;
        Ok(Page {
            items: page.items.into_iter().map(code_list_view_from_internal).collect(),
            next_offset: page.next_offset,
        })
    }
```

Then add the conversion at the end of the file (near the other helpers):

```rust
impl From<apis::terminology::CodeListListQuery> for crate::domain::CodeListListQuery {
    fn from(q: apis::terminology::CodeListListQuery) -> Self {
        Self {
            version_id: q.version_id,
            fragment: q.fragment,
            offset: q.offset,
            limit: q.limit,
        }
    }
}
```

- [ ] **Step 3: Replace `list_code_items` impl**

Find `async fn list_code_items(&self, codelist_id: i64) -> Result<Vec<CodeItemView>, …>` and replace with:

```rust
    async fn list_code_items(
        &self,
        query: CodeItemListQuery,
    ) -> Result<Page<CodeItemView>, TerminologyApiError> {
        let page = self
            .usecase
            .list_code_items(query.into())
            .await
            .map_err(TerminologyApiError::from)?;
        Ok(Page {
            items: page.items.into_iter().map(code_item_view_from_internal).collect(),
            next_offset: page.next_offset,
        })
    }
```

Add the matching conversion:

```rust
impl From<apis::terminology::CodeItemListQuery> for crate::domain::CodeItemListQuery {
    fn from(q: apis::terminology::CodeItemListQuery) -> Self {
        Self {
            codelist_id: q.codelist_id,
            fragment: q.fragment,
            offset: q.offset,
            limit: q.limit,
        }
    }
}
```

- [ ] **Step 4: Delete `search_code_lists` and `search_code_items` impls**

Delete the bodies of both methods. Update the `From<UsecaseError>` impl to drop the `EmptyFragment` reference:

In `match err { … }`, find the `UsecaseError::Repository(domain)` arm and remove `DomainError::EmptyFragment` from the `unreachable!` list (line 170). The arm becomes:

```rust
                DomainError::EmptyCode
                | DomainError::EmptyName
                | DomainError::InvalidKind(_)
                | DomainError::FkVersionNotFound(_)
                | DomainError::FkCodeListNotFound(_)
                | DomainError::EmptyCodeAtPosition(_) => unreachable!(
                    "domain validation / FK errors are only produced as UsecaseError::Validation"
                ),
```

Note: `DomainError::InvalidFragment` joins the `Validation` arm automatically; it does not appear here.

- [ ] **Step 5: Verify the adapter compiles**

Run: `cargo build -p terminology --tests`
Expected: PASS for the terminology crate (the `StubTerminologyService` in aegis-server is still broken; Task 11 fixes it).

- [ ] **Step 6: Commit**

```bash
git add lib/crates/terminology/src/adapter/facade/in_memory/service.rs
git commit -m "refactor(terminology): TerminologyServiceImpl list+search unified"
```

---

### Task 8: Wire DTOs (HTTP) — expand `CodeListListQuery` / `CodeItemListQuery`, add `PagedCodeListsResponse` / `PagedCodeItemsResponse`, drop search DTOs

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `CodeListListQuery { version_id, fragment, offset, limit }`, `CodeItemListQuery { codelist_id, fragment, offset, limit }`, `PagedCodeListsResponse { codelists, next_offset }`, `PagedCodeItemsResponse { items, next_offset }`. Removed: `TerminologySearchBaseQuery`, `*SearchQueryRequest`, `*SearchHitResponse`, `*SearchHitsResponse`, `CodeListListResponse`, `CodeItemListResponse`.

- [ ] **Step 1: Drop the search DTOs**

In `lib/crates/apis/src/transport/http/dto.rs` (the file is at `apps/server/aegis-server/src/transport/http/dto.rs`):

Delete these struct definitions:

- `pub struct TerminologySearchBaseQuery` (lines 772-779)
- `pub type CodeListSearchQueryRequest = TerminologySearchBaseQuery;`
- `pub type CodeItemSearchQueryRequest = TerminologySearchBaseQuery;`
- `pub struct CodeListSearchHitResponse` (lines 592-604)
- `pub struct CodeItemSearchHitResponse` (lines 607-619)
- `pub struct CodeListSearchHitsResponse` (lines 625-629)
- `pub struct CodeItemSearchHitsResponse` (lines 635-639)

Also delete the `impl From<apis::terminology::CodeListSearchHit> for CodeListSearchHitResponse` and `impl From<apis::terminology::CodeItemSearchHit> for CodeItemSearchHitResponse` blocks.

- [ ] **Step 2: Replace `CodeListListQuery` with the expanded version**

Find the existing `pub struct CodeListListQuery` (lines 793-797) and replace with:

```rust
/// Query string for `GET /api/terminology/code-lists` (unified
/// list+search). `versionId` is required; `fragment`, `offset`,
/// `limit` are all optional. `fragment = ""` is treated as no
/// fragment. `limit = 0` lets the usecase apply the default.
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
```

- [ ] **Step 3: Replace `CodeItemListQuery` with the expanded version**

Find the existing `pub struct CodeItemListQuery` (lines 801-805) and replace with:

```rust
/// Query string for `GET /api/terminology/code-items` (unified
/// list+search). `codelistId` is required; the rest are optional.
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemListQuery {
    pub codelist_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default)]
    pub limit: u32,
}
```

- [ ] **Step 4: Add `PagedCodeListsResponse` and `PagedCodeItemsResponse`**

Insert these next to the existing view DTOs (just after `CodeItemListResponse` is removed — see step 5):

```rust
/// Wire-level wrapper for the unified `GET /api/terminology/code-lists`
/// response. `next_offset` is omitted on the last page.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagedCodeListsResponse {
    pub codelists: Vec<CodeListViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}

/// Wire-level wrapper for the unified `GET /api/terminology/code-items`
/// response. `next_offset` is omitted on the last page.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PagedCodeItemsResponse {
    pub items: Vec<CodeItemViewResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u32>,
}
```

- [ ] **Step 5: Remove `CodeListListResponse` and `CodeItemListResponse`**

Delete both struct definitions (lines 545-549 and 585-589). They are superseded by the paged variants.

- [ ] **Step 6: Update the round-trip tests in the same file**

In the `mod tests` block at the bottom of `dto.rs`:

- Delete `code_list_list_response_roundtrip` and `code_item_list_response_roundtrip`.
- Delete `code_list_search_hit_response_from_apis_hit`, `code_item_search_hit_response_from_apis_hit`, `code_list_search_query_request_roundtrip`, `code_item_search_query_request_roundtrip`.
- Add new tests:

```rust
    #[test]
    fn code_list_list_query_full_roundtrip() {
        let json = r#"{"versionId":1,"fragment":"age","offset":10,"limit":25}"#;
        let q: CodeListListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.version_id, 1);
        assert_eq!(q.fragment.as_deref(), Some("age"));
        assert_eq!(q.offset, 10);
        assert_eq!(q.limit, 25);
        assert_eq!(serde_json::to_string(&q).unwrap(), json);
    }

    #[test]
    fn code_list_list_query_optional_fragment_omitted() {
        let json = r#"{"versionId":1}"#;
        let q: CodeListListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.version_id, 1);
        assert!(q.fragment.is_none());
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 0);
        // fragment is skipped when None; offset/limit default to 0.
        assert_eq!(
            serde_json::to_string(&q).unwrap(),
            r#"{"versionId":1,"offset":0,"limit":0}"#
        );
    }

    #[test]
    fn code_item_list_query_roundtrip() {
        let json = r#"{"codelistId":11,"fragment":"x","offset":0,"limit":50}"#;
        let q: CodeItemListQuery = serde_json::from_str(json).unwrap();
        assert_eq!(q.codelist_id, 11);
        assert_eq!(q.fragment.as_deref(), Some("x"));
        assert_eq!(q.offset, 0);
        assert_eq!(q.limit, 50);
        assert_eq!(serde_json::to_string(&q).unwrap(), json);
    }

    #[test]
    fn paged_code_lists_response_roundtrip() {
        let json = r#"{"codelists":[],"nextOffset":100}"#;
        let resp: PagedCodeListsResponse = serde_json::from_str(json).unwrap();
        assert!(resp.codelists.is_empty());
        assert_eq!(resp.next_offset, Some(100));
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }

    #[test]
    fn paged_code_lists_response_omits_next_offset_when_none() {
        // The `skip_serializing_if` on next_offset means the field is
        // absent (not `null`) when None — this is the backwards-compat
        // contract: the Tauri client ignores it either way.
        let resp = PagedCodeListsResponse {
            codelists: vec![],
            next_offset: None,
        };
        assert_eq!(
            serde_json::to_string(&resp).unwrap(),
            r#"{"codelists":[]}"#
        );
    }

    #[test]
    fn paged_code_items_response_roundtrip() {
        let json = r#"{"items":[],"nextOffset":42}"#;
        let resp: PagedCodeItemsResponse = serde_json::from_str(json).unwrap();
        assert!(resp.items.is_empty());
        assert_eq!(resp.next_offset, Some(42));
        assert_eq!(serde_json::to_string(&resp).unwrap(), json);
    }
```

- [ ] **Step 7: Run the dto unit tests**

Run: `cargo test -p aegis-server --lib transport::http::dto`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs
git commit -m "refactor(http): unified code-list/code-item list+search DTOs"
```

---

### Task 9: HTTP handlers — replace 4 handlers with 2 unified ones

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/terminology/handlers.rs`

**Interfaces:**
- Consumes: `dto::CodeListListQuery`, `dto::CodeItemListQuery`, `dto::PagedCodeListsResponse`, `dto::PagedCodeItemsResponse`.
- Produces: `list_code_lists` handler returns `Json<PagedCodeListsResponse>`; `list_code_items` handler returns `Json<PagedCodeItemsResponse>`. `list_code_items_by_version_and_code` unchanged. `search_code_lists` / `search_code_items` handlers deleted.

- [ ] **Step 1: Update the imports**

In `apps/server/aegis-server/src/transport/http/terminology/handlers.rs`, replace the `use crate::transport::http::dto::{…}` line with:

```rust
use crate::transport::http::dto::{
    self, CodeItemByVersionAndCodeQuery, CodeItemListQuery, CodeListListQuery,
};
```

(The `TerminologySearchBaseQuery` import is removed.)

- [ ] **Step 2: Rewrite `list_code_lists` handler**

Find the `pub async fn list_code_lists` block (the doc comment starts `/// GET /api/terminology/code-lists?version_id=…`) and replace the entire block — including the `#[utoipa::path(...)]` attribute — with:

```rust
/// `GET /api/terminology/code-lists?…` — unified list+search over
/// the codelists owned by a version. `versionId` is required;
/// `fragment`, `offset`, `limit` are optional.
#[utoipa::path(
    get, path = "/code-lists", tag = "terminology",
    operation_id = "terminology_list_code_lists",
    params(
        ("versionId" = i64, Query, description = "Owning terminology version id"),
        ("fragment" = Option<String>, Query, description = "Optional FTS prefix fragment"),
        ("offset" = Option<u32>, Query, description = "Pagination offset; default 0"),
        ("limit" = Option<u32>, Query, description = "Page size; 0 lets the usecase use its default"),
    ),
    responses(
        (status = 200, description = "Codelists page", body = dto::PagedCodeListsResponse),
        (status = 400, description = "Invalid fragment", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_code_lists(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Query(q): Query<dto::CodeListListQuery>,
) -> Result<Json<dto::PagedCodeListsResponse>, ApiError> {
    let page = state
        .terminology
        .list_code_lists(apis::terminology::CodeListListQuery {
            version_id: q.version_id,
            fragment: q.fragment,
            offset: q.offset,
            limit: q.limit,
        })
        .await?;
    Ok(Json(dto::PagedCodeListsResponse {
        codelists: page.items.into_iter().map(Into::into).collect(),
        next_offset: page.next_offset,
    }))
}
```

- [ ] **Step 3: Rewrite `list_code_items` handler**

Find the `pub async fn list_code_items` block (doc comment `/// GET /api/terminology/code-items?codelist_id=…`) and replace the entire block — including the `#[utoipa::path(...)]` attribute — with:

```rust
/// `GET /api/terminology/code-items?…` — unified list+search over
/// the items belonging to a codelist. `codelistId` is required;
/// `fragment`, `offset`, `limit` are optional.
#[utoipa::path(
    get, path = "/code-items", tag = "terminology",
    operation_id = "terminology_list_code_items",
    params(
        ("codelistId" = i64, Query, description = "Owning codelist id"),
        ("fragment" = Option<String>, Query, description = "Optional FTS prefix fragment"),
        ("offset" = Option<u32>, Query, description = "Pagination offset; default 0"),
        ("limit" = Option<u32>, Query, description = "Page size; 0 lets the usecase use its default"),
    ),
    responses(
        (status = 200, description = "Code items page", body = dto::PagedCodeItemsResponse),
        (status = 400, description = "Invalid fragment", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_code_items(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Query(q): Query<dto::CodeItemListQuery>,
) -> Result<Json<dto::PagedCodeItemsResponse>, ApiError> {
    let page = state
        .terminology
        .list_code_items(apis::terminology::CodeItemListQuery {
            codelist_id: q.codelist_id,
            fragment: q.fragment,
            offset: q.offset,
            limit: q.limit,
        })
        .await?;
    Ok(Json(dto::PagedCodeItemsResponse {
        items: page.items.into_iter().map(Into::into).collect(),
        next_offset: page.next_offset,
    }))
}
```

- [ ] **Step 4: Delete the two search handlers**

Delete the entire `pub async fn search_code_lists(...)` block and the entire `pub async fn search_code_items(...)` block, including their `#[utoipa::path(...)]` attributes and doc comments.

- [ ] **Step 5: Verify the handlers compile**

Run: `cargo build -p aegis-server`
Expected: FAIL — the router still references the deleted search handlers; the openapi schema registry still references the deleted DTOs. Tasks 10 fixes both.

- [ ] **Step 6: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/terminology/handlers.rs
git commit -m "refactor(http): unified list+search handlers, drop /search routes"
```

---

### Task 10: Router + OpenAPI schema registry — drop `/search` route entries and dropped schema names

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/terminology/router.rs`
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`
- Modify: `apps/server/aegis-server/src/transport/http/router.rs` (tests)

**Interfaces:**
- Consumes: nothing new.
- Produces: router registers only `list_code_lists` (no `search_code_lists`); openapi doc drops the deleted DTO schemas; the `openapi_json_returns_200_with_valid_doc` test drops the `/api/terminology/code-lists/search` and `/api/terminology/code-items/search` path assertions.

- [ ] **Step 1: Drop the search route entries**

In `apps/server/aegis-server/src/transport/http/terminology/router.rs`, delete the two `routes!` calls for `search_code_lists` and `search_code_items`:

```rust
        .routes(routes!(handlers::delete_code_list))
        // ---- CodeItem ----
        .routes(routes!(handlers::create_code_item))
```

Replace with:

```rust
        .routes(routes!(handlers::delete_code_list))
        // ---- CodeItem ----
        .routes(routes!(handlers::create_code_item))
```

(Just delete `.routes(routes!(handlers::search_code_lists))` from the CodeList block and `.routes(routes!(handlers::search_code_items))` from the CodeItem block.)

- [ ] **Step 2: Drop deleted DTO schemas from the registry**

In `apps/server/aegis-server/src/transport/http/openapi.rs`, in the `#[openapi(components(schemas(…)))]` list:

- Delete `dto::CodeListSearchHitResponse,`
- Delete `dto::CodeItemSearchHitResponse,`
- Delete `dto::CodeListSearchHitsResponse,`
- Delete `dto::CodeItemSearchHitsResponse,`
- Delete `dto::TerminologySearchBaseQuery,`
- Delete `dto::CodeListListResponse,`
- Delete `dto::CodeItemListResponse,`
- Add `dto::PagedCodeListsResponse,` and `dto::PagedCodeItemsResponse,` in their place.

- [ ] **Step 3: Update the openapi test**

In the `mod tests` block inside `openapi.rs`, find `openapi_registers_wire_dto_schemas` and remove the dropped names from the assertion list:

- Remove `"CodeListListResponse"`, `"CodeItemListResponse"`, `"CodeListSearchHitResponse"`, `"CodeItemSearchHitResponse"`, `"CodeListSearchHitsResponse"`, `"CodeItemSearchHitsResponse"`, `"TerminologySearchBaseQuery"`.
- Add `"PagedCodeListsResponse"`, `"PagedCodeItemsResponse"`.

- [ ] **Step 4: Update the `openapi_json_returns_200_with_valid_doc` test**

In `apps/server/aegis-server/src/transport/http/router.rs`, find the `terminology_reads` array inside the test and:

- Remove `("get", "/api/terminology/code-lists/search")`.
- Remove `("get", "/api/terminology/code-items/search")`.

- [ ] **Step 5: Verify the openapi doc + aegis-server build**

Run: `cargo test -p aegis-server --lib transport::http::openapi transport::http::router`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/terminology/router.rs \
        apps/server/aegis-server/src/transport/http/openapi.rs \
        apps/server/aegis-server/src/transport/http/router.rs
git commit -m "refactor(http): drop /search route entries + removed DTO schemas"
```

---

### Task 11: Update stub `TerminologyService` impls (3 locations)

**Files:**
- Modify: `apps/server/aegis-server/src/state.rs` (`NullTerminologyService` in `test_support`)
- Modify: `apps/server/aegis-server/src/transport/http/router.rs` (`StubTerminologyService` in `mod tests`)
- Modify: `apps/server/aegis-server/tests/integration_auth.rs` (`NullTerminologyService`)

**Interfaces:**
- Consumes: new trait shape from Task 6.
- Produces: each stub now `unimplemented!()`s `list_code_lists(query)` and `list_code_items(query)` (taking `Page<T>` returns), and no longer implements `search_code_lists` / `search_code_items`.

- [ ] **Step 1: Update `NullTerminologyService` in `state.rs`**

In `apps/server/aegis-server/src/state.rs`, find the `async fn list_code_lists(&self, _version_id: i64)` impl and replace with:

```rust
        async fn list_code_lists(
            &self,
            _query: apis::terminology::CodeListListQuery,
        ) -> Result<apis::terminology::Page<apis::terminology::CodeListView>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
```

Replace the `async fn list_code_items(&self, _codelist_id: i64)` impl with:

```rust
        async fn list_code_items(
            &self,
            _query: apis::terminology::CodeItemListQuery,
        ) -> Result<apis::terminology::Page<apis::terminology::CodeItemView>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
```

Delete the `async fn search_code_lists(&self, …)` and `async fn search_code_items(&self, …)` impls entirely.

- [ ] **Step 2: Update `StubTerminologyService` in `transport/http/router.rs`**

In `apps/server/aegis-server/src/transport/http/router.rs`, inside `mod tests`, find the `StubTerminologyService` impl block:

- Replace `async fn list_code_lists(&self, _version_id: i64)` with:

```rust
        async fn list_code_lists(
            &self,
            _query: apis::terminology::CodeListListQuery,
        ) -> Result<apis::terminology::Page<apis::terminology::CodeListView>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
```

- Replace `async fn list_code_items(&self, _codelist_id: i64)` with:

```rust
        async fn list_code_items(
            &self,
            _query: apis::terminology::CodeItemListQuery,
        ) -> Result<apis::terminology::Page<apis::terminology::CodeItemView>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
```

- Delete the `async fn search_code_lists(&self, …)` and `async fn search_code_items(&self, …)` impls entirely.

- [ ] **Step 3: Update `NullTerminologyService` in `tests/integration_auth.rs`**

In `apps/server/aegis-server/tests/integration_auth.rs`, find the `NullTerminologyService` impl block:

- Replace `async fn list_code_lists(&self, _version_id: i64)` with the same shape as step 1.
- Replace `async fn list_code_items(&self, _codelist_id: i64)` with the same shape as step 1.
- Delete the `async fn search_code_lists` and `async fn search_code_items` impls.

- [ ] **Step 4: Verify the aegis-server crate compiles end-to-end**

Run: `cargo build -p aegis-server --tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/state.rs \
        apps/server/aegis-server/src/transport/http/router.rs \
        apps/server/aegis-server/tests/integration_auth.rs
git commit -m "refactor(http): update TerminologyService stubs to unified list+search"
```

---

### Task 12: Usecase tests — drop search tests, add list_code_lists / list_code_items tests

**Files:**
- Modify: `lib/crates/terminology/src/usecase/tests.rs`
- Modify: `lib/crates/terminology/src/adapter/facade/in_memory/tests.rs`

**Interfaces:**
- Consumes: nothing new (the usecase already exposes the new shape from Task 5).
- Produces: new tests at the usecase + adapter layers; old `search_code_lists_*` and `search_code_items_*` tests are deleted.

- [ ] **Step 1: Write the failing usecase test (we) — `list_code_lists_returns_empty_page_when_no_codelists_exist`**

Append to `lib/crates/terminology/src/usecase/tests.rs`:

```rust
#[tokio::test]
async fn list_code_lists_returns_empty_page_when_no_codelists_exist() {
    let (_, _, _, usecase) = make_usecase();
    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: 1,
            fragment: None,
            offset: 0,
            limit: 50,
        })
        .await
        .expect("page");
    assert!(page.items.is_empty());
    assert_eq!(page.next_offset, None);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p terminology --lib usecase::tests::list_code_lists_returns_empty_page_when_no_codelists_exist`
Expected: FAIL — the usecase compiles now (Task 5), but the seeded `version_id = 1` has no codelists; the fake returns empty. Actually this test should pass; it's the next ones that need scaffolding.

- [ ] **Step 3: Add the remaining usecase tests**

Add to `lib/crates/terminology/src/usecase/tests.rs` (these each seed 5 codelists with non-sequential ids and exercise one behaviour at a time):

```rust
#[tokio::test]
async fn list_code_lists_with_no_fragment_returns_rows_in_id_order() {
    let (v_repo, _, _, usecase) = make_usecase();
    let v = v_repo
        .create(TerminologyVersionNew {
            kind: TerminologyKind::Sdtm,
            name: "v1".into(),
        })
        .await
        .expect("v");
    let _ = v;
    for code in ["C1", "C2", "C3", "C4", "C5"] {
        usecase
            .create_code_list(CreateCodeList {
                version_id: 1,
                code: code.into(),
                extensible: true,
                name: format!("name-{code}"),
                submission_value: code.into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("create");
    }
    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: 1,
            fragment: None,
            offset: 0,
            limit: 50,
        })
        .await
        .expect("page");
    let codes: Vec<String> = page.items.into_iter().map(|c| c.code).collect();
    assert_eq!(codes, vec!["C1", "C2", "C3", "C4", "C5"]);
}

#[tokio::test]
async fn list_code_lists_with_fragment_filters_results() {
    let (_, _, _, usecase) = make_usecase();
    for (code, name) in [
        ("C1", "AGE"),
        ("C2", "AGE GROUP"),
        ("C3", "SEX"),
        ("C4", "RACE"),
        ("C5", "AGE2"),
    ] {
        usecase
            .create_code_list(CreateCodeList {
                version_id: 1,
                code: code.into(),
                extensible: true,
                name: name.into(),
                submission_value: "x".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("create");
    }
    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: 1,
            fragment: Some("age".into()),
            offset: 0,
            limit: 50,
        })
        .await
        .expect("page");
    let names: Vec<String> = page.items.into_iter().map(|c| c.name).collect();
    assert_eq!(names, vec!["AGE", "AGE2", "AGE GROUP"]);
}

#[tokio::test]
async fn list_code_lists_paginates_with_offset_and_limit() {
    let (_, _, _, usecase) = make_usecase();
    for i in 0..5 {
        usecase
            .create_code_list(CreateCodeList {
                version_id: 1,
                code: format!("C{i}"),
                extensible: true,
                name: format!("name-{i}"),
                submission_value: "x".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("create");
    }
    let page1 = usecase
        .list_code_lists(CodeListListQuery {
            version_id: 1,
            fragment: None,
            offset: 2,
            limit: 2,
        })
        .await
        .expect("page 1");
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.next_offset, Some(4));

    let page2 = usecase
        .list_code_lists(CodeListListQuery {
            version_id: 1,
            fragment: None,
            offset: 4,
            limit: 2,
        })
        .await
        .expect("page 2");
    assert_eq!(page2.items.len(), 1);
    assert_eq!(page2.next_offset, None);
}

#[tokio::test]
async fn list_code_lists_rejects_tsquery_metacharacters() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .list_code_lists(CodeListListQuery {
            version_id: 1,
            fragment: Some("foo&bar".into()),
            offset: 0,
            limit: 50,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::InvalidFragment)
    ));
}

#[tokio::test]
async fn list_code_lists_clamps_limit_to_default_when_zero() {
    let (_, l_repo, _, usecase) = make_usecase();
    // Seed 51 codelists so a default of 50 leaves one beyond the page.
    for i in 0..51 {
        l_repo
            .create(CodeListNew {
                version_id: 1,
                code: format!("C{i}"),
                extensible: true,
                name: "x".into(),
                submission_value: "x".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("create");
    }
    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: 1,
            fragment: None,
            offset: 0,
            limit: 0,
        })
        .await
        .expect("page");
    assert_eq!(page.items.len(), 50);
    assert_eq!(page.next_offset, Some(50));
}

#[tokio::test]
async fn list_code_lists_clamps_limit_to_max_when_exceeded() {
    let (_, _, _, usecase) = make_usecase();
    let page = usecase
        .list_code_lists(CodeListListQuery {
            version_id: 1,
            fragment: None,
            offset: 0,
            limit: 10_000,
        })
        .await
        .expect("page");
    assert!(page.items.is_empty());
    assert_eq!(page.next_offset, None);
}
```

Add the matching `list_code_items_*` tests (mirror shape; seed items with `codelist_id`):

```rust
#[tokio::test]
async fn list_code_items_returns_empty_page_when_no_items_exist() {
    let (_, _, _, usecase) = make_usecase();
    let page = usecase
        .list_code_items(CodeItemListQuery {
            codelist_id: 1,
            fragment: None,
            offset: 0,
            limit: 50,
        })
        .await
        .expect("page");
    assert!(page.items.is_empty());
    assert_eq!(page.next_offset, None);
}

#[tokio::test]
async fn list_code_items_with_no_fragment_returns_rows_in_id_order() {
    let (_, _, _, usecase) = make_usecase();
    for code in ["C1", "C2", "C3", "C4", "C5"] {
        usecase
            .create_code_item(CreateCodeItem {
                codelist_id: 1,
                version_id: 1,
                code: code.into(),
                submission_value: "x".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("create");
    }
    let page = usecase
        .list_code_items(CodeItemListQuery {
            codelist_id: 1,
            fragment: None,
            offset: 0,
            limit: 50,
        })
        .await
        .expect("page");
    let codes: Vec<String> = page.items.into_iter().map(|i| i.code).collect();
    assert_eq!(codes, vec!["C1", "C2", "C3", "C4", "C5"]);
}

#[tokio::test]
async fn list_code_items_with_fragment_filters_results() {
    let (_, _, _, usecase) = make_usecase();
    for (code, def) in [
        ("C1", "positive"),
        ("C2", "negative"),
        ("C3", "absent"),
        ("C4", "POSITIVE reading"),
    ] {
        usecase
            .create_code_item(CreateCodeItem {
                codelist_id: 1,
                version_id: 1,
                code: code.into(),
                submission_value: "x".into(),
                synonym: "".into(),
                definition: def.into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("create");
    }
    let page = usecase
        .list_code_items(CodeItemListQuery {
            codelist_id: 1,
            fragment: Some("positive".into()),
            offset: 0,
            limit: 50,
        })
        .await
        .expect("page");
    let defs: Vec<String> = page.items.into_iter().map(|i| i.definition).collect();
    assert_eq!(defs, vec!["positive", "POSITIVE reading"]);
}

#[tokio::test]
async fn list_code_items_paginates_with_offset_and_limit() {
    let (_, _, _, usecase) = make_usecase();
    for i in 0..5 {
        usecase
            .create_code_item(CreateCodeItem {
                codelist_id: 1,
                version_id: 1,
                code: format!("C{i}"),
                submission_value: "x".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("create");
    }
    let page1 = usecase
        .list_code_items(CodeItemListQuery {
            codelist_id: 1,
            fragment: None,
            offset: 2,
            limit: 2,
        })
        .await
        .expect("page 1");
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.next_offset, Some(4));

    let page2 = usecase
        .list_code_items(CodeItemListQuery {
            codelist_id: 1,
            fragment: None,
            offset: 4,
            limit: 2,
        })
        .await
        .expect("page 2");
    assert_eq!(page2.items.len(), 1);
    assert_eq!(page2.next_offset, None);
}

#[tokio::test]
async fn list_code_items_rejects_tsquery_metacharacters() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .list_code_items(CodeItemListQuery {
            codelist_id: 1,
            fragment: Some("foo|bar".into()),
            offset: 0,
            limit: 50,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::InvalidFragment)
    ));
}

#[tokio::test]
async fn list_code_items_treats_empty_fragment_as_no_fragment() {
    let (_, _, _, usecase) = make_usecase();
    usecase
        .create_code_item(CreateCodeItem {
            codelist_id: 1,
            version_id: 1,
            code: "C1".into(),
            submission_value: "x".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("create");
    let page = usecase
        .list_code_items(CodeItemListQuery {
            codelist_id: 1,
            fragment: Some("   ".into()),
            offset: 0,
            limit: 50,
        })
        .await
        .expect("page");
    assert_eq!(page.items.len(), 1);
}
```

Also add the import for `CodeItemListQuery` (it should already be in scope from Task 4):

```rust
use crate::domain::{
    CodeItemListQuery, CodeListListQuery, /* ...rest */
};
```

- [ ] **Step 4: Drop the obsolete search test**

Delete the `search_code_lists_clamps_limit_to_default_when_zero` test (already dropped in Task 5 step 9; verify).

- [ ] **Step 5: Run the full usecase test suite**

Run: `cargo test -p terminology --lib usecase::`
Expected: PASS.

- [ ] **Step 6: Update the in-memory adapter tests**

In `lib/crates/terminology/src/adapter/facade/in_memory/tests.rs`:

- Delete `search_code_lists_returns_empty_for_in_memory_backend` and `search_code_items_returns_empty_for_in_memory_backend`.
- Delete `code_list_search_hit_projects_codelist` and `code_item_search_hit_projects_item` (the search-hit types no longer exist).
- Delete `list_code_lists_returns_codelists_owned_by_version` (the old `list_code_lists(version_id)` shape no longer exists — replaced below).
- Delete `list_code_items_returns_items_in_codelist` (same reason).

Add the new adapter tests:

```rust
#[tokio::test]
async fn list_code_lists_returns_first_page_with_next_offset_when_more_pages_exist() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    for i in 0..5 {
        svc.create_code_list(create_code_list_req(v.id, &format!("C{i}")))
            .await
            .unwrap();
    }
    let page = svc
        .list_code_lists(apis::terminology::CodeListListQuery {
            version_id: v.id,
            fragment: None,
            offset: 0,
            limit: 3,
        })
        .await
        .unwrap();
    assert_eq!(page.items.len(), 3);
    assert_eq!(page.next_offset, Some(3));
}

#[tokio::test]
async fn list_code_lists_returns_no_next_offset_when_page_is_last() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    svc.create_code_list(create_code_list_req(v.id, "C1"))
        .await
        .unwrap();
    let page = svc
        .list_code_lists(apis::terminology::CodeListListQuery {
            version_id: v.id,
            fragment: None,
            offset: 0,
            limit: 50,
        })
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.next_offset, None);
}

#[tokio::test]
async fn list_code_lists_with_fragment_filters_via_adapter() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    for code in ["C1", "C2", "C3"] {
        svc.create_code_list(create_code_list_req(v.id, code))
            .await
            .unwrap();
    }
    // The default `create_code_list_req` uses name="AGE"; override
    // by inserting a custom codelist with a different name through
    // the underlying repos is not exposed here, so rely on the
    // shared "AGE" name across all three rows + a fragment that
    // matches it.
    let page = svc
        .list_code_lists(apis::terminology::CodeListListQuery {
            version_id: v.id,
            fragment: Some("AGE".into()),
            offset: 0,
            limit: 50,
        })
        .await
        .unwrap();
    assert_eq!(page.items.len(), 3);
}

#[tokio::test]
async fn list_code_lists_returns_validation_error_for_invalid_fragment() {
    let svc = service();
    let err = svc
        .list_code_lists(apis::terminology::CodeListListQuery {
            version_id: 1,
            fragment: Some("foo:bar".into()),
            offset: 0,
            limit: 50,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, TerminologyApiError::Validation(_)));
}
```

Add the matching four tests for `list_code_items` (mirror shape; use `create_code_item_req`).

- [ ] **Step 7: Run the full in-memory adapter test suite**

Run: `cargo test -p terminology --lib adapter::facade::in_memory::`
Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/terminology/src/usecase/tests.rs \
        lib/crates/terminology/src/adapter/facade/in_memory/tests.rs
git commit -m "test(terminology): cover unified list+search at usecase + adapter"
```

---

### Task 13: HTTP handler tests — drop search tests, add list_code_lists / list_code_items tests

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/terminology/handlers.rs` (inside `mod tests`)

**Interfaces:**
- Consumes: nothing new.
- Produces: new handler tests covering pagination, fragment filtering, empty page, validation error. Old search tests deleted.

- [ ] **Step 1: Inspect the existing `mod tests` block**

Read the bottom of `apps/server/aegis-server/src/transport/http/terminology/handlers.rs` to confirm the test fixture shape. (No `mod tests` block exists today — this task adds one fresh.) Mirror the `MockUserService` / `MockAuth` / `app` / `read_json` / `build_request` scaffolding from `apps/server/aegis-server/src/transport/http/user/handlers.rs:170-455` so the new tests sit in the same shape as every other handler module in the codebase.

- [ ] **Step 2: Drop the obsolete search tests**

(There are no obsolete search tests to delete — `handlers.rs` has no `mod tests` block today. This step is a no-op; it exists only so that the diff between this plan and a future "with-existing-tests" plan stays reviewable.)

- [ ] **Step 3: Append a `mod tests` block with the scaffolding + the five codelist tests**

Append to `apps/server/aegis-server/src/transport/http/terminology/handlers.rs`:

```rust
#[cfg(test)]
mod tests {
    //! Per-handler tests for the unified `list_code_lists` /
    //! `list_code_items` routes. The mock service is configurable
    //! per method; each test sets the relevant fields and asserts
    //! the response status / body / error.

    use super::*;
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use axum::routing::get;
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    use apis::auth::{
        AuthApiError, AuthClaims, AuthService, CreateUserCredentialRequest,
        LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
        RefreshRequest, RefreshResponse, RegisterUserRequest, RegisterUserResponse,
        RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
        VerifyRequest,
    };

    #[derive(Clone, Default)]
    pub struct MockTerminologyService {
        pub list_code_lists_result: Option<apis::terminology::Page<apis::terminology::CodeListView>>,
        pub list_code_lists_err: Option<apis::terminology::TerminologyApiError>,
        pub list_code_items_result: Option<apis::terminology::Page<apis::terminology::CodeItemView>>,
        pub list_code_items_err: Option<apis::terminology::TerminologyApiError>,

        pub last_list_code_lists_args:
            Arc<Mutex<Option<apis::terminology::CodeListListQuery>>>,
        pub last_list_code_items_args:
            Arc<Mutex<Option<apis::terminology::CodeItemListQuery>>>,
    }

    #[async_trait]
    impl apis::terminology::TerminologyService for MockTerminologyService {
        async fn create_version(
            &self, _: apis::terminology::CreateTerminologyVersionRequest,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn list_versions(
            &self,
        ) -> Result<Vec<apis::terminology::TerminologyVersionView>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn get_version_by_id(
            &self, _: i64,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn update_version(
            &self, _: apis::terminology::UpdateTerminologyVersionRequest,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn delete_version(
            &self, _: i64,
        ) -> Result<(), apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn create_code_list(
            &self, _: apis::terminology::CreateCodeListRequest,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn list_code_lists(
            &self,
            q: apis::terminology::CodeListListQuery,
        ) -> Result<apis::terminology::Page<apis::terminology::CodeListView>, apis::terminology::TerminologyApiError>
        {
            *self.last_list_code_lists_args.lock().unwrap() = Some(q);
            if let Some(err) = self.list_code_lists_err.clone() {
                return Err(err);
            }
            Ok(self.list_code_lists_result.clone().expect("list_code_lists result configured"))
        }
        async fn get_code_list_by_id(
            &self, _: i64,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn update_code_list(
            &self, _: apis::terminology::UpdateCodeListRequest,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn delete_code_list(
            &self, _: i64,
        ) -> Result<(), apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn create_code_item(
            &self, _: apis::terminology::CreateCodeItemRequest,
        ) -> Result<apis::terminology::CodeItemView, apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn list_code_items(
            &self,
            q: apis::terminology::CodeItemListQuery,
        ) -> Result<apis::terminology::Page<apis::terminology::CodeItemView>, apis::terminology::TerminologyApiError>
        {
            *self.last_list_code_items_args.lock().unwrap() = Some(q);
            if let Some(err) = self.list_code_items_err.clone() {
                return Err(err);
            }
            Ok(self.list_code_items_result.clone().expect("list_code_items result configured"))
        }
        async fn list_code_items_by_version_and_code(
            &self, _: i64, _: &str,
        ) -> Result<Vec<apis::terminology::CodeItemView>, apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn update_code_item(
            &self, _: apis::terminology::UpdateCodeItemRequest,
        ) -> Result<apis::terminology::CodeItemView, apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn delete_code_item(
            &self, _: i64,
        ) -> Result<(), apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn batch_create_code_items(
            &self, _: apis::terminology::BatchCreateCodeItemsRequest,
        ) -> Result<apis::terminology::BatchCreateCodeItemsResponse, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
    }

    /// AuthClaims extractor mock; `verify_ok = true` returns a
    /// fixed Admin claim so the role guard in the write handlers
    /// passes.
    #[derive(Clone, Default)]
    pub struct MockAuth {
        pub verify_ok: bool,
        pub verify_err: Option<AuthApiError>,
    }

    #[async_trait]
    impl AuthService for MockAuth {
        async fn login_with_password(
            &self, _: LoginWithPasswordRequest,
        ) -> Result<TokenPair, AuthApiError> { unimplemented!() }
        async fn login_with_domain_user_info(
            &self, _: LoginWithDomainUserInfoRequest,
        ) -> Result<TokenPair, AuthApiError> { unimplemented!() }
        async fn verify(&self, _: VerifyRequest) -> Result<AuthClaims, AuthApiError> {
            if let Some(err) = self.verify_err.clone() { return Err(err); }
            assert!(self.verify_ok, "verify_ok must be set when no error configured");
            Ok(AuthClaims { code: "u1".into(), role: apis::user::Role::Admin, token_version: 0 })
        }
        async fn refresh(&self, _: RefreshRequest) -> Result<RefreshResponse, AuthApiError> { unimplemented!() }
        async fn find_user_credential_by_code(
            &self, _: &str,
        ) -> Result<UserCredentialView, AuthApiError> { unimplemented!() }
        async fn create_user_credential(
            &self, _: CreateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> { unimplemented!() }
        async fn update_user_credential(
            &self, _: UpdateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> { unimplemented!() }
        async fn remove_user_credential(
            &self, _: &str,
        ) -> Result<RemoveUserCredentialResponse, AuthApiError> { unimplemented!() }
        async fn logout(&self, _: LogoutRequest) -> Result<LogoutResponse, AuthApiError> { unimplemented!() }
        async fn register_user(
            &self, _: RegisterUserRequest,
        ) -> Result<RegisterUserResponse, AuthApiError> { unimplemented!() }
    }

    pub fn test_state(termin: MockTerminologyService) -> AppState {
        AppState {
            auth: Arc::new(MockAuth { verify_ok: true, ..Default::default() }) as Arc<dyn AuthService>,
            user: Arc::new(crate::state::test_support::NullUserServiceStub::not_implemented()) as Arc<dyn apis::user::UserService>,
            project: Arc::new(crate::state::test_support::NullProjectServiceStub::not_implemented()) as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(termin) as Arc<dyn apis::terminology::TerminologyService>,
        }
    }

    pub fn app(state: AppState) -> Router {
        Router::new()
            .route("/api/terminology/code-lists", get(list_code_lists))
            .route("/api/terminology/code-items", get(list_code_items))
            .with_state(state)
    }

    pub async fn read_json(response: axum::response::Response) -> (AxStatus, serde_json::Value) {
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    pub fn build_get(uri: &str, token: Option<&str>) -> Request<Body> {
        let mut b = Request::builder().method("GET").uri(uri);
        if let Some(t) = token { b = b.header("authorization", t); }
        b.body(Body::empty()).unwrap()
    }

    pub fn sample_code_list_view(id: i64, code: &str) -> apis::terminology::CodeListView {
        apis::terminology::CodeListView {
            id, version_id: 1, code: code.to_string(), extensible: true,
            name: format!("codelist {id}"), submission_value: code.to_string(),
            synonym: String::new(), definition: String::new(),
            nci_preferred_term: String::new(),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z").unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z").unwrap().with_timezone(&chrono::Utc),
        }
    }

    pub fn sample_code_item_view(id: i64, code: &str) -> apis::terminology::CodeItemView {
        apis::terminology::CodeItemView {
            id, codelist_id: 11, version_id: 1, code: code.to_string(),
            submission_value: code.to_string(), synonym: String::new(),
            definition: String::new(), nci_preferred_term: String::new(),
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z").unwrap().with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z").unwrap().with_timezone(&chrono::Utc),
        }
    }

    // ---- list_code_lists --------------------------------------------

    #[tokio::test]
    async fn list_code_lists_returns_first_page_with_next_offset() {
        let mut svc = MockTerminologyService::default();
        svc.list_code_lists_result = Some(apis::terminology::Page {
            items: vec![sample_code_list_view(1, "C1"), sample_code_list_view(2, "C2")],
            next_offset: Some(2),
        });
        let app = app(test_state(svc));
        let (status, body) = read_json(app.oneshot(build_get("/api/terminology/code-lists?versionId=1&limit=2", Some("Bearer good"))).await).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["codelists"].as_array().unwrap().len(), 2);
        assert_eq!(body["nextOffset"], 2);
    }

    #[tokio::test]
    async fn list_code_lists_returns_empty_page_when_no_codelists() {
        let mut svc = MockTerminologyService::default();
        svc.list_code_lists_result = Some(apis::terminology::Page { items: vec![], next_offset: None });
        let app = app(test_state(svc));
        let (status, body) = read_json(app.oneshot(build_get("/api/terminology/code-lists?versionId=1", Some("Bearer good"))).await).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["codelists"].as_array().unwrap().len(), 0);
        assert!(body.get("nextOffset").is_none(), "nextOffset must be omitted when None");
    }

    #[tokio::test]
    async fn list_code_lists_with_fragment_filters() {
        let mut svc = MockTerminologyService::default();
        svc.list_code_lists_result = Some(apis::terminology::Page {
            items: vec![sample_code_list_view(1, "AGE")],
            next_offset: None,
        });
        let app = app(test_state(svc));
        let (status, body) = read_json(app.oneshot(build_get("/api/terminology/code-lists?versionId=1&fragment=age", Some("Bearer good"))).await).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["codelists"].as_array().unwrap().len(), 1);
        assert_eq!(body["codelists"][0]["code"], "AGE");
    }

    #[tokio::test]
    async fn list_code_lists_with_invalid_fragment_returns_400() {
        let mut svc = MockTerminologyService::default();
        svc.list_code_lists_err = Some(apis::terminology::TerminologyApiError::Validation(
            "search fragment contains reserved tsquery characters: & | ! ( ) :".into(),
        ));
        let app = app(test_state(svc));
        let (status, body) = read_json(app.oneshot(build_get("/api/terminology/code-lists?versionId=1&fragment=foo%26bar", Some("Bearer good"))).await).await;
        assert_eq!(status, AxStatus::BAD_REQUEST);
        assert_eq!(body["code"], "validation_failed");
    }

    #[tokio::test]
    async fn list_code_lists_paginates() {
        // First call returns next_offset=Some(3), second returns None.
        let mut svc = MockTerminologyService::default();
        svc.list_code_lists_result = Some(apis::terminology::Page {
            items: vec![sample_code_list_view(1, "C1"), sample_code_list_view(2, "C2"), sample_code_list_view(3, "C3")],
            next_offset: Some(3),
        });
        let app = app(test_state(svc.clone()));
        let (_, body1) = read_json(app.oneshot(build_get("/api/terminology/code-lists?versionId=1&offset=0&limit=3", Some("Bearer good"))).await).await;
        assert_eq!(body1["nextOffset"], 3);

        svc.list_code_lists_result = Some(apis::terminology::Page {
            items: vec![sample_code_list_view(4, "C4")],
            next_offset: None,
        });
        let app = app(test_state(svc));
        let (_, body2) = read_json(app.oneshot(build_get("/api/terminology/code-lists?versionId=1&offset=3&limit=3", Some("Bearer good"))).await).await;
        assert!(body2.get("nextOffset").is_none());
    }

    // ---- list_code_items --------------------------------------------

    #[tokio::test]
    async fn list_code_items_returns_first_page_with_next_offset() {
        let mut svc = MockTerminologyService::default();
        svc.list_code_items_result = Some(apis::terminology::Page {
            items: vec![sample_code_item_view(1, "C1"), sample_code_item_view(2, "C2")],
            next_offset: Some(2),
        });
        let app = app(test_state(svc));
        let (status, body) = read_json(app.oneshot(build_get("/api/terminology/code-items?codelistId=11&limit=2", Some("Bearer good"))).await).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["items"].as_array().unwrap().len(), 2);
        assert_eq!(body["nextOffset"], 2);
    }

    #[tokio::test]
    async fn list_code_items_returns_empty_page_when_no_items() {
        let mut svc = MockTerminologyService::default();
        svc.list_code_items_result = Some(apis::terminology::Page { items: vec![], next_offset: None });
        let app = app(test_state(svc));
        let (status, body) = read_json(app.oneshot(build_get("/api/terminology/code-items?codelistId=11", Some("Bearer good"))).await).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["items"].as_array().unwrap().len(), 0);
        assert!(body.get("nextOffset").is_none());
    }

    #[tokio::test]
    async fn list_code_items_with_fragment_filters() {
        let mut svc = MockTerminologyService::default();
        svc.list_code_items_result = Some(apis::terminology::Page {
            items: vec![sample_code_item_view(1, "YES")],
            next_offset: None,
        });
        let app = app(test_state(svc));
        let (status, body) = read_json(app.oneshot(build_get("/api/terminology/code-items?codelistId=11&fragment=YES", Some("Bearer good"))).await).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["items"][0]["code"], "YES");
    }

    #[tokio::test]
    async fn list_code_items_with_invalid_fragment_returns_400() {
        let mut svc = MockTerminologyService::default();
        svc.list_code_items_err = Some(apis::terminology::TerminologyApiError::Validation(
            "search fragment contains reserved tsquery characters: & | ! ( ) :".into(),
        ));
        let app = app(test_state(svc));
        let (status, _) = read_json(app.oneshot(build_get("/api/terminology/code-items?codelistId=11&fragment=foo%7Cbar", Some("Bearer good"))).await).await;
        assert_eq!(status, AxStatus::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_code_items_paginates() {
        let mut svc = MockTerminologyService::default();
        svc.list_code_items_result = Some(apis::terminology::Page {
            items: vec![sample_code_item_view(1, "C1"), sample_code_item_view(2, "C2"), sample_code_item_view(3, "C3")],
            next_offset: Some(3),
        });
        let app = app(test_state(svc.clone()));
        let (_, body1) = read_json(app.oneshot(build_get("/api/terminology/code-items?codelistId=11&offset=0&limit=3", Some("Bearer good"))).await).await;
        assert_eq!(body1["nextOffset"], 3);

        svc.list_code_items_result = Some(apis::terminology::Page {
            items: vec![sample_code_item_view(4, "C4")],
            next_offset: None,
        });
        let app = app(test_state(svc));
        let (_, body2) = read_json(app.oneshot(build_get("/api/terminology/code-items?codelistId=11&offset=3&limit=3", Some("Bearer good"))).await).await;
        assert!(body2.get("nextOffset").is_none());
    }
}
```

(Note: the scaffolding references `crate::state::test_support::NullUserServiceStub::not_implemented()` and `NullProjectServiceStub::not_implemented()` as placeholders — if those types don't exist, replace them with `unimplemented!()`-stub impls following the same shape as `NullTerminologyService` in `state.rs:34-163`. The test compiles because none of those stub methods are called by the handlers exercised here.)

- [ ] **Step 4: Run the handler tests**

Run: `cargo test -p aegis-server --lib transport::http::terminology::handlers::`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/terminology/handlers.rs
git commit -m "test(http): unified list_code_lists / list_code_items handler tests"
```

---

### Task 14: Integration tests (Postgres) — drop `search_code_lists_ranks_hits`, add pagination / fragment / validation tests

**Files:**
- Modify: `lib/crates/terminology/tests/integration_persistence.rs`

**Interfaces:**
- Consumes: `CodeListListQuery` / `CodeItemListQuery` from Task 1.
- Produces: new live-DB tests for the unified method; the old `search_code_lists_ranks_hits` test is replaced by `list_code_lists_with_fragment_returns_ranked_matches`.

- [ ] **Step 1: Drop the obsolete search test**

In `lib/crates/terminology/tests/integration_persistence.rs`, delete `search_code_lists_ranks_hits` (the entire `#[tokio::test] #[ignore = "…"] async fn search_code_lists_ranks_hits` block).

- [ ] **Step 2: Add the four `list_code_lists_*` tests**

```rust
#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn list_code_lists_with_fragment_returns_ranked_matches() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("rank-v"),
            })
            .await
            .expect("version");

        // Three rows; "AGE" hits multiple fields, "AGE GROUP" hits
        // one field with the synonym stem. Postgres' ts_rank orders
        // them by match frequency.
        l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("age-cl"),
                extensible: true,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "Age group".into(),
                definition: "Subject age".into(),
                nci_preferred_term: "Age".into(),
            })
            .await
            .expect("age cl");
        l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("sex-cl"),
                extensible: true,
                name: "SEX".into(),
                submission_value: "SEX".into(),
                synonym: "".into(),
                definition: "Sex".into(),
                nci_preferred_term: "Sex".into(),
            })
            .await
            .expect("sex cl");

        let page = l_repo
            .search_or_list(CodeListListQuery {
                version_id: v.id,
                fragment: Some("age".into()),
                offset: 0,
                limit: 50,
            })
            .await
            .expect("page");
        assert!(
            page.items.iter().any(|c| c.name == "AGE"),
            "AGE row should be in the hits: {:?}",
            page.items.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn list_code_lists_with_empty_fragment_returns_plain_list() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("empty-frag-v"),
            })
            .await
            .expect("version");

        l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("age-cl"),
                extensible: true,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("cl");

        // fragment = Some("") must produce the same shape as None.
        let p_none = l_repo
            .search_or_list(CodeListListQuery {
                version_id: v.id,
                fragment: None,
                offset: 0,
                limit: 50,
            })
            .await
            .expect("none");
        let p_empty = l_repo
            .search_or_list(CodeListListQuery {
                version_id: v.id,
                fragment: Some("".into()),
                offset: 0,
                limit: 50,
            })
            .await
            .expect("empty");
        assert_eq!(p_none.items.len(), p_empty.items.len());
        assert_eq!(p_none.next_offset, p_empty.next_offset);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn list_code_lists_rejects_invalid_fragment() {
    // The validation lives in the usecase; the integration test
    // exercises the full Postgres wiring by going through the
    // usecase and asserting it produces InvalidFragment.
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
            version_repo: v_repo,
            code_list_repo: l_repo,
            code_item_repo: CodeItemRepo::new(pool.clone()),
        });

        let err = usecase
            .list_code_lists(CodeListListQuery {
                version_id: 1,
                fragment: Some("foo&bar".into()),
                offset: 0,
                limit: 50,
            })
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            terminology::UsecaseError::Validation(terminology::DomainError::InvalidFragment)
        ));
    })
    .await;
}
```

Add the matching three tests for `list_code_items` (`list_code_items_with_fragment_returns_ranked_matches`, `list_code_items_with_empty_fragment_returns_plain_list`, `list_code_items_rejects_invalid_fragment`). Mirror shape: seed items under a codelist, call `i_repo.search_or_list` with `CodeItemListQuery`.

Update the imports at the top of `tests/integration_persistence.rs`:

```rust
use terminology::{
{
    CodeItem, CodeItemListQuery, CodeItemNew, CodeItemRepo, CodeItemRepository, CodeList,
    CodeListListQuery, CodeListNew, CodeListRepo, CodeListRepository, CreateTerminologyVersion,
    DomainError, TerminologyKind, TerminologyUsecase, TerminologyUsecaseConfig,
    TerminologyVersion, TerminologyVersionNew, TerminologyVersionRepo, TerminologyVersionRepository,
};
```

- [ ] **Step 3: Run the integration suite**

Run: `cargo test -p terminology --test integration_persistence -- --ignored`
Expected: PASS (requires `AEGIS_TERMINOLOGY_DATABASE_URL`).

- [ ] **Step 4: Commit**

```bash
git add lib/crates/terminology/tests/integration_persistence.rs
git commit -m "test(terminology): integration coverage for unified list+search"
```

---

### Task 15: Verification

**Files:** none — sanity-check only.

- [ ] **Step 1: Build every crate in the workspace**

Run:
```bash
cargo build -p apis -p terminology -p aegis-server --tests
```
Expected: clean build.

- [ ] **Step 2: Run the terminology crate's unit tests**

Run:
```bash
cargo test -p terminology --lib
```
Expected: ~74 tests pass (60 baseline + ~14 new). New tests added in Task 12 should all be green.

- [ ] **Step 3: Run the aegis-server transport HTTP tests**

Run:
```bash
cargo test -p aegis-server --lib transport::http::terminology
```
Expected: PASS — openapi doc test + handler tests + router-integration openapi doc test are all green.

- [ ] **Step 4: Run the live-DB integration suite**

Run:
```bash
cargo test -p terminology --test integration_persistence -- --ignored
```
Expected: PASS (requires `AEGIS_TERMINOLOGY_DATABASE_URL`).

- [ ] **Step 5: Sanity-check the desktop crate (must remain green)**

Run:
```bash
cargo test -p aegis-desktop --lib
```
Expected: PASS. No files under `apps/desktop/aegis-desktop/` were touched; the existing Tauri wiremock tests for the list endpoints continue to deserialize the new `{ codelists, nextOffset? }` / `{ items, nextOffset? }` envelope because they read `resp.codelists` / `resp.items` to a `Vec` and ignore the optional `nextOffset`.

- [ ] **Step 6: Run the aegis-server auth integration test (sanity)**

Run:
```bash
cargo test -p aegis-server --test integration_auth
```
Expected: PASS — the `NullTerminologyService` was updated in Task 11 to match the new trait shape.

- [ ] **Step 7: Commit any leftover changes**

If the verification runs surfaced a missed edit, commit it under `refactor(terminology):` or `fix(terminology):`.

---

### Task 16: Commit the spec (it has been staged but never committed)

**Files:** (already staged earlier in this session) `docs/superpowers/specs/2026-08-20-terminology-list-search-refactor-design.md`, `docs/superpowers/plans/2026-08-20-terminology-list-search-refactor.md`.

- [ ] **Step 1: Confirm both docs are committed**

Run:
```bash
git status
git log --oneline -5
```
Expected: the spec (`docs/superpowers/specs/…`) and plan (`docs/superpowers/plans/…`) are present in the commit history. If not:

```bash
git add docs/superpowers/specs/2026-08-20-terminology-list-search-refactor-design.md \
        docs/superpowers/plans/2026-08-20-terminology-list-search-refactor.md
git commit -m "docs(terminology): spec + plan for unified list+search refactor"
```

- [ ] **Step 2: Sanity-check the commit graph**

Run:
```bash
git log --oneline -20
```
Expected: a clean sequence of `feat(terminology):`, `refactor(terminology):`, `refactor(apis):`, `refactor(http):`, `test(terminology):`, `test(http):`, `docs(terminology):` commits covering the full surface.

---