# CRF Form Detail Endpoint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `GET /api/crf/forms/{id}/details` — a single endpoint that returns one CRF form together with every piece of state owned by that form (items composed with their options/units/annotations, domain annotations, and form-level annotations).

**Architecture:** Five new batch port methods (`list_by_items` on `CrfOptionRepository` / `CrfUnitRepository`; `list_by_items` / `list_by_options` / `list_by_units` on `AnnotationRepository`) keep round-trips bounded at 9 queries with up to 4 in flight via `tokio::try_join!`. Usecase composes the tree in memory; four new view structs (`CrfFormDetailView`, `CrfItemDetailView`, `CrfOptionDetailView`, `CrfUnitDetailView`) carry it across the apis boundary; four wire DTOs project it to JSON.

**Tech Stack:** Rust 2024 edition, axum 0.7, sqlx (runtime API), thiserror, chrono, serde, tokio, utoipa-axum, ToSchema.

**Spec:** [docs/superpowers/specs/2026-08-27-crf-form-detail-design.md](../specs/2026-08-27-crf-form-detail-design.md)

---

## Conventions for every task

- Run `cargo fmt --all` and `cargo clippy -p crf --all-targets --all-features -- -D warnings` after every code change in the `crf` crate; fix any warning before committing.
- Run `cargo test -p crf` after every commit to catch regressions; the suite is fast (<10s without live DB).
- Live-DB tests are gated `#[ignore]`; run them with `cargo test -p crf -- --ignored --test-threads=1` only when `AEGIS_DATABASE_URL` is set.
- Commit messages follow the existing convention: `feat(crf): …`, `refactor(crf): …`, `test(crf): …`, `docs(crf): …`. End each with `Co-Authored-By: Claude <noreply@anthropic.com>`.
- The trait extensions in Tasks 1-3 must update **both** the trait and **every existing impl** in the same commit (Postgres adapter + in-memory fake in `usecase/tests.rs`), otherwise the workspace won't compile.

---

### Task 1: Extend `CrfOptionRepository` with `list_by_items`

**Files:**
- Modify: `lib/crates/crf/src/domain/crf_option.rs:87-102` (trait)
- Modify: `lib/crates/crf/src/adapter/persistence/postgres/crf_option_repo.rs:122` (impl — add after `search_by_version`)
- Modify: `lib/crates/crf/src/usecase/tests.rs:353` (impl — add after `search_by_version` in `InMemoryOptions`)

- [ ] **Step 1: Add the trait method**

In `lib/crates/crf/src/domain/crf_option.rs`, inside the `CrfOptionRepository` trait (after `count_by_item`), add:

```rust
/// Batch fetch every option whose `item_id` is in `item_ids`.
/// Returns `Ok(Vec::new())` for empty input without hitting the DB.
async fn list_by_items(
    &self,
    item_ids: &[i64],
) -> Result<Vec<CrfOption>, DomainError>;
```

- [ ] **Step 2: Add the Postgres impl**

In `lib/crates/crf/src/adapter/persistence/postgres/crf_option_repo.rs`, at the end of the `impl CrfOptionRepository for CrfOptionRepoPg` block (after `search_by_version`), add:

```rust
async fn list_by_items(
    &self,
    item_ids: &[i64],
) -> Result<Vec<CrfOption>, DomainError> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, CrfOptionRow>(
        "SELECT id, item_id, value, not_submitted, created_at, updated_at
         FROM crf_options WHERE item_id = ANY($1) ORDER BY id ASC",
    )
    .bind(item_ids)
    .fetch_all(&self.pool)
    .await
    .map_err(map_db_err)?;
    Ok(rows.into_iter().map(Into::into).collect())
}
```

- [ ] **Step 3: Add the in-memory fake impl**

In `lib/crates/crf/src/usecase/tests.rs`, inside `impl CrfOptionRepository for InMemoryOptions` (after `search_by_version`), add:

```rust
async fn list_by_items(
    &self,
    item_ids: &[i64],
) -> Result<Vec<CrfOption>, DomainError> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = self.rows.lock().unwrap();
    Ok(rows
        .values()
        .filter(|o| item_ids.contains(&o.item_id))
        .cloned()
        .collect())
}
```

- [ ] **Step 4: Build the crate to verify the new method compiles everywhere**

Run: `cargo build -p crf --all-targets`
Expected: success (the trait + both impls now line up).

- [ ] **Step 5: Run the test suite**

Run: `cargo test -p crf`
Expected: all existing tests pass; no new tests added in this task (the round-trip is covered by the integration test in Task 13).

- [ ] **Step 6: Commit**

