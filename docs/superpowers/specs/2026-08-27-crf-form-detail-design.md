# CRF Form Detail Endpoint — Design

## Goal

Add a single new endpoint `GET /api/crf/forms/{id}/details` that returns one
CRF form together with every piece of state owned by that form, in a single
response. The existing scalar `GET /api/crf/forms/{id}` (returning the flat
`CrfFormView`) stays untouched.

This complements the existing per-aggregate list endpoints
(`list_items_by_form`, `list_options_by_item`, `list_domain_annotations_by_form`,
`list_annotations_by_form`, etc.) — those stay for clients that only need one
slice at a time. The detail endpoint is the "load the whole form" path the
desktop client (and any future "edit a form" flow) needs.

Out of scope: parallel changes to `list_items_by_form` to make *that* endpoint
return a nested tree. That is a separate ticket; this spec touches only the
new detail endpoint.

## Architecture

The new method follows the existing four-layer convention exactly — the same
shape as `bulk_create_form`, but for read:

- **Domain** — five new batch port methods on the existing repository traits.
  No new traits, no new error variants, no new aggregates.
- **Usecase** — one new method `get_form_detail(form_id)`. Owns the wave
  structure (parallel queries, then tree assembly in memory).
- **Adapter (facade + persistence)** — concrete implementations of the five
  new batch methods on `CrfServiceImpl`, `CrfOptionRepoPg`, `CrfUnitRepoPg`,
  and `AnnotationRepoPg`.
- **apis** — one new trait method, four new view structs.
- **Server (HTTP)** — one new handler, four new wire DTOs, one new route.

## Response shape

Annotations are nested under their owner (your earlier choice). The form
carries its own annotations; each item carries its options, units, and
item-level annotations; each option / unit carries its own annotations.

```text
CrfFormDetailView
├── form                      : CrfFormView                // scalar form
├── form_annotations          : Vec<AnnotationView>        // AnnotationOwner::Form
├── items                     : Vec<CrfItemDetailView>
│   ├── item                  : CrfItemView
│   ├── options               : Vec<CrfOptionDetailView>
│   │   ├── option            : CrfOptionView
│   │   └── annotations       : Vec<AnnotationView>        // AnnotationOwner::Option
│   ├── units                 : Vec<CrfUnitDetailView>
│   │   ├── unit              : CrfUnitView
│   │   └── annotations       : Vec<AnnotationView>        // AnnotationOwner::Unit
│   └── annotations           : Vec<AnnotationView>        // AnnotationOwner::Item
└── domain_annotations        : Vec<DomainAnnotationView>
```

Sorting:

- `items` ordered `order ASC, id ASC` (matches `list_items_by_form`).
- `options` / `units` / every `annotations` vec ordered `id ASC` (matches every
  other list endpoint in the surface).

Empty: a form with no items / no options / no units / no annotations / no
domain annotations returns the corresponding empty vec, not `null`.

## View types

Three copies of the same shape — one per layer — each `From`-converted at the
boundary.

Conversion pattern (note: differs from the existing scalar views, which have a
domain aggregate on the left side of `From`):

- No `From<Domain>` impl for the `*DetailView` types. The domain has no
  aggregate that carries the nested tree; the usecase composes the detail
  types in memory during `get_form_detail`.
- `From<crate::usecase::CrfFormDetailView> for apis::crf::CrfFormDetailView`
  (and the three nested `*DetailView` impls) — field-by-field, living in
  `crf::adapter::facade::in_memory::service.rs` next to the existing
  `From<crate::usecase::CrfFormView> for ApiCrfFormView` block.
- `From<apis::crf::CrfFormDetailView> for dto::CrfFormDetailResponse`
  (and the three nested impls) — field-by-field, living in `dto.rs` next to
  the existing `From<apis::crf::BulkCreateCrfFormResult> for BulkCreateCrfFormResponse`
  impl.

### Usecase (`lib/crates/crf/src/usecase/views.rs`, new types)

```rust
pub struct CrfFormDetailView {
    pub form: CrfFormView,
    pub form_annotations: Vec<AnnotationView>,
    pub items: Vec<CrfItemDetailView>,
    pub domain_annotations: Vec<DomainAnnotationView>,
}

pub struct CrfItemDetailView {
    pub item: CrfItemView,
    pub options: Vec<CrfOptionDetailView>,
    pub units: Vec<CrfUnitDetailView>,
    pub annotations: Vec<AnnotationView>,
}

pub struct CrfOptionDetailView {
    pub option: CrfOptionView,
    pub annotations: Vec<AnnotationView>,
}

pub struct CrfUnitDetailView {
    pub unit: CrfUnitView,
    pub annotations: Vec<AnnotationView>,
}
```

