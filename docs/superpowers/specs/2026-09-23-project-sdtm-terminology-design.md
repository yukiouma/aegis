# Project Configuration — Optional SDTM Terminology Version Pointer

## Goal

Add a fourth optional control to a project's stored `configurations`
JSONB column: a **SDTM Terminology Version** pointer
(`{ versionId, versionName }`) that references a row in
`terminology::terminology_versions`. This is distinct from the
existing `sdtmig` pointer, which targets
`domain_model::sdtm_versions`. The two answer different questions:

- `sdtmig` — which SDTM **Implementation Guide** the project targets
  (e.g. "SDTMIG v3.4"). Already shipped.
- `sdtm_terminology` — which SDTM **controlled terminology** release
  the project uses (e.g. "2024-03-29"). New.

Both pointers travel together because the same project typically pins
both — but the spec keeps them as independent optional fields so a
project can set one without the other.

## Scope

Full-stack change:

- `lib/crates/project` — domain pointer type, configuration field,
  validation, repository migration.
- `lib/crates/apis` — port DTOs (data + view).
- `apps/server/aegis-server` — wire DTOs (request + response).
- `apps/desktop/aegis-desktop` — Rust HTTP mirror + TS types +
  `ConfigurationGeneralSection` picker + tests + i18n.

## Architecture

### Domain layer

New file `lib/crates/project/src/domain/terminology_version.rs`
holds `TerminologyVersionData` — sibling of `ModelVersion` but with
its own Rust type so the wire field name (`sdtm_terminology`) is
free to diverge from `sdtmig` without alias gymnastics, and so the
two pointer types can evolve validation independently:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyVersionData {
    pub version_id: i64,
    pub version_name: String,
}

impl TerminologyVersionData {
    pub fn new(version_id: i64, version_name: String) -> Result<Self, DomainError> { /* non-empty name */ }
    pub fn for_repository(version_id: i64, version_name: String) -> Self { /* bypass validation */ }
}
```

`ProjectConfiguration` grows a fourth optional field:

```rust
pub struct ProjectConfiguration {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<ProjectTag>,
    pub sdtmig: Option<ModelVersion>,
    pub sdtm_terminology: Option<TerminologyVersionData>,
}
```

`ProjectConfiguration::for_repository` accepts the new arg;
`Default` is still empty-everywhere.

`DomainError` gains one variant: `EmptySdtmTerminologyName`, parallel
to the existing `EmptySdtmigName`. The trust contract matches
`ModelVersion`: the domain does NOT verify the row exists in
`terminology.terminology_versions` at write time. Existing rows are
backfilled by the migration below.

`project_usecase::validate_configuration` grows a third arm:

```rust
if let Some(ref t) = c.sdtm_terminology {
    match TerminologyVersionData::new(t.version_id, t.version_name.clone()) {
        Ok(_) => {}
        Err(DomainError::EmptySdtmTerminologyName) =>
            return Err(UsecaseError::Validation(DomainError::EmptySdtmTerminologyName)),
        Err(other) => return Err(UsecaseError::Repository(other)),
    }
}
```

### Migration

`lib/crates/project/migrations/0004_add_project_configuration_sdtm_terminology.sql`:

```sql
UPDATE projects
SET configuration = configuration || '{"sdtm_terminology": null}'::jsonb;
ALTER TABLE projects
  ALTER COLUMN configuration
  SET DEFAULT '{"language": null, "tags": [], "sdtmig": null, "sdtm_terminology": null}'::jsonb;
```

Mirrors `0003_add_project_configuration_sdtmig.sql`. No schema change
to `projects` itself; the column stays JSONB and gains a key.

### Apis port

`lib/crates/apis/src/project.rs` adds two structs and one extra
field on each configuration shape:

```rust
pub struct TerminologyVersionData { pub version_id: i64, pub version_name: String }
pub struct TerminologyVersionView { pub version_id: i64, pub version_name: String }