```bash
git add lib/crates/crf/src/domain/crf_option.rs \
        lib/crates/crf/src/adapter/persistence/postgres/crf_option_repo.rs \
        lib/crates/crf/src/usecase/tests.rs
git commit -m "feat(crf): add CrfOptionRepository::list_by_items batch port

Single WHERE item_id = ANY(\$1) query for batch option lookup. Used
by get_form_detail to hydrate the items subtree in one round-trip.
Empty input returns Ok(Vec::new()) without hitting the DB.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Extend `CrfUnitRepository` with `list_by_items`

**Files:**
- Modify: `lib/crates/crf/src/domain/crf_unit.rs:87-98` (trait)
- Modify: `lib/crates/crf/src/adapter/persistence/postgres/crf_unit_repo.rs` (impl — add after `search_by_version`)
- Modify: `lib/crates/crf/src/usecase/tests.rs` (impl — add after `search_by_version` in `InMemoryUnits`)

- [ ] **Step 1: Add the trait method**

In `lib/crates/crf/src/domain/crf_unit.rs`, inside the `CrfUnitRepository` trait (after `list_by_item`), add:

```rust
/// Batch fetch every unit whose `item_id` is in `item_ids`.
/// Returns `Ok(Vec::new())` for empty input without hitting the DB.
async fn list_by_items(
    &self,
    item_ids: &[i64],
) -> Result<Vec<CrfUnit>, DomainError>;
```

- [ ] **Step 2: Add the Postgres impl**

In `lib/crates/crf/src/adapter/persistence/postgres/crf_unit_repo.rs`, at the end of the `impl CrfUnitRepository for CrfUnitRepoPg` block (after `search_by_version`), add:

```rust
async fn list_by_items(
    &self,
    item_ids: &[i64],
) -> Result<Vec<CrfUnit>, DomainError> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, CrfUnitRow>(
        "SELECT id, item_id, value, not_submitted, created_at, updated_at
         FROM crf_units WHERE item_id = ANY($1) ORDER BY id ASC",
    )
    .bind(item_ids)
    .fetch_all(&self.pool)
    .await
    .map_err(map_db_err)?;
    Ok(rows.into_iter().map(Into::into).collect())
}
```

(Inspect the file first to confirm the row struct is named `CrfUnitRow`; if it's been renamed since the spec, follow the local convention.)

- [ ] **Step 3: Add the in-memory fake impl**

In `lib/crates/crf/src/usecase/tests.rs`, inside `impl CrfUnitRepository for InMemoryUnits` (after `search_by_version`), add:

```rust
async fn list_by_items(
    &self,
    item_ids: &[i64],
) -> Result<Vec<CrfUnit>, DomainError> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = self.rows.lock().unwrap();
    Ok(rows
        .values()
        .filter(|u| item_ids.contains(&u.item_id))
        .cloned()
        .collect())
}
```

- [ ] **Step 4: Build and test**

Run: `cargo build -p crf --all-targets && cargo test -p crf`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/crf/src/domain/crf_unit.rs \
        lib/crates/crf/src/adapter/persistence/postgres/crf_unit_repo.rs \
        lib/crates/crf/src/usecase/tests.rs
git commit -m "feat(crf): add CrfUnitRepository::list_by_items batch port

Single WHERE item_id = ANY(\$1) query for batch unit lookup. Used
by get_form_detail to hydrate the items subtree in one round-trip.
Empty input returns Ok(Vec::new()) without hitting the DB.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Extend `AnnotationRepository` with `list_by_items` / `list_by_options` / `list_by_units`

**Files:**
- Modify: `lib/crates/crf/src/domain/annotation.rs:197-211` (trait)
- Modify: `lib/crates/crf/src/adapter/persistence/postgres/annotation_repo.rs:168` (impl — add after `list_by_unit`)
- Modify: `lib/crates/crf/src/usecase/tests.rs:512` (impl — add after `list_by_unit` in `InMemoryAnnotations`)

- [ ] **Step 1: Add the three trait methods**

In `lib/crates/crf/src/domain/annotation.rs`, inside the `AnnotationRepository` trait (after `list_by_unit`), add:

```rust
/// Batch fetch every annotation owned by an item whose id is in
/// `item_ids` (i.e. `item_id IN (...)` and the other three FK
/// columns null). Returns `Ok(Vec::new())` for empty input.
async fn list_by_items(
    &self,
    item_ids: &[i64],
) -> Result<Vec<Annotation>, DomainError>;
/// Batch fetch every annotation owned by an option whose id is in
/// `option_ids`. Returns `Ok(Vec::new())` for empty input.
async fn list_by_options(
    &self,
    option_ids: &[i64],
) -> Result<Vec<Annotation>, DomainError>;
/// Batch fetch every annotation owned by a unit whose id is in
/// `unit_ids`. Returns `Ok(Vec::new())` for empty input.
async fn list_by_units(
    &self,
    unit_ids: &[i64],
) -> Result<Vec<Annotation>, DomainError>;
```

- [ ] **Step 2: Add the Postgres impl**

In `lib/crates/crf/src/adapter/persistence/postgres/annotation_repo.rs`, at the end of the `impl AnnotationRepository for AnnotationRepoPg` block (after `list_by_unit`), add:

```rust
async fn list_by_items(
    &self,
    item_ids: &[i64],
) -> Result<Vec<Annotation>, DomainError> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, AnnotationRow>(
        "SELECT id, domain_annotation_id, content, assign,
                form_id, item_id, option_id, unit_id, created_at, updated_at
         FROM crf_annotations WHERE item_id = ANY($1) ORDER BY id ASC",
    )
    .bind(item_ids)
    .fetch_all(&self.pool)
    .await
    .map_err(map_db_err)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

async fn list_by_options(
    &self,
    option_ids: &[i64],
) -> Result<Vec<Annotation>, DomainError> {
    if option_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, AnnotationRow>(
        "SELECT id, domain_annotation_id, content, assign,
                form_id, item_id, option_id, unit_id, created_at, updated_at
         FROM crf_annotations WHERE option_id = ANY($1) ORDER BY id ASC",
    )
    .bind(option_ids)
    .fetch_all(&self.pool)
    .await
    .map_err(map_db_err)?;
    Ok(rows.into_iter().map(Into::into).collect())
}

async fn list_by_units(
    &self,
    unit_ids: &[i64],
) -> Result<Vec<Annotation>, DomainError> {
    if unit_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query_as::<_, AnnotationRow>(
        "SELECT id, domain_annotation_id, content, assign,
                form_id, item_id, option_id, unit_id, created_at, updated_at
         FROM crf_annotations WHERE unit_id = ANY($1) ORDER BY id ASC",
    )
    .bind(unit_ids)
    .fetch_all(&self.pool)
    .await
    .map_err(map_db_err)?;
    Ok(rows.into_iter().map(Into::into).collect())
}
```

- [ ] **Step 3: Add the in-memory fake impl**

In `lib/crates/crf/src/usecase/tests.rs`, inside `impl AnnotationRepository for InMemoryAnnotations` (after `list_by_unit`), add:

```rust
async fn list_by_items(
    &self,
    item_ids: &[i64],
) -> Result<Vec<Annotation>, DomainError> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = self.rows.lock().unwrap();
    Ok(rows
        .values()
        .filter(|a| {
            matches!(a.owner, AnnotationOwner::Item { id } if item_ids.contains(&id))
        })
        .cloned()
        .collect())
}

async fn list_by_options(
    &self,
    option_ids: &[i64],
) -> Result<Vec<Annotation>, DomainError> {
    if option_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = self.rows.lock().unwrap();
    Ok(rows
        .values()
        .filter(|a| {
            matches!(a.owner, AnnotationOwner::Option { id } if option_ids.contains(&id))
        })
        .cloned()
        .collect())
}

async fn list_by_units(
    &self,
    unit_ids: &[i64],
) -> Result<Vec<Annotation>, DomainError> {
    if unit_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = self.rows.lock().unwrap();
    Ok(rows
        .values()
        .filter(|a| {
            matches!(a.owner, AnnotationOwner::Unit { id } if unit_ids.contains(&id))
        })
        .cloned()
        .collect())
}
```

- [ ] **Step 4: Build and test**

Run: `cargo build -p crf --all-targets && cargo test -p crf`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/crf/src/domain/annotation.rs \
        lib/crates/crf/src/adapter/persistence/postgres/annotation_repo.rs \
        lib/crates/crf/src/usecase/tests.rs
git commit -m "feat(crf): add AnnotationRepository::list_by_items/options/units

Three batch port methods mirroring list_by_item/list_by_option/
list_by_unit but accepting a slice of ids. Single WHERE <fk> = ANY
query each. Used by get_form_detail to hydrate per-layer
annotations in three extra round-trips.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Add the four usecase view structs

**Files:**
- Modify: `lib/crates/crf/src/usecase/views.rs` (append new types at the end)
- Modify: `lib/crates/crf/src/lib.rs:48-57` (add re-exports)

- [ ] **Step 1: Add the four structs**

In `lib/crates/crf/src/usecase/views.rs`, append at the end of the file:

```rust
/// Composed view for `CrfUsecase::get_form_detail`. Returns the
/// form together with every piece of state owned by it:
/// items composed with their options, units, and per-layer
/// annotations; the form's domain annotations; and form-level
/// annotations.
///
/// Mirrors `apis::crf::CrfFormDetailView`. Annotations are
/// nested under their owner: form-level annotations live next
/// to the form; item/option/unit-level annotations live inside
/// their parent. Empty subtrees return an empty vec (never
/// `None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfFormDetailView {
    pub form: CrfFormView,
    pub form_annotations: Vec<AnnotationView>,
    pub items: Vec<CrfItemDetailView>,
    pub domain_annotations: Vec<DomainAnnotationView>,
}

