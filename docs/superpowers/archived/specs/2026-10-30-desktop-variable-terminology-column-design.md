# Aegis Desktop — SDTM variable Terminology column — design

- **Date:** 2026-10-30
- **Branch:** `feat/desktop_sdtm-variable-terminology`
- **Status:** approved (design review pending)
- **Scope:** `apps/desktop/aegis-desktop` (Rust transport + TS frontend), `lib/packages/ui` (i18n resources)
- **Follows:** `docs/superpowers/specs/2026-10-29-sdtm-variable-controlled-design.md` (the server-side wire change this desktop turn adopts)

---

## Intent

Two things, one change.

1. **Close the wire skew.** The server now sends `variable_controlled` as
   `Option<SdtmVariableControlled>` — an object with two optional string
   lists — instead of the legacy `Option<String>`. The desktop crate
   still mirrors the old shape, so the desktop's variable edit drawer
   calls `.trim()` on an object and throws on submit. This turn adopts
   the new contract on the desktop side, Rust and TypeScript alike.
2. **Surface the controlled terminology in the variable table.** A new
   **Terminology** column sits next to **Label**. It lists the codelist
   references a variable declares (`variable_controlled.code`), and
   clicking one opens a drawer with the permissible values of that
   codelist, so a user can read the controlled terminology without
   leaving the SDTM domain page.

## Goals

- The desktop compiles and runs against a server built from
  `feat/sdtm-model_variable_control` — no `TypeError` on save, no
  plain-text 422 on write.
- `variable_controlled` round-trips through the desktop unchanged:
  `{ code: [...], format: [...] }` in, the same lists out.
- The Terminology column renders every codelist reference of a
  variable, and each one opens a drawer listing that codelist's code
  items.
- The drawer resolves a terminology code to a real codelist in the
  latest SDTM terminology version and shows its items read-only.

## Non-goals

- No server changes. Every endpoint the drawer needs already exists
  (`GET /api/terminology/versions`, `GET /api/terminology/code-lists`,
  `GET /api/terminology/code-items`).
- No change to the `format` list's *display*. The Terminology column
  shows `code` only; `format` stays an editable field in the drawer.