pub struct ProjectConfigurationData {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagData>,
    pub sdtmig: Option<ModelVersionData>,
    pub sdtm_terminology: Option<TerminologyVersionData>,
}

pub struct ProjectConfigurationView {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagView>,
    pub sdtmig: Option<ModelVersionView>,
    pub sdtm_terminology: Option<TerminologyVersionView>,
}
```

### Server wire DTOs

`apps/server/aegis-server/src/transport/http/dto.rs` adds the wire
pair and extends both configuration shapes:

```rust
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionRequest { pub version_id: i64, pub version_name: String }

#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionResponse { pub version_id: i64, pub version_name: String }
```

`ProjectConfigurationDataRequest` and `ProjectConfigurationViewResponse`
gain `sdtm_terminology: Option<TerminologyVersionRequest/Response>`,
both with `#[serde(default, skip_serializing_if = "Option::is_none")]`
so a present-but-`None` field round-trips cleanly.

Wire field name: `sdtmTerminology` (camelCase from the snake_case
identifier — auto via `rename_all = "camelCase"`). `From` impls
between wire ↔ apis thread the field through verbatim.

### Desktop HTTP mirror

`apps/desktop/aegis-desktop/src-tauri/src/http/project.rs` mirrors
the server's wire DTOs (`SdtmTerminologyVersionRequest` /
`SdtmTerminologyVersionResponse`) and extends both
`ProjectConfigurationDataRequest` and
`ProjectConfigurationViewResponse` with `sdtm_terminology: Option<_>`.

A round-trip unit test pins the wire shape
(`{"sdtmTerminology":{"versionId":3,"versionName":"..."}}`) and the
empty-config serialises-as-`{}` invariant.

### Desktop TS types

`apps/desktop/aegis-desktop/src/shared/api/types.ts`:

```ts
export interface ProjectConfigurationSdtmTerminology {
  versionId: number;
  versionName: string;
}

export interface ProjectConfiguration {
  language: ProjectLanguage | null;
  tags: Tag[];
  sdtmig?: ProjectConfigurationSdtmig | null;
  sdtmTerminology?: ProjectConfigurationSdtmTerminology | null;
}
```

### Desktop UI

`apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx`
gains a sibling `<FormControl>` for the SDTM Terminology picker,
following the exact pattern of the existing `sdtmig` picker:

```
┌─ ConfigurationGeneralSection ────────────────────────────────┐
│                                                             │
│  Preferred language     [ English ▾ ]                       │
│                                                             │
│  SDTMIG Version          [ SDTMIG v3.4 ▾ ]                  │
│                                                             │
│  SDTM Terminology       [ 2024-03-29 ▾ ]   ← new           │
│                                                             │
│  Tags           (TagEditor)                              │
│                                                             │
│                                       [ Save ]              │
└─────────────────────────────────────────────────────────────┘
```

Local state, touched ref, dirty flag, and save body extend in
parallel:

```tsx
const [sdtmTerminology, setSdtmTerminology] =
  useState<ProjectConfigurationSdtmTerminology | null>(
    initial.sdtmTerminology ?? null,
  );
const sdtmTerminologyTouchedRef = useRef(false);

const terminologyVersions = useListTerminologyVersions();
// Filter to SDTM in-flight so the picker only shows SDTM releases
// even though the backend mixes Sdtm + Adam.
const sdtmReleases = (terminologyVersions.data ?? [])
  .filter((v) => v.kind === "sdtm");
```

Save body becomes
`{ language, tags, sdtmig, sdtmTerminology }`. The reseed
`useEffect` resets the new touched flag and resnaps `sdtmTerminology`
on every fresh `initial`.

`ProjectDrawer` (the create drawer) is intentionally **not** touched —
it already doesn't expose `sdtmig` either, on the convention that
"general settings get filled in on the workspace". Adding the
terminology picker there is out of scope.

### i18n

`lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts`:

```
'project.configuration.general.sdtmTerminology': 'SDTM Terminology' / 'SDTM 术语'
'project.configuration.general.sdtmTerminology.none': '(unspecified)' / '（未指定）'
```

