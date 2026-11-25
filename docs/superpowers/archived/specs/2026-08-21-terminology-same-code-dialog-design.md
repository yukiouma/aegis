# Terminology — "Same code across codelists" dialog — Design

Date: 2026-08-21
Owner: terminology feature
Branch: `feat/desktop_termimology-same-code-query`

## Goal

In `aegis-desktop` terminology's `CodeListDetailPage`, let the user click any
row's **code cell** in `CodeItemTable` to pop a dialog listing every code item
that shares that exact `code` value across the same terminology version —
regardless of which codelist it lives in. Each row in the dialog carries the
owning codelist's `code` and `submissionValue` so the user can identify the
duplicate at a glance, and is a link to that codelist's detail page.

## Non-goals

- No new server endpoint. We reuse `GET
  /api/terminology/code-items/by-version-and-code` (already exists in
  `apps/server/aegis-server/src/transport/http/terminology/handlers.rs`).
- No server-side DTO changes.
- No TS DTO changes (`CodeItemListResponse` already exported by
  `shared/api/types.ts`).
- No reuse of this dialog from `TerminologyListPage` in this change.
- No inline editing of codelists from the dialog.

## UX

### Code cell on the detail table

The code value in each row of `CodeItemTable` becomes an inline, visually
affordant click target:
- `cursor: pointer`
- On hover: `text-decoration: underline`
- A `Tooltip` reading "Show this code in other codelists"

The cell still looks like a normal table cell at rest. Row-level hover
behaviour is preserved. Clicking the code does not trigger any other row
handler.

### Dialog

| Concern | Behaviour |
|---|---|
| Title | `"Code items with code \"<code>\""` (i18n key with placeholder) |
| Columns | Code / Submission value / Code List Code / Code List Submission value |
| Rows | Hoverable. The row whose `codelistId` equals the page's current `codelistId` is tinted (`bgcolor: 'action.hover'`) so the user can locate themselves. |
| Row click | Navigate to `/terminology/<kind>/codelists/<codelistId>?versionId=<versionId>` and close the dialog. |
| Close | ESC, backdrop click, and a close `IconButton` in the title row all call `onClose`. Cache persists; reopening reuses it. |
| Loading (items) | Centered `CircularProgress`. |
| Loading (per-codelist info) | Rows render with `—` for the missing codelist fields, no blocking overlay. |
| Error (items) | Inline `Alert` + Retry button (`refetch()`). |
| Empty | Defensive "No items share this code." — in practice the endpoint always returns at least the current row. |
| Scroll | `TableContainer` constrains height inside `DialogContent` so very long lists scroll without growing the dialog. |

## Architecture

```
CodeItemTable row → onCodeClick(row.code)
   → CodeListDetailPage sets sameCodeDialog = { code }
   → <SameCodeItemsDialog open code versionId currentCodelistId kind onClose />
        ├─ useListCodeItemsByVersionAndCode(versionId, code)
        │     → api.listCodeItemsByVersionAndCode(vid, code)
        │           → invoke 'list_code_items_by_version_and_code'
        │                 → HTTP GET /api/terminology/code-items/by-version-and-code
        ├─ useGetCodeListsByIds(unique(items.codelistId))
        │     → useQueries over api.getCodeListById (cached by id)
        └─ row click → useNavigate(...) + onClose()
```

The split mirrors the existing `CodeItemDrawer` / `CodeListDrawer` pattern:
the page owns dialog state; the dialog owns its own query hooks and
navigation glue.

## File plan

| File | Change |
|---|---|
| `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs` | Add `CodeItemListResponse`, `list_by_version_and_code`, wiremock tests. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs` | **Modify.** Append a new Tauri command shim `list_code_items_by_version_and_code`. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology.rs` | No change. The `code_item` submodule is already declared. |
| `apps/desktop/aegis-desktop/src/shared/api/index.ts` | Add `api.listCodeItemsByVersionAndCode(versionId, code)` (positional args, matches the existing `getCodeListById(id)` style). No new type in `types.ts` is required. |
| `apps/desktop/aegis-desktop/src/shared/query/keys.ts` | Add `queryKeys.terminology.codeItemsByCode(versionId, code)`. |
| `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts` | Add `useListCodeItemsByVersionAndCode`, `useGetCodeListsByIds`. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/CodeItemTable.tsx` | Add optional `onCodeClick` prop; restyle code cell with hover affordance + Tooltip. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/SameCodeItemsDialog.tsx` | New file. |
| `apps/desktop/aegis-desktop/src/features/terminology/pages/CodeListDetailPage.tsx` | Add `sameCodeDialog` state; wire `onCodeClick`; render the dialog. |
| `lib/packages/ui/src/i18n/locales/en.ts` and `.../zhCN.ts` | 3 new keys under `terminology.codeitem.*`. |