/// One item in `CrfFormDetailView::items`, composed with its
/// options, units, and item-level annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfItemDetailView {
    pub item: CrfItemView,
    pub options: Vec<CrfOptionDetailView>,
    pub units: Vec<CrfUnitDetailView>,
    pub annotations: Vec<AnnotationView>,
}

/// One option in `CrfItemDetailView::options`, composed with
/// its option-level annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfOptionDetailView {
    pub option: CrfOptionView,
    pub annotations: Vec<AnnotationView>,
}

/// One unit in `CrfItemDetailView::units`, composed with its
/// unit-level annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfUnitDetailView {
    pub unit: CrfUnitView,
    pub annotations: Vec<AnnotationView>,
}
```

(No `From<Domain>` impls — the domain has no aggregate with the nested
shape; the usecase composes these directly during `get_form_detail`.)

- [ ] **Step 2: Re-export the new types from the crate root**

In `lib/crates/crf/src/lib.rs`, in the `pub use usecase::{ ... }` block, add the four new types alongside `CrfBulkFormResult`:

```rust
pub use usecase::{
    AnnotationView, CreateAnnotation, CreateCrfBulkForm, CreateCrfBulkItem, CreateCrfForm,
    CreateCrfItem, CreateCrfOption, CreateCrfUnit, CreateCrfVersion, CreateDomainAnnotation,
    CrfBulkFormResult, CrfFormDetailView, CrfFormView, CrfItemDetailView, CrfItemView,
    CrfOptionDetailView, CrfOptionView, CrfUnitDetailView, CrfUnitView, CrfUsecase,
    CrfUsecaseConfig, CrfVersionView, DomainAnnotationView, SearchAnnotationsByVersion,
    SearchCrfFormsByVersion, SearchCrfItemsByVersion, SearchCrfOptionsByVersion,
    SearchCrfUnitsByVersion, SearchDomainAnnotationsByVersion, UpdateAnnotation, UpdateCrfForm,
    UpdateCrfItem, UpdateCrfOption, UpdateCrfUnit, UpdateCrfVersion, UpdateDomainAnnotation,
    UsecaseError,
};
```

- [ ] **Step 3: Build the crate**

Run: `cargo build -p crf`
Expected: success.

- [ ] **Step 4: Commit**

```bash
git add lib/crates/crf/src/usecase/views.rs lib/crates/crf/src/lib.rs
git commit -m "feat(crf): add CrfFormDetailView and nested detail view types

Four composed view structs that carry the form, items, options,
units, domain annotations, and per-layer annotations across the
usecase / apis boundary. Re-exported from the crate root.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Implement `CrfUsecase::get_form_detail` (TDD)

**Files:**
- Modify: `lib/crates/crf/src/usecase/crf_usecase.rs` (add `get_form_detail` in the `// ---- CrfForm ----` section, after `create_bulk_form`)
- Modify: `lib/crates/crf/src/usecase/tests.rs` (add the failing test first, then watch it pass)

- [ ] **Step 1: Write the failing usecase test**

Append at the end of `lib/crates/crf/src/usecase/tests.rs`. The setup mirrors `crud_annotation_item_owner` (line 1210) — drive the usecase through its public `create_*` methods so the path under test is the same one `get_form_detail` will use:

```rust
#[tokio::test]
async fn get_form_detail_assembles_tree_in_id_order() {
    let uc = usecase();

    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = uc
        .create_form(CreateCrfForm {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let d = uc
        .create_domain_annotation(CreateDomainAnnotation {
            form_id: f.id,
            name: "Hint".into(),
            description: "".into(),
        })
        .await
        .unwrap();

    // Item 1: Text with one option, one unit, one item-annotation.
    let i1 = uc
        .create_item(CreateCrfItem {
            form_id: f.id,
            code: "I1".into(),
            name: "Item 1".into(),
            kind: CrfItemKind::Text,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let o1 = uc
        .create_option(CreateCrfOption {
            item_id: i1.id,
            value: "yes".into(),
            not_submitted: false,
        })
        .await
        .unwrap();
    let u1 = uc
        .create_unit(CreateCrfUnit {
            item_id: i1.id,
            value: "mg".into(),
            not_submitted: false,
        })
        .await
        .unwrap();

    // Item 2: Selection with one option, no units, no annotations.
    let i2 = uc
        .create_item(CreateCrfItem {
            form_id: f.id,
            code: "I2".into(),
            name: "Item 2".into(),
            kind: CrfItemKind::Selection,
            order: 1,
            not_submitted: false,
        })
        .await
        .unwrap();
    let o2 = uc
        .create_option(CreateCrfOption {
            item_id: i2.id,
            value: "no".into(),
            not_submitted: false,
        })
        .await
        .unwrap();

    // Annotations at every layer.
    uc.create_annotation(CreateAnnotation {
        domain_annotation_id: d.id,
        content: "form-level".into(),
        assign: false,
        owner: AnnotationOwner::Form { id: f.id },
    })
    .await
    .unwrap();
    uc.create_annotation(CreateAnnotation {
        domain_annotation_id: d.id,
        content: "item-1".into(),
        assign: false,
        owner: AnnotationOwner::Item { id: i1.id },
    })
    .await
    .unwrap();
    uc.create_annotation(CreateAnnotation {
        domain_annotation_id: d.id,
        content: "option-1".into(),
        assign: false,
        owner: AnnotationOwner::Option { id: o1.id },
    })
    .await
    .unwrap();
    uc.create_annotation(CreateAnnotation {
        domain_annotation_id: d.id,
        content: "unit-1".into(),
        assign: false,
        owner: AnnotationOwner::Unit { id: u1.id },
    })
    .await
    .unwrap();

    let detail = uc.get_form_detail(f.id).await.unwrap();

    assert_eq!(detail.form.id, f.id);
    assert_eq!(detail.form_annotations.len(), 1);
    assert_eq!(
        detail.form_annotations[0].owner,
        AnnotationOwner::Form { id: f.id }
    );
    assert_eq!(detail.domain_annotations.len(), 1);
    assert_eq!(detail.domain_annotations[0].id, d.id);

    assert_eq!(detail.items.len(), 2);

    // Items are returned in `order ASC, id ASC`.
    assert_eq!(detail.items[0].item.id, i1.id);
    assert_eq!(detail.items[0].options.len(), 1);
    assert_eq!(detail.items[0].options[0].option.id, o1.id);
    assert_eq!(detail.items[0].options[0].annotations.len(), 1);
    assert_eq!(
        detail.items[0].options[0].annotations[0].owner,
        AnnotationOwner::Option { id: o1.id }
    );
    assert_eq!(detail.items[0].units.len(), 1);
    assert_eq!(detail.items[0].units[0].unit.id, u1.id);
    assert_eq!(detail.items[0].units[0].annotations.len(), 1);
    assert_eq!(
        detail.items[0].units[0].annotations[0].owner,
        AnnotationOwner::Unit { id: u1.id }
    );
    assert_eq!(detail.items[0].annotations.len(), 1);
    assert_eq!(
        detail.items[0].annotations[0].owner,
        AnnotationOwner::Item { id: i1.id }
    );

    assert_eq!(detail.items[1].item.id, i2.id);
    assert_eq!(detail.items[1].options.len(), 1);
    assert_eq!(detail.items[1].options[0].option.id, o2.id);
    assert_eq!(detail.items[1].options[0].annotations.len(), 0);
    assert_eq!(detail.items[1].units.len(), 0);
    assert_eq!(detail.items[1].annotations.len(), 0);
}
```