## Files

### New

- `lib/crates/project/src/domain/terminology_version.rs`
- `lib/crates/project/migrations/0004_add_project_configuration_sdtm_terminology.sql`

### Modified

Rust:

- `lib/crates/project/src/domain.rs` — module + re-export.
- `lib/crates/project/src/domain/error.rs` — new variant.
- `lib/crates/project/src/domain/project_configuration.rs` — new
  field + `for_repository`.
- `lib/crates/project/src/usecase/project_usecase.rs` —
  `validate_configuration` arm.
- `lib/crates/project/src/usecase/views.rs` — `ProjectConfigurationView`
  gains the field; `From<ProjectConfiguration>` includes it.
- `lib/crates/project/src/adapter/facade/in_memory/service.rs` —
  data-to-domain and view-to-apis `From` impls thread the field;
  `configuration_data_to_domain` accepts it.
- `lib/crates/apis/src/project.rs` — new structs + field on
  configuration data/view.
- `apps/server/aegis-server/src/transport/http/dto.rs` — wire DTOs +
  `From` impls.
- `apps/server/aegis-server/src/transport/http/project/handlers.rs` —
  no source change required; the existing
  `ProjectConfigurationDataRequest::from(req.configurations)` call
  in `configuration_data` already threads every field on the wire
  DTO through `From<…for apis>`, so the new `sdtm_terminology`
  field is carried automatically once `dto.rs` adds it.
- `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs` — Rust
  HTTP mirror + round-trip unit test.

TS:

- `apps/desktop/aegis-desktop/src/shared/api/types.ts` — new
  interface + field on `ProjectConfiguration`.
- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx`
  — new state, picker, dirty-flag, save body, reseed.
- `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx`
  — focused cases for the new picker.
- `lib/packages/ui/src/i18n/locales/en.ts` — two keys.
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — two keys.

## Wire shape (delta)

Request body (`CreateProjectRequest.configurations` /
`UpdateProjectRequest.configurations`):

```jsonc
{
  "language": "en",                 // unchanged
  "tags": [{"key":"Product","value":"DEMO-001"}],
  "sdtmig": {"versionId": 3, "versionName": "SDTMIG v3.4"},
  "sdtmTerminology": {"versionId": 7, "versionName": "2024-03-29"}
}
```

Both new fields use `skip_serializing_if = "Option::is_none"` so a
`None` selection drops the key entirely — the existing "no change"
branch on the server side carries through.

## State + save semantics

Mirrors the existing `sdtmig` pattern in
`ConfigurationGeneralSection`:

```tsx
const sdtmTerminologyTouchedRef = useRef(false);

const dirty =
  languageTouchedRef.current ||
  tagsTouchedRef.current ||
  sdtmTouchedRef.current ||
  sdtmTerminologyTouchedRef.current;

const configurations: ProjectConfiguration = {
  language,
  tags,
  sdtmig,
  sdtmTerminology,
};
await update.mutateAsync({ code: projectCode, body: { configurations } });
```

Server-side the new field threads through unchanged: `apis` sees it
as `Option<TerminologyVersionData>`, the project domain validates the
non-empty `version_name` rule, and the JSONB column persists it as
the `sdtm_terminology` key.

## Tests

### Rust

- `lib/crates/project/src/domain/tests.rs` — add a case that
  `TerminologyVersionData::new` rejects empty/whitespace
  `version_name` (`DomainError::EmptySdtmTerminologyName`) and that
  `for_repository` bypasses validation.
- `lib/crates/project/src/usecase/tests.rs` — extend the existing
  `validate_configuration` tests with a sdtm_terminology arm:
  valid pointer is accepted, empty-name pointer becomes
  `UsecaseError::Validation(EmptySdtmTerminologyName)`, `None` is
  a no-op.
- `lib/crates/project/src/adapter/facade/in_memory/tests.rs` — a
  round-trip test that a project configuration with both `sdtmig`
  and `sdtm_terminology` survives the facade
  `data → domain → usecase → view → apis` chain.
- `lib/crates/project/src/adapter/persistence/postgres/tests.rs`
  (the `#[ignore]` live-DB suite) — a round-trip write/read that
  asserts the new field survives JSONB materialisation.