## Public contracts

### Rust — `http/terminology/code_item.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemListResponse {
    pub items: Vec<CodeItemViewResponse>,
}

pub async fn list_by_version_and_code(
    c: &HttpClient,
    version_id: i64,
    code: &str,
) -> Result<CodeItemListResponse, ApiError> {
    let path = format!(
        "/api/terminology/code-items/by-version-and-code?versionId={}&code={}",
        version_id,
        percent_encode_fragment(code),
    );
    c.request(reqwest::Method::GET, &path, None::<&()>).await
}
```

`CodeItemViewResponse` already exists in this file.

### Rust — Tauri command

```rust
#[tauri::command]
pub async fn list_code_items_by_version_and_code(
    client: State<'_, HttpClient>,
    version_id: i64,
    code: String,
) -> Result<CodeItemListResponse, ApiError> {
    code_item::list_by_version_and_code(&client, version_id, &code).await
}
```

### Shared TS API

```ts
listCodeItemsByVersionAndCode: (
  versionId: number,
  code: string,
): Promise<CodeItemListResponse> =>
  call<CodeItemListResponse>("list_code_items_by_version_and_code", { versionId, code }),
```

### Query key

```ts
codeItemsByCode: (versionId: number, code: string) =>
  ["terminology", "codeItemsByCode", versionId, code] as const,
```

### Hooks

```ts
export function useListCodeItemsByVersionAndCode(
  versionId: number | null,
  code: string | null,
): UseQueryResult<CodeItemListResponse, ApiError>;

export function useGetCodeListsByIds(
  ids: number[],
): UseQueryResult<CodeListView, ApiError>[];
```

`useGetCodeListsByIds` uses `useQueries` with `staleTime: 30s` and the
existing `queryKeys.terminology.codeList(id)` key, so repeated visits reuse
the cache and the existing `useGetCodeList` also benefits.

### `CodeItemTable` prop

```ts
onCodeClick?: (row: CodeItemView) => void;
```

If omitted, the cell renders identically to today's — no cursor / underline.

### `SameCodeItemsDialog` props

```ts
interface SameCodeItemsDialogProps {
  open: boolean;
  code: string | null;
  versionId: number;
  currentCodelistId: number;
  kind: TerminologyKind;
  onClose: () => void;
}
```

## i18n keys (additions)

| Key | English |
|---|---|
| `terminology.codeitem.codeClick.tooltip` | `Show this code in other codelists` |
| `terminology.codeitem.sameCode.dialogTitle` | `Code items with code "{code}"` |
| `terminology.codeitem.sameCode.empty` | `No items share this code.` |

Existing keys reused for column headers:

- `terminology.codeitem.field.code`
- `terminology.codeitem.field.submissionValue`
- `terminology.codelist.field.code`
- `terminology.codelist.field.submissionValue`

## Error handling

- Endpoint failure → Alert with retry button.
- Per-codelist fetch failure → row still renders; the two codelist columns
  show `—` until the request lands or errors out. We do not block the whole
  dialog on a single missing codelist.
- Route navigation failure is the framework's concern; no extra handling.

## Testing

### Rust (unit, wiremock)

Add to `http/terminology/code_item.rs::tests`:

- `list_by_version_and_code_returns_items` — mocks 200 with two items,
  asserts decoded list shape.
- `list_by_version_and_code_encodes_special_chars` — sends a `code` with a
  space and a `/`, asserts the path contains the percent-encoded form and
  the server-expected code is decoded back.

### TS / UI

No new automated tests. UI verification is manual: load a version with at
least one code shared between ≥2 codelists, click, confirm the dialog
opens, the columns are correct, navigation works, and the dialog closes.

## Out of scope

- Extending the dialog for description columns or descriptions tooltips.
- Sorting / filtering inside the dialog.
- Highlighting "this is the current codelist" beyond a subtle row tint.
- Linking from `TerminologyListPage`'s `CodeListTable` (same codelist *code*
  across versions is a different feature).
- Reusing the dialog from the `CodeListTable` "open" affordance.

## Rollback

All changes are additive. Reverting the merge commit removes:
- The new Tauri command shim and its registration.
- The new TS hooks, query keys, and api wrapper.
- The new component, prop, page state, and i18n keys.

No migrations, no schema changes, no server changes — rollback is purely a
frontend revert.