- No code-item editing from the variable table. The drawer is read-only
  by design (the user asked for the code-item table "without the edit
  buttons and SameCodeItemDialog").
- No new codelist lookup endpoint on the server.

## Anti-goals

- **No legacy string acceptance on the desktop.** The desktop's DTOs
  take the object shape only. A desktop client that still sends
  `"variableControlled": "C66731"` is a bug to fix, not a shape to
  keep alive with an untagged enum.
- **No silent coercion of a string into `{code: [string]}`.** The
  server refuses it, so the desktop refuses it too — the two halves
  break together, per the desktop guideline ("Any change to wire shape
  … is paired with a TS update in the same commit").
- **No fallback from the resolved codelist to a same-code search.** The
  drawer's job is "the items of *this* codelist". A second, different
  lookup (items whose own NCI code happens to equal the codelist's NCI
  code) would show an unrelated set.

---

## Constraints

- **The desktop hand-duplicates the server's wire DTOs.** It does not
  depend on `apis`; `src-tauri/src/http/domain_model/variable.rs` and
  `src/shared/api/types.ts` mirror the server 1:1. Both halves change
  in this turn.
- **`code_items` reference their codelist by `codelist_id`, not by the
  codelist's `code`.** In the workbook, an item row points at its
  parent codelist by the codelist's NCI code (column 1 of the sheet);
  the importer resolves that reference into a `codelist_id` when it
  writes the rows. In the database, a code item's own `code` is the
  item's NCI code (`C174106`), while the codelist's `code` (`C66741`)
  is the codelist's NCI code. **The query that returns "the items of
  codelist X" is `list_code_items?codelistId=X`**, not
  `by-version-and-code?code=X` — the latter matches items whose *own*
  code equals X, which for a codelist's own code is empty.
- **The terminology version list is ordered by `id ASC`.** "Latest
  SDTM terminology version" therefore means the highest `id` among the
  versions with `kind === "sdtm"` — the most recently imported one.
- **The codelist full-text index does not cover the `code` column.**
  `code_lists.tsv` is generated over `name`, `submission_value`,
  `synonym`, `definition`, `nci_preferred_term`. A terminology code
  that is an NCI c-code (`C66731`) therefore does not match on its own;
  a terminology code that is a codelist name (`VSTESTCD`) does.
- **i18n catalogs must keep the same key set.** `registry.test.ts`
  asserts `Object.keys(zhCN).sort()` equals `Object.keys(en).sort()`,
  and `TranslationKey = keyof typeof en`, so every new key goes into
  both catalogs.
- **Rows in the variable table are draggable.** A clickable element
  inside a draggable row must be a real interactive element, or
  `@dnd-kit/dom`'s `preventActivation` will not suppress drag
  activation.

---

## Approaches

### A. Which query lists the code items? *(decided)*

| Option | Query | Result | Verdict |
|---|---|---|---|
| **Items by codelist id** | `GET /api/terminology/code-items?codelistId=<id>&versionId=<v>` | The permissible values that belong to the codelist — the rows the workbook lists under it | **Chosen.** This is what "the related codelist items" means. |
| Items by version and code | `GET /api/terminology/code-items/by-version-and-code?versionId=<v>&code=<codelist.code>` | Items whose *own* NCI code equals the codelist's NCI code — empty for real CDISC data | Rejected: it answers a different question. |
| Both, codelist id first | as above, with a fallback | Union of two unrelated sets | Rejected: two request paths, ambiguous result. |

### B. Where does the drawer live? *(decided)*

| Option | Verdict |
|---|---|
| **Terminology feature** — `features/terminology/components/TerminologyCodeItemsDrawer.tsx`, next to the `CodeItemTable` it renders | **Chosen.** The drawer is a terminology view over terminology data; terminology keeps ownership of its components, and the drawer is reusable by any feature that shows a codelist's items. Terminology gains a `components/index.ts` barrel (it has none today). |
| Domain-model feature — `features/domain-model/components/…` | Rejected: it would import `CodeItemTable` across features, which the desktop guideline forbids except through a barrel — and that barrel has to exist either way. |

### C. How does the edit drawer edit `{code[], format[]}`? *(decided)*

| Option | Verdict |
|---|---|
| **Two comma-separated text fields** — one for codes, one for formats; `"A, B"` parses to `["A","B"]` | **Chosen.** Smallest change from today's single free-text field, both lists stay editable, one parse rule to test. |
| Chip editors (add/remove one value at a time) | Rejected for now: nicer to use, but more UI surface to translate and test, for a field that is edited rarely. |

### D. Everything else

- The **Terminology column** renders one small clickable chip per
  codelist reference; a variable with no references shows `—`.
- The **drawer** owns its own queries and rendering; the page owns only
  the open/close boolean and the selected code. This mirrors
  `SameCodeItemsDialog`, which "owns its own queries and navigation
  glue — the parent page only controls `open` / `code` / `onClose`".
- The **drawer's items are paginated with the same infinite-scroll
  sentinel** `CodeListDetailPage` uses, because a codelist can hold
  hundreds of permissible values.

---

## Design

### 1. The wire shape (Rust)

`src-tauri/src/http/domain_model/variable.rs` gains a mirror of the
server's `dto::SdtmVariableControlled`, byte-for-byte the same serde
attributes so the two halves agree on what is on the wire:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableControlled {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Vec<String>>,
}
```

`default` makes an absent key and an explicit `null` both read back as
`None`; `skip_serializing_if` keeps absent lists out of the JSON, so
the stored document is `{"code": ["VSTESTCD"]}` and never
`{"code": ["VSTESTCD"], "format": null}`.

Three fields change type:

| Struct | Field | New type |
|---|---|---|
| `SdtmVariableViewResponse` | `variable_controlled` | `Option<SdtmVariableControlled>` |
| `CreateSdtmVariableRequest` | `variable_controlled` | `Option<SdtmVariableControlled>` |
| `UpdateSdtmVariableRequest` | `variable_controlled` | `Option<Option<SdtmVariableControlled>>` |

The update field keeps the `Option<Option<T>>` type and gains the
server's `double_option` deserializer (see A1), so the three states
survive both hops — the Tauri payload from the webview and the HTTP
request to the server:

| Desktop sends | JSON | Server effect |
|---|---|---|
| outer `None` | key omitted | column unchanged |
| outer `Some(None)` | `"variableControlled": null` | column cleared |
| outer `Some(Some(v))` | `"variableControlled": {…}` | column replaced |

`skip_serializing_if = "Option::is_none"` stays on the outer option, so
"unchanged" is expressed by *absence*, not by `null`.

Nothing else in the crate moves: the command shims
(`src-tauri/src/commands/domain_model/variable.rs`) pass the DTOs
through untouched, and the `http` layer is transport-only.

### 2. The wire shape (TypeScript)

`src/shared/api/types.ts`:

```ts
export interface SdtmVariableControlled {
  code?: string[];
  format?: string[];
}
```

| Interface | Field | New type |
|---|---|---|
| `SdtmVariableView` | `variableControlled` | `SdtmVariableControlled` (optional) |
| `CreateSdtmVariableInput` | `variableControlled` | `SdtmVariableControlled` (optional) |
| `UpdateSdtmVariableInput` | `variableControlled` | `SdtmVariableControlled \| null` (optional) |

`null` on the update input means "clear", matching the Rust
three-state encoding above.

### 3. The variable edit drawer

`features/domain-model/components/VariableEditDrawer.tsx` replaces its
single "Controlled vocabulary (CCDD)" text field with two fields:

- **Controlled terminology (code list)** — the `code` list.
- **Controlled terminology (format)** — the `format` list.

Both are comma-separated free text. The parse rule is one function:
split on `,`, trim each part, drop empty parts.

- **Seeding (edit mode).** `row.variableControlled?.code ?? []` and
  `row.variableControlled?.format ?? []`, joined with `", "`.
- **Create.** `variableControlled` is omitted entirely when both lists
  are empty; otherwise it is
  `{ code?: string[], format?: string[] }` with empty lists left out
  (the server stores what is set and nothing else).
- **Update.** Three states, decided by comparing the parsed lists with
  the row's current lists:
  - unchanged → `variableControlled` is not sent at all;
  - both lists emptied while the row had content → `null` (clear);
  - otherwise → the new object (replace).

### 4. The Terminology column

`features/domain-model/components/VariableTable.tsx`:

- A new column between **Label** and **Role**, headed
  `domainModel.sdtm.detail.col.terminology` ("Terminology" / "术语").
- The cell renders `row.variableControlled?.code ?? []`. Each reference
  is a small clickable chip (`component="button"`, so `@dnd-kit/dom`'s
  `preventActivation` suppresses drag activation on it); a row with no
  references renders `—`.
- A new optional prop `onTerminologyClick?: (code: string) => void`
  fires on click. The table stays presentational — it does not fetch
  anything.

### 5. The drawer

`features/terminology/components/TerminologyCodeItemsDrawer.tsx`, new.

```ts
export interface TerminologyCodeItemsDrawerProps {
  open: boolean;
  code: string | null;
  onClose: () => void;
}
```

It owns its own queries, in this order:

1. **The SDTM terminology version to resolve against.**
   The project's configured release when a project is in scope, else the
   most recently imported SDTM release — see A6 for the full rule.
2. **Resolve the code to a codelist.** A new query hook
   `useFindCodeListBySubmissionValue(versionId, submissionValue)` in the
   terminology data layer: `GET /api/terminology/code-lists?versionId=<v>&fragment=<code>`
   (the code as the search fragment, as specified), then pick the row
   whose `submissionValue` equals the code **exactly**. The search is a
   full-text match over `name`, `submission_value`, `synonym`,
   `definition` and `nci_preferred_term`, so several rows can come
   back; only the exact submission-value match is the codelist we want.
   The request asks for the server's maximum page size so a common
   token does not push the exact match off the first page.
3. **List the items.** The existing `useListCodeItems(codelistId)`
   infinite query — the same hook `CodeListDetailPage` uses — pages
   the items 20 at a time.

The drawer renders:

- a title naming the terminology code, and the resolved codelist's
  name and NCI code when it was found, with an "EXT" chip when the
  codelist is extensible;
- a spinner while resolving;
- an `Alert` + **Retry** button when a query fails;
- "no code list found for {code}" when the resolution comes back
  empty;
- otherwise a `CodeItemTable` with `canMutate={false}` and no
  `onCodeClick` — so no create/edit/delete buttons and no
  `SameCodeItemsDialog` — plus the `InfiniteScrollSentinel` bottom
  slot for paging.

### 6. Wiring

`features/domain-model/pages/SdtmDomainDetail.tsx` owns the drawer
state, per the guideline that pages own the open/close boolean:

```tsx
const [terminologyCode, setTerminologyCode] = useState<string | null>(null);
…
<VariableTable … onTerminologyClick={(code) => setTerminologyCode(code)} />
<TerminologyCodeItemsDrawer
  open={terminologyCode !== null}
  code={terminologyCode}
  onClose={() => setTerminologyCode(null)}
/>
```

The drawer is imported through the terminology barrel, which gains
`export * from "./components";`.

### 7. i18n

New keys, in both `lib/packages/ui/src/i18n/locales/en.ts` and
`zhCN.ts`:

| Key | en | zh-CN |
|---|---|---|
| `domainModel.sdtm.detail.col.terminology` | `Terminology` | `术语` |
| `domainModel.sdtm.variable.field.variableControlledCode` | `Controlled terminology (code list)` | `受控术语（代码列表）` |
| `domainModel.sdtm.variable.field.variableControlledFormat` | `Controlled terminology (format)` | `受控术语（格式）` |
| `terminology.codeitem.drawer.title` | `Code items for {code}` | `{code} 的代码项` |
| `terminology.codeitem.drawer.notFound` | `No code list found for {code}` | `未找到 {code} 对应的代码列表` |
| `terminology.codeitem.drawer.openTooltip` | `View code items` | `查看代码项` |

The old `domainModel.sdtm.variable.field.variableControlled` key
("Controlled vocabulary (CCDD)") is replaced by the two new ones, since
one field became two.

---

## Tests

**Rust** (`src-tauri/src/http/domain_model/variable.rs`, `wiremock`):

- a view with `"variableControlled": {"code": ["VSTESTCD"], "format":
  ["Codelist"]}` decodes into both lists;
- a view with `"variableControlled": null` decodes to `None`;
- a view whose `variableControlled` omits `format` decodes `format` as
  `None` (the `default` attribute);
- an update request with `Some(Some(controlled))` serializes to
  `{"variableControlled":{"code":["VSTESTCD"]}}`;
- an update request with `Some(None)` serializes to
  `{"variableControlled":null}` — the "clear" state;
- an update request with the outer `None` omits the key.

**TypeScript** (`src/test/features/…`):

- `domain-model/variable-table.test.tsx` — the column indices shift by
  one (the Terminology column lands at `cells[4]`); the new tests
  assert the header, that a row's codes render as chips, that a row
  with no codes renders `—`, and that clicking a chip calls
  `onTerminologyClick` with that code.
- `domain-model/variable-edit-drawer.test.tsx` — the two comma-
  separated fields seed from the row, parse into arrays, send `null`
  when cleared, and send nothing when unchanged.
- `terminology/terminology-code-items-drawer.test.tsx` — with the
  `invoke` mock answering `list_terminology_versions`,
  `list_code_lists` and `list_code_items`, the drawer resolves the
  latest SDTM version, picks the codelist whose name matches exactly,
  and renders its items read-only (no edit buttons).
- `domain-model/sdtm-domain-detail.test.tsx` — the page mounts the
  drawer and opens it when a code is clicked.

---

## Amendments (2026-10-30, after builder review)

Five points the review raised, resolved as follows.

**A1. The desktop needs the `double_option` deserializer, not the plain
derive.** The desktop's `UpdateSdtmVariableRequest` is deserialized
from the *Tauri payload* (webview → Rust) before it is serialized to
the server. A plain `Option<Option<T>>` derive maps the payload's
`null` to the outer `None`, which then trips
`skip_serializing_if = "Option::is_none"` — the key is dropped from
the HTTP request and the server reads "unchanged". The desktop crate
has no `double_option` helper today (grep over `src-tauri/src` finds
no `deserialize_with` at all), so "clear" would be a silent no-op —
the same gap the 2026-10-29 spec flagged on the server side.

The desktop therefore mirrors the server's helper: a private
`double_option` in `http/domain_model/variable.rs` and
`#[serde(default, deserialize_with = "double_option")]` on
`UpdateSdtmVariableRequest.variable_controlled`. The three states then
survive both hops.

**A2. The match is on the code list's *submission value*, exact first
and case-insensitive second.** The SDTMIG writes a codelist reference
as "Test Code [VSTESTCD]" — the bracketed part is the codelist's
submission value, and that is what a variable's controlled terminology
holds. The codelist's `name` is its human-readable NCI name ("Test
Code"), so matching on it would pick the wrong row whenever a name
happens to equal the code. The full-text search is already
case-insensitive (Postgres lowercases the `tsvector`), so a stored
value of `vstestcd` finds the codelist in the search result but fails a
raw string comparison; the resolution takes the first row whose
`submissionValue` equals the code exactly, then falls back to the first
row whose `submissionValue` equals it case-insensitively.

The drawer also shows the resolved code list's extensibility, with the
same "EXT" chip the code-list table uses.

**A3. Known limitation: a code stored as an NCI c-code cannot be
resolved.** `code_lists.tsv` is generated over the five text columns
and *not* over `code`, and no endpoint looks a codelist up by its
`code`, so a terminology code of `C66731` matches nothing. The column
is meant to hold codelist *names* (`VSTESTCD`), which is what the
SDTMIG notation means and what the example in the request uses. A code
that resolves to nothing shows the "no code list found" state — it
never shows an empty table as if the codelist were empty.

**A4. A code containing a character Postgres reserves is an error, not
a miss.** The server rejects a fragment containing `&`, `|`, `!`, `(`,
`)` or `:` with a validation error. The drawer shows the error state
(with **Retry**) in that case, not "not found".

**A5. "No terminology imported" and "no such code list" are different
states.** The drawer distinguishes them: no SDTM terminology version
at all is its own message, and so is a version with no matching code
list.

**A6. The drawer resolves against the *project's configured* SDTM
terminology release.** A project pins the SDTM controlled-terminology
release it works against (`project.configurations.sdtmTerminology`),
and that is the release whose codelists its variables' controlled
terminology refers to — not merely the newest one on the server. The
SDTM domain model page is global, so the project comes from the URL:
`?projectCode=…` on both the domain list and the domain detail route,
carried across the list → detail navigation.

Resolution:

1. A project code in the URL → the project's configured
   `sdtmTerminology.versionId`. A stale id whose `versionName` still
   matches a known release resolves by name; the drawer never mutates
   the project, so the stored pointer is not healed.
2. No project, no configured release, or one that matches nothing →
   the most recently imported SDTM release (highest id among
   `kind === "sdtm"`).

While the project is still loading the version is unresolved, so the
code-list lookup waits rather than firing against the fallback version
first.

---

## What's left for the builder

- The exact wording of the two new field labels in the edit drawer.
- Whether the drawer's item list needs a search box. **Call:** no — the
  codelist's items are the whole point, and the table is read-only.
- The exact wording of the drawer's "not found" state. **Call:** as
  tabled above.