- `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs::tests` —
  round-trip JSON for the new shape; serialise-empty-stays-`{}`.

### TS

`apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx`
gets focused cases mirroring the sdtmig tests:

| Case | Expectation |
| --- | --- |
| Renders the SDTM Terminology Select with the current selection when `initial.sdtmTerminology` is present | The Select's value label shows the saved `versionName`. |
| Renders an "(unspecified)" item as the first option | `screen.findByText("(unspecified)")` is present. |
| Picking a version enables Save | After change, `config-general-save` is no longer disabled. |
| Picking "(unspecified)" → Save → `configurations.sdtmTerminology === null` | `invoke` last call's `body.configurations.sdtmTerminology` is `null`. |
| Save with terminology change sends `{ language, tags, sdtmig, sdtmTerminology }` | `expect.objectContaining(...)`. |
| Non-leader: SDTM Terminology Select is disabled | The Select's form control has `Mui-disabled`. |
| Picker filters to SDTM only (Adam versions are absent) | `list_terminology_versions` mock returns a mix; only `kind: "sdtm"` rows render as MenuItems. |

Existing test "Save fires update_project with configurations { language, tags, sdtmig }" extends its `expect.objectContaining` to also assert `sdtmTerminology` for clarity.

The `mockCommands` block in the test gains a `list_terminology_versions`
handler returning one SDTM release so MenuItems render.

## Verification gate

```bash
cargo check --workspace
cargo test -p project
cargo test -p aegis-server --lib transport::http::project::handlers::tests
cargo test -p aegis-desktop --lib http::project::tests
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter @aegis/ui typecheck
```

Live-DB tests are excluded from the gate per the project's
convention; rerun them manually with
`cargo test -p project -- --ignored --test-threads=1`.

## Out of scope

- Showing the SDTM Terminology version's full details (codelist
  counts, etc.) in the picker. The picker is intentionally a flat
  list of `{id, name}` like the SDTMIG one.
- A way to *create* a new terminology version from the configuration
  surface. Versions already have their own page under
  `features/terminology`.
- A "search" affordance. The list is short (a few dozen releases at
  most); a flat `<Select>` matches the SDTMIG UX. Swap to
  `<Autocomplete>` only if it ever grows past ~20 items.
- Filtering by `kind` other than `sdtm`. A project never carries an
  ADaM terminology pointer today, and there's no on-the-wire kind
  field — adding one would be a wire-shape change.
- `ProjectDrawer` (the create drawer) — the existing convention is
  that the create form leaves `sdtmig` unset. Adding the terminology
  picker there is a separate UX change.
- A new apis method on the project service. The project service
  already carries the configuration as opaque JSONB; the only
  validation is the non-empty-name rule, which the usecase enforces
  before persistence.

## Commits

5 commits in dependency order:

1. **feat(project):** domain pointer type, configuration field,
   validation, migration, in-memory facade plumbing. Includes the
   new variant on `DomainError` and the new domain-side `View`.
2. **feat(apis):** port DTOs in `apis/src/project.rs`. (Depends on
   #1 for the field naming.)
3. **feat(server):** wire DTOs in
   `apps/server/aegis-server/src/transport/http/dto.rs`. Includes the
   new request/response shapes and `From` impls; the existing
   handler `configuration_data` is unchanged (it goes through the
   `From` impl).
4. **feat(desktop-tauri):** Rust HTTP mirror in
   `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs` plus
   round-trip tests.
5. **feat(desktop):** TS types, `ConfigurationGeneralSection` picker,
   i18n keys (en + zhCN), and focused vitest cases.

Each commit is independently compilable; types line up across the
crates in lock-step.