- [ ] **Step 2: Run the test to verify it fails to compile**

Run: `cargo test -p crf --lib usecase::tests::get_form_detail_assembles_tree_in_id_order`
Expected: compile error — `get_form_detail` is not a method on `CrfUsecase`.

- [ ] **Step 3: Implement `get_form_detail`**

In `lib/crates/crf/src/usecase/crf_usecase.rs`, after the `create_bulk_form` method (the `// ---- CrfForm ----` section ends with it), add:

```rust
/// Return every piece of state owned by this form (items
/// composed with their options / units / annotations, domain
/// annotations, and form-level annotations) in a single
/// response. Returns `UsecaseError::Repository(NotFound)`
/// (mapped to `CrfFormNotFound(form_id)` by the facade) if the
/// form does not exist.
///
/// Wave structure: 4 concurrent reads in wave 1, 3 in wave 2,
/// 1 in wave 3, 1 in wave 4 (9 queries, max 4 in flight).
/// Waves 2-4 are skipped entirely when their inputs are empty.
pub async fn get_form_detail(
    &self,
    form_id: i64,
) -> Result<CrfFormDetailView, UsecaseError> {
    // Wave 1: form + items + domain_annotations + form-level annotations.
    let (form, items, domain_annotations, form_annotations) = tokio::try_join!(
        self.form_repo.find_by_id(form_id),
        self.item_repo.list_by_form(form_id),
        self.domain_annotation_repo.list_by_form(form_id),
        self.annotation_repo.list_by_form(form_id),
    )?;
    // `form` is a CrfForm; converted to CrfFormView via `Into`
    // at the end of this function. `items` is `Vec<CrfItem>`;
    // `domain_annotations` is `Vec<DomainAnnotation>`;
    // `form_annotations` is `Vec<Annotation>`.

    if items.is_empty() {
        return Ok(CrfFormDetailView {
            form: form.into(),
            form_annotations: form_annotations
                .into_iter().map(Into::into).collect(),
            items: Vec::new(),
            domain_annotations: domain_annotations
                .into_iter().map(Into::into).collect(),
        });
    }

    let item_ids: Vec<i64> = items.iter().map(|i| i.id).collect();

    // Wave 2: options + units + item-level annotations.
    let (options, units, item_annotations) = tokio::try_join!(
        self.option_repo.list_by_items(&item_ids),
        self.unit_repo.list_by_items(&item_ids),
        self.annotation_repo.list_by_items(&item_ids),
    )?;

    // Build maps for O(1) parent lookups during assembly.
    use std::collections::HashMap;
    let mut options_by_item: HashMap<i64, Vec<CrfOption>> = HashMap::new();
    for o in options {
        options_by_item.entry(o.item_id).or_default().push(o);
    }
    let mut units_by_item: HashMap<i64, Vec<CrfUnit>> = HashMap::new();
    for u in units {
        units_by_item.entry(u.item_id).or_default().push(u);
    }
    let mut item_anns_by_item: HashMap<i64, Vec<Annotation>> = HashMap::new();
    for a in item_annotations {
        if let AnnotationOwner::Item { id } = a.owner {
            item_anns_by_item.entry(id).or_default().push(a);
        }
    }

    // Collect option / unit ids across all items for waves 3 & 4.
    let option_ids: Vec<i64> = options_by_item
        .values().flat_map(|v| v.iter().map(|o| o.id)).collect();
    let unit_ids: Vec<i64> = units_by_item
        .values().flat_map(|v| v.iter().map(|u| u.id)).collect();

    // Wave 3: option-level annotations.
    let option_anns = if option_ids.is_empty() {
        Vec::new()
    } else {
        self.annotation_repo.list_by_options(&option_ids).await?
    };
    let mut option_anns_by_option: HashMap<i64, Vec<Annotation>> = HashMap::new();
    for a in option_anns {
        if let AnnotationOwner::Option { id } = a.owner {
            option_anns_by_option.entry(id).or_default().push(a);
        }
    }

    // Wave 4: unit-level annotations.
    let unit_anns = if unit_ids.is_empty() {
        Vec::new()
    } else {
        self.annotation_repo.list_by_units(&unit_ids).await?
    };
    let mut unit_anns_by_unit: HashMap<i64, Vec<Annotation>> = HashMap::new();
    for a in unit_anns {
        if let AnnotationOwner::Unit { id } = a.owner {
            unit_anns_by_unit.entry(id).or_default().push(a);
        }
    }

    // Assemble. Items are already in `order ASC, id ASC` order
    // per the list_by_form contract; sort defensively.
    let mut sorted_items = items;
    sorted_items.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));

    let item_views = sorted_items
        .into_iter()
        .map(|item| {
            let mut opts = options_by_item.remove(&item.id).unwrap_or_default();
            opts.sort_by_key(|o| o.id);
            let mut uns = units_by_item.remove(&item.id).unwrap_or_default();
            uns.sort_by_key(|u| u.id);
            let mut item_anns = item_anns_by_item.remove(&item.id).unwrap_or_default();
            item_anns.sort_by_key(|a| a.id);

            let option_views = opts
                .into_iter()
                .map(|o| {
                    let mut anns = option_anns_by_option.remove(&o.id).unwrap_or_default();
                    anns.sort_by_key(|a| a.id);
                    CrfOptionDetailView { option: o.into(), annotations: anns.into_iter().map(Into::into).collect() }
                })
                .collect();
            let unit_views = uns
                .into_iter()
                .map(|u| {
                    let mut anns = unit_anns_by_unit.remove(&u.id).unwrap_or_default();
                    anns.sort_by_key(|a| a.id);
                    CrfUnitDetailView { unit: u.into(), annotations: anns.into_iter().map(Into::into).collect() }
                })
                .collect();

            CrfItemDetailView {
                item: item.into(),
                options: option_views,
                units: unit_views,
                annotations: item_anns.into_iter().map(Into::into).collect(),
            }
        })
        .collect();

    let mut sorted_domain_annotations = domain_annotations;
    sorted_domain_annotations.sort_by_key(|d| d.id);

    Ok(CrfFormDetailView {
        form: form.into(),
        form_annotations: form_annotations.into_iter().map(Into::into).collect(),
        items: item_views,
        domain_annotations: sorted_domain_annotations.into_iter().map(Into::into).collect(),
    })
}
```