`CrfFormDetailView` and friends gain `pub use` re-exports at the crate root in
`lib.rs` (matching the existing pattern for `CrfBulkFormResult`).

### apis (`lib/crates/apis/src/crf.rs`, new types + new trait method)

```rust
pub struct CrfFormDetailView {
    pub form: CrfFormView,
    pub form_annotations: Vec<AnnotationView>,
    pub items: Vec<CrfItemDetailView>,
    pub domain_annotations: Vec<DomainAnnotationView>,
}
pub struct CrfItemDetailView   { /* mirrors usecase */ }
pub struct CrfOptionDetailView { /* mirrors usecase */ }
pub struct CrfUnitDetailView   { /* mirrors usecase */ }

#[async_trait]
pub trait CrfService: Send + Sync {
    // ... existing methods unchanged ...

    /// Return every piece of state owned by this form (items +
    /// their options / units / annotations, domain annotations, and
    /// form-level annotations) in a single response. Returns
    /// `CrfApiError::CrfFormNotFound(id)` if the form does not exist.
    async fn get_form_detail(
        &self,
        form_id: i64,
    ) -> Result<CrfFormDetailView, CrfApiError>;
}
```

No new `CrfApiError` variant — `CrfFormNotFound(i64)` (already present) is the
only error path.

### Server wire DTOs (`apps/server/aegis-server/src/transport/http/dto.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfFormDetailResponse {
    pub form: CrfFormViewResponse,
    pub form_annotations: Vec<AnnotationViewResponse>,
    pub items: Vec<CrfItemDetailResponse>,
    pub domain_annotations: Vec<DomainAnnotationViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfItemDetailResponse {
    pub item: CrfItemViewResponse,
    pub options: Vec<CrfOptionDetailResponse>,
    pub units: Vec<CrfUnitDetailResponse>,
    pub annotations: Vec<AnnotationViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CrfOptionDetailResponse {
    pub option: CrfOptionViewResponse,
    pub annotations: Vec<AnnotationViewResponse>,
}

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
// + From impls for the three nested view types — trivial field-by-field conversions.
```

All wire fields use `camelCase` via `#[serde(rename_all = "camelCase")]`,
matching every other response DTO in the file.

## New domain port methods

Five new methods on three existing traits. No new traits.

```rust
// domain/crf_option.rs
#[async_trait]
pub trait CrfOptionRepository: Send + Sync {
    // ... existing methods unchanged ...
    /// Batch fetch every option whose `item_id` is in `item_ids`.
    /// Returns `Ok(Vec::new())` for empty input without hitting the DB.
    async fn list_by_items(
        &self,
        item_ids: &[i64],
    ) -> Result<Vec<CrfOption>, DomainError>;
}

// domain/crf_unit.rs
#[async_trait]
pub trait CrfUnitRepository: Send + Sync {
    /// Batch fetch every unit whose `item_id` is in `item_ids`.
    /// Returns `Ok(Vec::new())` for empty input without hitting the DB.
    async fn list_by_items(
        &self,
        item_ids: &[i64],
    ) -> Result<Vec<CrfUnit>, DomainError>;
}

// domain/annotation.rs
#[async_trait]
pub trait AnnotationRepository: Send + Sync {
    // ... existing methods unchanged ...
    /// Batch fetch every annotation owned by an item in `item_ids`
    /// (i.e. `item_id IN (...)` and the other three FK columns null).
    /// Returns `Ok(Vec::new())` for empty input.
    async fn list_by_items(
        &self,
        item_ids: &[i64],
    ) -> Result<Vec<Annotation>, DomainError>;
    /// Batch fetch every annotation owned by an option in `option_ids`.
    /// Returns `Ok(Vec::new())` for empty input.
    async fn list_by_options(
        &self,
        option_ids: &[i64],
    ) -> Result<Vec<Annotation>, DomainError>;
    /// Batch fetch every annotation owned by a unit in `unit_ids`.
    /// Returns `Ok(Vec::new())` for empty input.
    async fn list_by_units(
        &self,
        unit_ids: &[i64],
    ) -> Result<Vec<Annotation>, DomainError>;
}
```

The three batch `AnnotationRepository` methods each filter to the matching
owner column only (`item_id IN (...)` etc.) — they do NOT union across
owner kinds. That's the same shape as the existing single-id
`list_by_item` / `list_by_option` / `list_by_unit` methods, just batched.

## Usecase method

`CrfUsecase::get_form_detail(form_id: i64) -> Result<CrfFormDetailView, UsecaseError>`.

The wave structure uses `tokio::try_join!` (already in the dependency tree):

```text
Wave 1 (4 concurrent):
  - form_repo.find_by_id(form_id)
  - item_repo.list_by_form(form_id)
  - domain_annotation_repo.list_by_form(form_id)
  - annotation_repo.list_by_form(form_id)            // form-level annotations

Wave 2 (3 concurrent, after Wave 1):
  - option_repo.list_by_items(&item_ids)
  - unit_repo.list_by_items(&item_ids)
  - annotation_repo.list_by_items(&item_ids)          // item-level annotations

Wave 3 (1 query, after Wave 2):
  - annotation_repo.list_by_options(&option_ids)

Wave 4 (1 query, after Wave 3):
  - annotation_repo.list_by_units(&unit_ids)
```

If `form_repo.find_by_id` returns `Err(DomainError::NotFound)`, the whole
`try_join!` returns `Err` early (the other three wave-1 queries are
cancelled). That `Err(NotFound)` is mapped to `UsecaseError::Repository(NotFound)`
via the existing `From<DomainError> for UsecaseError` impl, which the facade
maps to `CrfApiError::CrfFormNotFound(form_id)` via the existing
`map_domain_error` table.

Empty-input short-circuits: if `items.is_empty()`, the usecase skips waves 2-4
entirely (no DB round-trips for empty subtrees). Same for empty `option_ids`
in wave 3 and empty `unit_ids` in wave 4.

Tree assembly:

1. `form_annotations` ← wave-1 `annotation_repo.list_by_form(form_id)`.
2. `domain_annotations` ← wave-1 `domain_annotation_repo.list_by_form(form_id)`,
   sorted `id ASC`.
3. Build `options_by_item: HashMap<i64, Vec<CrfOption>>` from wave-2.
4. Build `units_by_item: HashMap<i64, Vec<CrfUnit>>` from wave-2.
5. Build `item_annotations_by_item: HashMap<i64, Vec<Annotation>>` from wave-2.
6. Build `option_annotations_by_option: HashMap<i64, Vec<Annotation>>` from wave-3.
7. Build `unit_annotations_by_unit: HashMap<i64, Vec<Annotation>>` from wave-4.
8. Walk `items` in `order ASC, id ASC` order, look up children in the four
   maps, sort each child list `id ASC`, and assemble `CrfItemDetailView`s.

The `order ASC, id ASC` item sort is applied once at the start of step 8 (the
items returned by `item_repo.list_by_form` are already in that order per the
existing port contract, but the sort is re-applied as a defensive check).

## Adapter impls

### Postgres (`adapter/persistence/postgres/*_repo.rs`)

Each batch method runs a single SQL query using `WHERE <fk> = ANY($1)`:

```rust
// crf_option_repo.rs
async fn list_by_items(
    &self,
    item_ids: &[i64],
) -> Result<Vec<CrfOption>, DomainError> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<CrfOptionRow> = sqlx::query_as(
        "SELECT id, item_id, value, not_submitted, created_at, updated_at \
         FROM crf_options WHERE item_id = ANY($1) ORDER BY id ASC",
    )
    .bind(item_ids)
    .fetch_all(&self.pool)
    .await
    .map_err(map_db_err)?;
    Ok(rows.into_iter().map(Into::into).collect())
}
```

`crf_unit_repo.rs` and the three `annotation_repo.rs` methods follow the same
shape. The `Annotation` queries filter on the appropriate owner column only
(`item_id`, `option_id`, `unit_id`) — never `UNION` across owner kinds; that
mirrors the single-id methods.

`map_db_err` is unchanged. No new SQL migrations.

### In-memory (`adapter/facade/in_memory/service.rs`)

Each batch method iterates the in-memory `Vec` and filters:

```rust
async fn list_by_items(
    &self,
    item_ids: &[i64],
) -> Result<Vec<CrfOption>, DomainError> {
    Ok(self
        .options
        .iter()
        .filter(|o| item_ids.contains(&o.item_id))
        .cloned()
        .collect())
}
```

(Exact implementation depends on the in-memory fake shape — the principle is
the same.) The `CrfServiceImpl` adds the trait method `get_form_detail` that
delegates to `self.usecase.get_form_detail(form_id)` and maps the result
through `Into<apis::crf::CrfFormDetailView>` + `map_error`.

## HTTP handler

`apps/server/aegis-server/src/transport/http/crf/handlers.rs` gets one new
handler, slotted between `get_form_by_id` and `update_form`:

```rust
/// `GET /api/crf/forms/{id}/details` — return the form together with
/// every piece of state owned by it: items composed with their
/// options, units, and per-layer annotations; the form's domain
/// annotations; and form-level annotations. Single response, six
/// (sometimes fewer) DB round-trips with up to four in flight.
#[utoipa::path(
    get, path = "/forms/{id}/details", tag = "crf",
    operation_id = "crf_get_form_details",
    params(("id" = i64, Path, description = "CRF form id")),
    responses(
        (status = 200, description = "Form detail", body = dto::CrfFormDetailResponse),
        (status = 401, description = "Missing / invalid token", body = ErrorBody),
        (status = 404, description = "Form not found", body = ErrorBody),
        (status = 500, description = "Repository failure", body = ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_form_details(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<Json<dto::CrfFormDetailResponse>, ApiError> {
    let view = state.crf.get_form_detail(id).await?;
    Ok(Json(view.into()))
}
```

Mounted in `transport/http/crf/router.rs` alongside the existing form routes:

```rust
.routes(routes!(handlers::get_form_details))
```

The URL map comment block at the top of `router.rs` gets a new line:

```text
//! - `GET    /forms/{id}/details`                        get_form_details
```

`apps/server/aegis-server/src/transport/http/openapi.rs` is regenerated
automatically by the `utoipa` macros — no manual edit needed beyond including
the new response types in the `#[derive(ToSchema)]` derives (already done in
the dto.rs additions above).

## Tests

1. **Domain unit tests** (in `src/domain/tests.rs`, alongside the existing
   `Annotation::for_*` tests) — none added. The new port methods are pure
   adapters of existing port behavior; the existing
   `migration_file_is_present_and_idempotent` tests on the postgres repos
   still cover the schema surface.

2. **Adapter unit tests** (in
   `src/adapter/persistence/postgres/*_repo.rs`) — one new test per
   `list_by_items` / `list_by_options` / `list_by_units` impl, named
   `list_by_X_empty_input_returns_empty_vec` (asserts the empty-input
   short-circuit without hitting the DB).

3. **Facade unit tests** (`src/adapter/facade/in_memory/tests.rs`):
   - `facade_get_form_detail_round_trip` — seed a form with two items,
     each with options + units + annotations at every layer, plus one
     domain annotation, plus one form-level annotation. Call
     `CrfServiceImpl::get_form_detail(form_id)`, assert the assembled tree.
   - `facade_get_form_detail_missing_form` — call on a non-existent id,
     assert `Err(CrfApiError::CrfFormNotFound(id))`.

4. **Usecase unit tests** (`src/usecase/tests.rs`):
   - `get_form_detail_assembles_tree_in_id_order` — seed with fakes, assert
     item / option / unit / annotation ordering matches the contract
     (`order ASC, id ASC` for items; `id ASC` for everything else).
   - `get_form_detail_empty_form` — form with no items returns empty `items`
     vec (and waves 2-4 are skipped — verified by counting fake-repo
     invocations).
   - `get_form_detail_missing_form` — fake `form_repo.find_by_id` returns
     `Err(DomainError::NotFound)`, assert `Err(UsecaseError::Repository(...))`.

5. **`tests/public_api.rs`** — adds the four new view types and the new trait
   method to the compile-only import check.

6. **`tests/integration_persistence.rs`** (`#[ignore]`-gated, live DB):
   - `get_form_detail_round_trip` — seed a form with two items (one
     `Selection`, one `Text`), each with options / units / annotations
     across all four layers + one domain annotation + one form-level
     annotation. Call the new batch port methods directly (this is a
     persistence-layer test, not a facade test, per the existing
     convention in this file). Assert the assembled tree matches.
   - `get_form_detail_missing_form` — call on a non-existent form id, assert
     `DomainError::NotFound`.

## Wiring and verification gate

```bash
cargo fmt --all -- --check
cargo clippy -p crf --all-targets --all-features -- -D warnings
cargo test -p crf
cargo doc -p crf --no-deps
cargo test -p crf -- --ignored --test-threads=1   # needs AEGIS_DATABASE_URL
cargo check --workspace
```

`lib.rs` re-exports add the four new view types (usecase-side) alongside the
existing `CrfBulkFormResult` export, so the consumer can name them via
`crf::CrfFormDetailView` etc.

## Out of scope

- Changes to `list_items_by_form` to make *that* endpoint return a nested tree.
- Changes to `list_annotations_by_form` to nest annotations under their owners.
- New search endpoints scoped to the form detail.
- ETag / `If-None-Match` caching on the detail endpoint (defer until a client
  demonstrates a hot-loop that benefits).
- Filtering or pagination on the detail response — clients that want one
  slice call the existing single-id list endpoints instead.