Also add to the import line at the top of the file:

```rust
use super::views::{
    AnnotationView, CrfBulkFormResult, CrfFormDetailView, CrfFormView, CrfItemDetailView,
    CrfItemView, CrfOptionDetailView, CrfOptionView, CrfUnitDetailView, CrfUnitView,
    CrfVersionView, DomainAnnotationView,
};
```

And add the domain-side imports needed (look for what's already in scope; add `use crate::domain::{Annotation, AnnotationOwner, CrfOption, CrfUnit};` if not already present).

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p crf --lib usecase::tests::get_form_detail_assembles_tree_in_id_order`
Expected: PASS.

- [ ] **Step 5: Add the empty-form and missing-form tests**

Append two more tests in `lib/crates/crf/src/usecase/tests.rs`:

```rust
#[tokio::test]
async fn get_form_detail_empty_form_returns_empty_items() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = uc
        .create_form(CreateCrfForm {
            version_id: v.id,
            code: "F_E".into(),
            name: "Empty".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();

    let detail = uc.get_form_detail(f.id).await.unwrap();
    assert_eq!(detail.form.id, f.id);
    assert!(detail.items.is_empty());
    assert!(detail.form_annotations.is_empty());
    assert!(detail.domain_annotations.is_empty());
}

#[tokio::test]
async fn get_form_detail_missing_form_returns_not_found() {
    let uc = usecase();
    let err = uc.get_form_detail(9_999_999).await.unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::CrfFormNotFound(9_999_999))
    ));
}
```

Run: `cargo test -p crf --lib usecase::tests::get_form_detail`
Expected: 3 passing tests.

- [ ] **Step 6: Run the full crf suite**

Run: `cargo test -p crf && cargo clippy -p crf --all-targets --all-features -- -D warnings`
Expected: all green.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/crf/src/usecase/crf_usecase.rs \
        lib/crates/crf/src/usecase/tests.rs
git commit -m "feat(crf): add CrfUsecase::get_form_detail with 4-wave query plan

Returns one form together with items composed with their
options/units/annotations, domain annotations, and form-level
annotations. Uses tokio::try_join! for 4 concurrent wave-1 reads
(form, items, domain_annotations, form_annotations), 3 wave-2
reads (options, units, item_annotations), and single wave-3/4
reads for option/unit annotations. 9 queries max, 4 in flight.
Empty input short-circuits waves 2-4.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Add `CrfFormDetailView` + trait method to apis

**Files:**
- Modify: `lib/crates/apis/src/crf.rs` (add 4 view structs + new trait method)

- [ ] **Step 1: Add the four view structs**

In `lib/crates/apis/src/crf.rs`, append after the existing `BulkCreateCrfFormResult` (around line 183):

```rust
/// Composed view for [`CrfService::get_form_detail`]. Mirrors
/// `crf::usecase::CrfFormDetailView` — see that type for the
/// field-level semantics. The facade adapts `usecase::*DetailView`
/// into this shape via field-by-field `From` impls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfFormDetailView {
    pub form: CrfFormView,
    pub form_annotations: Vec<AnnotationView>,
    pub items: Vec<CrfItemDetailView>,
    pub domain_annotations: Vec<DomainAnnotationView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfItemDetailView {
    pub item: CrfItemView,
    pub options: Vec<CrfOptionDetailView>,
    pub units: Vec<CrfUnitDetailView>,
    pub annotations: Vec<AnnotationView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfOptionDetailView {
    pub option: CrfOptionView,
    pub annotations: Vec<AnnotationView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfUnitDetailView {
    pub unit: CrfUnitView,
    pub annotations: Vec<AnnotationView>,
}
```

- [ ] **Step 2: Add the request DTO (id-only) and the trait method**

In `lib/crates/apis/src/crf.rs`, append a new request DTO next to the other id-only DTOs (around line 310):

```rust
#[derive(Debug, Clone)]
pub struct GetCrfFormDetailRequest {
    pub form_id: i64,
}
```

In the `CrfService` trait, add the new method (in the `// ---- CrfForm ----` section, after `bulk_create_form` / `get_form_by_id`):

```rust
/// Return every piece of state owned by this form (items
/// composed with their options, units, and per-layer
/// annotations, plus the form's domain annotations and
/// form-level annotations) in a single response. Returns
/// `CrfApiError::CrfFormNotFound(form_id)` if the form does
/// not exist.
async fn get_form_detail(
    &self,
    req: GetCrfFormDetailRequest,
) -> Result<CrfFormDetailView, CrfApiError>;
```

- [ ] **Step 3: Build the workspace**

Run: `cargo build --workspace`
Expected: **compile failure** — `CrfServiceImpl` doesn't implement `get_form_detail` yet. That's expected; fix it in Task 7.

- [ ] **Step 4: Commit (compile failure is fine here — Task 7 fixes it)**

```bash
git add lib/crates/apis/src/crf.rs
git commit -m "feat(crf): add CrfFormDetailView and get_form_detail trait method

Four composed apis view structs mirroring the usecase types,
plus the new trait method. Wire format is owned by the
server-side dto.rs in a follow-up.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: Add the facade impl in `CrfServiceImpl`

**Files:**
- Modify: `lib/crates/crf/src/adapter/facade/in_memory/service.rs` (add `From` impls + trait method)

- [ ] **Step 1: Add the four `From` impls**

In `lib/crates/crf/src/adapter/facade/in_memory/service.rs`, append after the existing `From<crate::usecase::AnnotationView> for ApiAnnotationView` block (around line 773):

```rust
impl From<crate::usecase::CrfFormDetailView> for apis::crf::CrfFormDetailView {
    fn from(v: crate::usecase::CrfFormDetailView) -> Self {
        Self {
            form: v.form.into(),
            form_annotations: v.form_annotations.into_iter().map(Into::into).collect(),
            items: v.items.into_iter().map(Into::into).collect(),
            domain_annotations: v.domain_annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::usecase::CrfItemDetailView> for apis::crf::CrfItemDetailView {
    fn from(v: crate::usecase::CrfItemDetailView) -> Self {
        Self {
            item: v.item.into(),
            options: v.options.into_iter().map(Into::into).collect(),
            units: v.units.into_iter().map(Into::into).collect(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::usecase::CrfOptionDetailView> for apis::crf::CrfOptionDetailView {
    fn from(v: crate::usecase::CrfOptionDetailView) -> Self {
        Self {
            option: v.option.into(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::usecase::CrfUnitDetailView> for apis::crf::CrfUnitDetailView {
    fn from(v: crate::usecase::CrfUnitDetailView) -> Self {
        Self {
            unit: v.unit.into(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}
```

- [ ] **Step 2: Add the trait method impl**

In `lib/crates/crf/src/adapter/facade/in_memory/service.rs`, inside the `impl CrfService for CrfServiceImpl` block (in the `// ---- CrfForm ----` section, right after `bulk_create_form`):

```rust
async fn get_form_detail(
    &self,
    req: apis::crf::GetCrfFormDetailRequest,
) -> Result<apis::crf::CrfFormDetailView, CrfApiError> {
    self.usecase
        .get_form_detail(req.form_id)
        .await
        .map(Into::into)
        .map_err(map_error)
}
```

- [ ] **Step 3: Build the workspace — compile failure should be gone**

Run: `cargo build --workspace`
Expected: success.

- [ ] **Step 4: Run the test suite**

Run: `cargo test -p crf`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/crf/src/adapter/facade/in_memory/service.rs
git commit -m "feat(crf): wire CrfServiceImpl::get_form_detail + From impls

Four From impls adapt usecase::CrfFormDetailView and friends
into apis::crf::CrfFormDetailView and friends. The trait method
delegates to the usecase and maps errors via map_error.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 8: Add facade-level round-trip + missing-form tests

**Files:**
- Modify: `lib/crates/crf/src/adapter/facade/in_memory/tests.rs`

- [ ] **Step 1: Read the existing test patterns**

Open `lib/crates/crf/src/adapter/facade/in_memory/tests.rs` and find the existing `facade_bulk_create_form_round_trip` test. Mirror its setup (which constructs a `CrfServiceImpl` wired with the in-memory fakes and a `ProjectLookupMock`) — adopt that pattern verbatim.

- [ ] **Step 2: Add the round-trip test**

Append at the end of the file:

```rust
#[tokio::test]
async fn facade_get_form_detail_round_trip() {
    // Drives the public apis wire shape end-to-end and
    // confirms the projections on the response.
    use apis::crf::{BulkCreateCrfItemInput, CrfItemKind as ApiKind};

    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let bulk = svc
        .bulk_create_form(apis::crf::BulkCreateCrfFormRequest {
            form: apis::crf::CreateCrfFormRequest {
                version_id: v.id,
                code: "F1".into(),
                name: "Form 1".into(),
                order: 0,
                not_submitted: false,
            },
            items: vec![
                BulkCreateCrfItemInput {
                    item: apis::crf::CreateCrfItemRequest {
                        form_id: 0,
                        code: "I1".into(),
                        name: "Item 1".into(),
                        kind: ApiKind::Text,
                        order: 0,
                        not_submitted: false,
                    },
                    options: vec![apis::crf::CreateCrfOptionRequest {
                        item_id: 0,
                        value: "yes".into(),
                        not_submitted: false,
                    }],
                    units: vec![apis::crf::CreateCrfUnitRequest {
                        item_id: 0,
                        value: "mg".into(),
                        not_submitted: false,
                    }],
                },
                BulkCreateCrfItemInput {
                    item: apis::crf::CreateCrfItemRequest {
                        form_id: 0,
                        code: "I2".into(),
                        name: "Item 2".into(),
                        kind: ApiKind::Text,
                        order: 1,
                        not_submitted: false,
                    },
                    options: vec![],
                    units: vec![],
                },
            ],
        })
        .await
        .unwrap();
    let form_id = bulk.form.id;
    let item1_id = bulk.items[0].id;
    let item2_id = bulk.items[1].id;
    let d = svc
        .create_domain_annotation(CreateDomainAnnotationRequest {
            form_id,
            name: "Hint".into(),
            description: "".into(),
        })
        .await
        .unwrap();
    svc.create_annotation(CreateAnnotationRequest {
        domain_annotation_id: d.id,
        content: "form-level".into(),
        assign: false,
        owner: AnnotationOwner::Form(form_id),
    })
    .await
    .unwrap();
    svc.create_annotation(CreateAnnotationRequest {
        domain_annotation_id: d.id,
        content: "item-1".into(),
        assign: false,
        owner: AnnotationOwner::Item(item1_id),
    })
    .await
    .unwrap();

    let detail = svc
        .get_form_detail(apis::crf::GetCrfFormDetailRequest { form_id })
        .await
        .unwrap();

    assert_eq!(detail.form.id, form_id);
    assert_eq!(detail.items.len(), 2);
    assert_eq!(detail.items[0].item.id, item1_id);
    assert_eq!(detail.items[0].options.len(), 1);
    assert_eq!(detail.items[0].units.len(), 1);
    assert_eq!(detail.items[0].annotations.len(), 1);
    assert_eq!(detail.items[1].item.id, item2_id);
    assert_eq!(detail.items[1].options.len(), 0);
    assert_eq!(detail.form_annotations.len(), 1);
    assert_eq!(detail.domain_annotations.len(), 1);
}

#[tokio::test]
async fn facade_get_form_detail_missing_form() {
    let svc = service();
    let err = svc
        .get_form_detail(apis::crf::GetCrfFormDetailRequest { form_id: 9_999_999 })
        .await
        .unwrap_err();
    assert!(matches!(err, CrfApiError::CrfFormNotFound(9_999_999)));
}
```

- [ ] **Step 3: Run the new tests**

Run: `cargo test -p crf --lib adapter::facade::in_memory::tests::facade_get_form_detail`
Expected: 2 passing.

- [ ] **Step 4: Run clippy + the full suite**

Run: `cargo clippy -p crf --all-targets --all-features -- -D warnings && cargo test -p crf`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/crf/src/adapter/facade/in_memory/tests.rs
git commit -m "test(crf): add facade round-trip + missing-form tests for get_form_detail

End-to-end coverage through CrfServiceImpl. Confirms the
in-memory facade produces the same tree shape as the usecase
tests and surfaces CrfApiError::CrfFormNotFound on missing id.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 9: Add the wire DTOs and `From` impls

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs` (append after `BulkCreateCrfFormResponse`, around line 1674)

- [ ] **Step 1: Add the four wire DTOs**

In `apps/server/aegis-server/src/transport/http/dto.rs`, append:

```rust
/// Wire projection of [`apis::crf::CrfFormDetailView`]. Returned
/// by `GET /api/crf/forms/{id}/details`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormDetailResponse {
    pub form: CrfFormViewResponse,
    pub form_annotations: Vec<AnnotationViewResponse>,
    pub items: Vec<CrfItemDetailResponse>,
    pub domain_annotations: Vec<DomainAnnotationViewResponse>,
}

/// Wire projection of [`apis::crf::CrfItemDetailView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemDetailResponse {
    pub item: CrfItemViewResponse,
    pub options: Vec<CrfOptionDetailResponse>,
    pub units: Vec<CrfUnitDetailResponse>,
    pub annotations: Vec<AnnotationViewResponse>,
}

/// Wire projection of [`apis::crf::CrfOptionDetailView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionDetailResponse {
    pub option: CrfOptionViewResponse,
    pub annotations: Vec<AnnotationViewResponse>,
}

/// Wire projection of [`apis::crf::CrfUnitDetailView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfUnitDetailResponse {
    pub unit: CrfUnitViewResponse,
    pub annotations: Vec<AnnotationViewResponse>,
}

impl From<apis::crf::CrfFormDetailView> for CrfFormDetailResponse {
    fn from(v: apis::crf::CrfFormDetailView) -> Self {
        Self {
            form: v.form.into(),
            form_annotations: v.form_annotations.into_iter().map(Into::into).collect(),
            items: v.items.into_iter().map(Into::into).collect(),
            domain_annotations: v.domain_annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<apis::crf::CrfItemDetailView> for CrfItemDetailResponse {
    fn from(v: apis::crf::CrfItemDetailView) -> Self {
        Self {
            item: v.item.into(),
            options: v.options.into_iter().map(Into::into).collect(),
            units: v.units.into_iter().map(Into::into).collect(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<apis::crf::CrfOptionDetailView> for CrfOptionDetailResponse {
    fn from(v: apis::crf::CrfOptionDetailView) -> Self {
        Self {
            option: v.option.into(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<apis::crf::CrfUnitDetailView> for CrfUnitDetailResponse {
    fn from(v: apis::crf::CrfUnitDetailView) -> Self {
        Self {
            unit: v.unit.into(),
            annotations: v.annotations.into_iter().map(Into::into).collect(),
        }
    }
}
```

- [ ] **Step 2: Build the server crate**

Run: `cargo build -p aegis-server`
Expected: success.

- [ ] **Step 3: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs
git commit -m "feat(server): add CrfFormDetailResponse wire DTO and From impls

Four wire DTOs (camelCase, ToSchema) project apis::crf::
CrfFormDetailView and friends into JSON. Used by
GET /api/crf/forms/{id}/details.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 10: Add the HTTP handler `get_form_details`

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/crf/handlers.rs` (add after `get_form_by_id`, around line 376)

- [ ] **Step 1: Add the handler**

In `apps/server/aegis-server/src/transport/http/crf/handlers.rs`, append after the `get_form_by_id` function:

```rust
/// `GET /api/crf/forms/{id}/details` — return the form together
/// with every piece of state owned by it: items composed with
/// their options, units, and per-layer annotations; the form's
/// domain annotations; and form-level annotations. Single
/// response, up to nine DB round-trips with at most four in
/// flight via `tokio::try_join!`.
#[utoipa::path(
    get, path = "/forms/{id}/details", tag = "crf",
    operation_id = "crf_get_form_details",
    params(("id" = i64, Path, description = "CRF form id")),
    responses(
        (status = 200, description = "Form detail", body = dto::CrfFormDetailResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Form not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_form_details(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<Json<dto::CrfFormDetailResponse>, ApiError> {
    let view = state
        .crf
        .get_form_detail(apis::crf::GetCrfFormDetailRequest { form_id: id })
        .await?;
    Ok(Json(view.into()))
}
```

- [ ] **Step 2: Register the route**

In `apps/server/aegis-server/src/transport/http/crf/router.rs`, in the `// ---- CrfForm ----` block, after `.routes(routes!(handlers::get_form_by_id))`, add:

```rust
.routes(routes!(handlers::get_form_details))
```

Also update the URL map comment at the top of `router.rs`: insert `GET    /forms/{id}/details                              get_form_details` after the `GET    /forms/{id}` line, keeping the column alignment.

- [ ] **Step 3: Build the server crate**

Run: `cargo build -p aegis-server`
Expected: success.

- [ ] **Step 4: Run the server's existing tests**

Run: `cargo test -p aegis-server`
Expected: all green. (The utoipa annotation should be picked up by `openapi.rs` automatically; no manual edit to `openapi.rs` needed.)

- [ ] **Step 5: Verify the route appears in OpenAPI**

Run: `cargo run -p aegis-server -- --help` (or whichever flag dumps routes; consult the project's startup code if unsure). Confirm `crf_get_form_details` is listed under the `crf` tag. Alternatively, start the server briefly and curl `/api-docs/openapi.json` to confirm the path appears.

- [ ] **Step 6: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/crf/handlers.rs \
        apps/server/aegis-server/src/transport/http/crf/router.rs
git commit -m "feat(server): add GET /api/crf/forms/{id}/details handler + route

Thin handler delegates to state.crf.get_form_detail and projects
the apis view to dto::CrfFormDetailResponse. utoipa annotation
documents the 200/401/404/500 responses. Registered in the crf
sub-router alongside the existing form routes.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 11: Extend the public_api compile-only check

**Files:**
- Modify: `lib/crates/crf/tests/public_api.rs`

- [ ] **Step 1: Add the new types to the import block**

In `lib/crates/crf/tests/public_api.rs`, extend the `use crf::{ ... }` line to include the four new view types:

```rust
use crf::{
    Annotation, AnnotationOwner, AnnotationView, CrfBulkFormRepository, CrfForm, CrfFormDetailView,
    CrfFormNew, CrfFormRepository, CrfFormUpdate, CrfFormView, CrfItem, CrfItemDetailView,
    CrfItemKind, CrfItemNew, CrfItemRepository, CrfItemUpdate, CrfItemView, CrfOption, CrfOptionDetailView,
    CrfOptionNew, CrfOptionRepository, CrfOptionUpdate, CrfOptionView, CrfUnit, CrfUnitDetailView,
    CrfUnitNew, CrfUnitRepository, CrfUnitUpdate, CrfUnitView, CrfUsecase, CrfUsecaseConfig,
    CrfVersion, CrfVersionNew, CrfVersionRepository, CrfVersionUpdate, CrfVersionView,
    DomainAnnotation, DomainAnnotationNew, DomainAnnotationRepository, DomainAnnotationUpdate,
    DomainAnnotationView, DomainError, ProjectLookup, ProjectLookupImpl, UsecaseError,
};
```

- [ ] **Step 2: Add Send + Sync assertions for the new view types**

In the `view_dtos_are_send_and_sync` test, append:

```rust
    _assert_send_sync::<CrfFormDetailView>();
    _assert_send_sync::<CrfItemDetailView>();
    _assert_send_sync::<CrfOptionDetailView>();
    _assert_send_sync::<CrfUnitDetailView>();
```

- [ ] **Step 3: Run the public_api test**

Run: `cargo test -p crf --test public_api`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add lib/crates/crf/tests/public_api.rs
git commit -m "test(crf): extend public_api check with detail view types

Adds the four composed view types to the compile-only import
check and pins Send + Sync for them. Catches future breakage of
the public API surface.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 12: Add the live-DB integration test

**Files:**
- Modify: `lib/crates/crf/tests/integration_persistence.rs`

- [ ] **Step 1: Read the existing test pattern**

Open `lib/crates/crf/tests/integration_persistence.rs` and study the `polymorphic_owner_round_trip` test. It seeds a version + form + items + options + units + domain_annotation, then constructs annotations via each polymorphic constructor and inserts via direct SQL. Mirror that exact setup.

- [ ] **Step 2: Add the round-trip test**

Append at the end of `lib/crates/crf/tests/integration_persistence.rs`:

```rust
#[tokio::test]
#[ignore]
async fn get_form_detail_batch_ports_round_trip() {
    let pool = connect().await;
    let versions = CrfVersionRepoPg::new(pool.clone());
    let forms = CrfFormRepoPg::new(pool.clone());
    let items = CrfItemRepoPg::new(pool.clone());
    let options = CrfOptionRepoPg::new(pool.clone());
    let units = CrfUnitRepoPg::new(pool.clone());
    let domain_annotations = DomainAnnotationRepoPg::new(pool.clone());
    let annotations = AnnotationRepoPg::new(pool.clone());

    let suffix = unique_suffix();
    let v = versions
        .create(CrfVersionNew {
            project_code: format!("P_{suffix}"),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = forms
        .create(CrfFormNew {
            version_id: v.id,
            code: format!("F_{suffix}"),
            name: "F".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let i1 = items
        .create(CrfItemNew {
            form_id: f.id,
            code: format!("I1_{suffix}"),
            name: "I1".into(),
            kind: CrfItemKind::Selection,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let i2 = items
        .create(CrfItemNew {
            form_id: f.id,
            code: format!("I2_{suffix}"),
            name: "I2".into(),
            kind: CrfItemKind::Text,
            order: 1,
            not_submitted: false,
        })
        .await
        .unwrap();
    let o1 = options
        .create(CrfOptionNew { item_id: i1.id, value: "yes".into(), not_submitted: false })
        .await
        .unwrap();
    let u1 = units
        .create(CrfUnitNew { item_id: i1.id, value: "mg".into(), not_submitted: false })
        .await
        .unwrap();
    let d = domain_annotations
        .create(DomainAnnotationNew {
            form_id: f.id,
            name: format!("D_{suffix}"),
            description: "d".into(),
        })
        .await
        .unwrap();

    // Insert one annotation per layer (form / item / option / unit).
    for (owner_kind, id) in [
        (AnnotationOwner::Form { id: f.id }, f.id),
        (AnnotationOwner::Item { id: i1.id }, i1.id),
        (AnnotationOwner::Option { id: o1.id }, o1.id),
        (AnnotationOwner::Unit { id: u1.id }, u1.id),
    ] {
        let _annotation = match owner_kind {
            AnnotationOwner::Form { id } => Annotation::for_form(d.id, "x".into(), false, id).unwrap(),
            AnnotationOwner::Item { id } => Annotation::for_item(d.id, "x".into(), false, id).unwrap(),
            AnnotationOwner::Option { id } => Annotation::for_option(d.id, "x".into(), false, id).unwrap(),
            AnnotationOwner::Unit { id } => Annotation::for_unit(d.id, "x".into(), false, id).unwrap(),
        };
        // Use the repository's create path so it routes through the
        // owning owner correctly.
        annotations
            .create(AnnotationNew {
                domain_annotation_id: d.id,
                content: "x".into(),
                assign: false,
                owner: owner_kind,
            })
            .await
            .unwrap();
    }

    // Exercise every new batch port.
    let item_ids = vec![i1.id, i2.id];
    let batch_opts = options.list_by_items(&item_ids).await.unwrap();
    assert_eq!(batch_opts.len(), 1, "i1 has one option");
    assert_eq!(batch_opts[0].id, o1.id);

    let batch_units = units.list_by_items(&item_ids).await.unwrap();
    assert_eq!(batch_units.len(), 1);
    assert_eq!(batch_units[0].id, u1.id);

    let batch_item_anns = annotations.list_by_items(&item_ids).await.unwrap();
    assert_eq!(batch_item_anns.len(), 1);
    assert_eq!(batch_item_anns[0].owner, AnnotationOwner::Item { id: i1.id });

    let batch_opt_anns = annotations.list_by_options(&[o1.id]).await.unwrap();
    assert_eq!(batch_opt_anns.len(), 1);
    assert_eq!(batch_opt_anns[0].owner, AnnotationOwner::Option { id: o1.id });

    let batch_unit_anns = annotations.list_by_units(&[u1.id]).await.unwrap();
    assert_eq!(batch_unit_anns.len(), 1);
    assert_eq!(batch_unit_anns[0].owner, AnnotationOwner::Unit { id: u1.id });

    // Empty-input short-circuit.
    let empty_opts = options.list_by_items(&[]).await.unwrap();
    assert!(empty_opts.is_empty());
}

#[tokio::test]
#[ignore]
async fn get_form_detail_missing_form() {
    let pool = connect().await;
    let forms = CrfFormRepoPg::new(pool.clone());
    let result = forms.find_by_id(99_999_999).await;
    assert!(matches!(result, Err(DomainError::NotFound)));
}
```

The test imports `Annotation`, `AnnotationNew`, `AnnotationOwner`, `AnnotationRepoPg`, `CrfFormNew`, `CrfItemKind`, `CrfItemNew`, `CrfOptionNew`, `CrfUnitNew`, `CrfVersionNew`, `DomainAnnotationNew`, and the `*Repository` traits — all already imported at the top of the file, but verify before editing.

- [ ] **Step 3: Run the new tests against a live DB**

Set `AEGIS_DATABASE_URL` and run:
```bash
cargo test -p crf --test integration_persistence -- --ignored --test-threads=1
```
Expected: both new tests PASS.

- [ ] **Step 4: Commit**

```bash
git add lib/crates/crf/tests/integration_persistence.rs
git commit -m "test(crf): add live-DB round-trip + missing-form for batch ports

Exercises the five new batch port methods end-to-end against
Postgres. Asserts the assembled rows match what we seeded at
every layer, and confirms the empty-input short-circuit on
list_by_items.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 13: Final verification

**Files:** none — read-only verification.

- [ ] **Step 1: Format**

Run: `cargo fmt --all -- --check`
Expected: no diff.

- [ ] **Step 2: Lint the touched crates**

Run:
```bash
cargo clippy -p crf --all-targets --all-features -- -D warnings
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
cargo clippy -p apis --all-targets --all-features -- -D warnings
```
Expected: no warnings.

- [ ] **Step 3: Run the full Rust test suite**

Run: `cargo test --workspace`
Expected: all green.

- [ ] **Step 4: Run the live-DB integration suite**

Set `AEGIS_DATABASE_URL` and run:
```bash
cargo test -p crf -- --ignored --test-threads=1
```
Expected: all green.

- [ ] **Step 5: Build the docs**

Run: `cargo doc -p crf --no-deps`
Expected: success.

- [ ] **Step 6: Sanity-check the OpenAPI doc**

Start the server (`cargo run -p aegis-server`) and `curl http://localhost:8080/api-docs/openapi.json | jq '.paths."/api/crf/forms/{id}/details"'` to confirm the new route is registered with the correct operation_id (`crf_get_form_details`), request params, and response schemas. Stop the server when done.

- [ ] **Step 7: Final commit (if any)**

If step 1 or 2 required fixes, commit them with a `chore(crf): …` / `chore(server): …` message. Otherwise no commit.

```bash
git status  # confirm clean tree
```

---

## Out-of-scope reminders (do not implement in this plan)

- Changes to `list_items_by_form` to return a nested tree.
- ETag / `If-None-Match` caching.
- Cross-version rollups.
- Pagination on the detail response.

If a follow-up is needed, file a new spec — do not extend this plan.
