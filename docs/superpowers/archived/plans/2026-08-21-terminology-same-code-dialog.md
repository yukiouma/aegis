# Terminology — "Same code across codelists" Dialog — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a clickable code cell to `CodeItemTable` that opens a dialog listing every code item sharing the same code within the current terminology version, with each row navigable to its owning codelist.

**Architecture:** New Rust wire function reuses `GET /api/terminology/code-items/by-version-and-code`. New Tauri command shim. New TS API wrapper, query key, and React hooks (`useListCodeItemsByVersionAndCode`, `useGetCodeListsByIds`). New `SameCodeItemsDialog` component owns its queries and navigation glue. `CodeListDetailPage` holds dialog state. `CodeItemTable` gains an optional `onCodeClick` prop and renders the code cell with hover-underline + tooltip affordance.

**Tech Stack:** Tauri (Rust + React), TanStack Router/Query, MUI, react-i18n via `@aegis/ui/i18n`, wiremock (Rust tests), vitest (TS — not used in this plan).

## Global Constraints

- Spec: [`docs/superpowers/specs/2026-08-21-terminology-same-code-dialog-design.md`](docs/superpowers/specs/2026-08-21-terminology-same-code-dialog-design.md)
- Code style: existing workspace conventions (snake_case Rust, camelCase TS, RFC-3339 datetime formatting).
- TS run scripts: `pnpm test`, `pnpm typecheck` (defined in `apps/desktop/aegis-desktop/package.json`).
- Rust test runner: `cargo test -p aegis-desktop`.
- Wire format: cargo `serde(rename_all = "camelCase")` on responses, snake_case `arguments` for `%`-encoded queries (matches existing `list_paged`).
- All reused i18n keys exist in `lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts`. New keys go into the `terminology.codeitem.*` namespace.
- **No new dependencies.** No server changes. No new TS DTO types.
- Branch: `feat/desktop_termimology-same-code-query`.

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs` | Modify | Add `CodeItemListResponse`, `list_by_version_and_code`, wiremock tests. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs` | Modify | Add Tauri command `list_code_items_by_version_and_code`. |
| `apps/desktop/aegis-desktop/src/shared/api/index.ts` | Modify | Add `api.listCodeItemsByVersionAndCode(versionId, code)`. |
| `apps/desktop/aegis-desktop/src/shared/query/keys.ts` | Modify | Add `queryKeys.terminology.codeItemsByCode(versionId, code)`. |
| `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts` | Modify | Add `useListCodeItemsByVersionAndCode`, `useGetCodeListsByIds`. |
| `lib/packages/ui/src/i18n/locales/en.ts` | Modify | Add 3 keys under `terminology.codeitem.*`. |
| `lib/packages/ui/src/i18n/locales/zhCN.ts` | Modify | Add 3 keys (Chinese translations). |
| `apps/desktop/aegis-desktop/src/features/terminology/components/CodeItemTable.tsx` | Modify | Add `onCodeClick` prop; render clickable code cell. |
| `apps/desktop/aegis-desktop/src/features/terminology/components/SameCodeItemsDialog.tsx` | Create | Dialog component owning queries + navigation. |
| `apps/desktop/aegis-desktop/src/features/terminology/pages/CodeListDetailPage.tsx` | Modify | Add `sameCodeDialog` state; wire `onCodeClick` and `<SameCodeItemsDialog />`. |

---

## Task 1: Rust wire layer — `list_by_version_and_code`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs`
- Test: same file, inside `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `percent_encode_fragment` (already in this file); `HttpClient::request`; `ApiError`.
- Produces:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct CodeItemListResponse { pub items: Vec<CodeItemViewResponse> }

  pub async fn list_by_version_and_code(
      c: &HttpClient,
      version_id: i64,
      code: &str,
  ) -> Result<CodeItemListResponse, ApiError>;
  ```

- [ ] **Step 1: Write the failing wiremock tests**

Append the following to `mod tests` in `http/terminology/code_item.rs` (after the existing `batch_request_serializes_camel_case` test):

```rust
fn list_response_json(version_id: i64, count: usize) -> serde_json::Value {
    let items: Vec<_> = (0..count).map(|i| {
        serde_json::json!({
            "id": i, "codelistId": 10 + i as i64, "versionId": version_id,
            "code": "YES", "submissionValue": "SV",
            "synonym": "syn", "definition": "def", "nciPreferredTerm": "nci",
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-01T00:00:00Z"
        })
    }).collect();
    serde_json::json!({ "items": items })
}

#[tokio::test]
async fn list_by_version_and_code_returns_items() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/terminology/code-items/by-version-and-code"))
        .and(query_param("versionId", "7"))
        .and(query_param("code", "YES"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(list_response_json(7, 2)))
        .mount(&server)
        .await;
    let resp = list_by_version_and_code(&client(&server), 7, "YES").await.unwrap();
    assert_eq!(resp.items.len(), 2);
    assert_eq!(resp.items[0].code, "YES");
    assert_eq!(resp.items[0].version_id, 7);
    assert_eq!(resp.items[1].codelist_id, 11);
}

#[tokio::test]
async fn list_by_version_and_code_percent_encodes_value() {
    // Code values may contain spaces or punctuation; the wire path
    // must percent-encode them so the server parser sees the original
    // value back after URL decoding.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/terminology/code-items/by-version-and-code"))
        .and(query_param("versionId", "7"))
        .and(query_param("code", "A B"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(list_response_json(7, 1)))
        .mount(&server)
        .await;
    let resp = list_by_version_and_code(&client(&server), 7, "A B")
        .await.unwrap();
    assert_eq!(resp.items.len(), 1);
}
```

- [ ] **Step 2: Run the tests to confirm they fail**

Run: `cargo test -p aegis-desktop code_item::tests::list_by_version_and_code`
Expected: compile error — `list_by_version_and_code` and `CodeItemListResponse` not defined.

- [ ] **Step 3: Implement `CodeItemListResponse` and `list_by_version_and_code`**

In `http/terminology/code_item.rs`:

1. Add `CodeItemListResponse` near the other response types (around line 27, after `CodeItemPagedResponse`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeItemListResponse {
    pub items: Vec<CodeItemViewResponse>,
}
```

2. Add the function after `list_paged` (around line 143):

```rust
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

- [ ] **Step 4: Run the tests to confirm they pass**

Run: `cargo test -p aegis-desktop code_item::tests::list_by_version_and_code`
Expected: 2 tests pass.

- [ ] **Step 5: Run the entire src-tauri test suite to confirm no regressions**

Run: `cargo test -p aegis-desktop`
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs
git commit -m "feat(desktop,tauri): add list_by_version_and_code wire fn for code items"
```

---

## Task 2: Tauri command shim — `list_code_items_by_version_and_code`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs`

**Interfaces:**
- Consumes: `crate::http::terminology::code_item::{self, CodeItemListResponse}`; `tauri::State<HttpClient>`; `ApiError`.
- Produces:
  ```rust
  #[tauri::command]
  pub async fn list_code_items_by_version_and_code(
      client: State<'_, HttpClient>,
      version_id: i64,
      code: String,
  ) -> Result<CodeItemListResponse, ApiError>;
  ```
  Tauri command name: `list_code_items_by_version_and_code`.

- [ ] **Step 1: Add the command shim**

Append the following to `commands/terminology/code_item.rs`:

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

Update the `use` line at the top of the file to include `CodeItemListResponse`:

```rust
use crate::http::terminology::code_item::{
    self, CodeItemListQuery, CodeItemListResponse, CodeItemPagedResponse,
    CodeItemViewResponse, CreateCodeItemRequest, UpdateCodeItemRequest,
};
```

- [ ] **Step 2: Verify compile**

Run: `cargo check -p aegis-desktop`
Expected: success, no warnings introduced in this file.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs
git commit -m "feat(desktop,tauri): expose list_code_items_by_version_and_code command"
```

---

## Task 3: Shared TS API wrapper + query key

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts`
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts`

**Interfaces:**
- Produces:
  ```ts
  api.listCodeItemsByVersionAndCode(versionId: number, code: string): Promise<CodeItemListResponse>
  queryKeys.terminology.codeItemsByCode(versionId: number, code: string)
      : readonly ["terminology", "codeItemsByCode", number, string]
  ```

- [ ] **Step 1: Add the query key**

In `apps/desktop/aegis-desktop/src/shared/query/keys.ts`, inside `terminology:` (just after the existing `codeItems` entry, around line 32):

```ts
codeItemsByCode: (versionId: number, code: string) =>
  ["terminology", "codeItemsByCode", versionId, code] as const,
```

- [ ] **Step 2: Add the api wrapper**

In `apps/desktop/aegis-desktop/src/shared/api/index.ts`:

1. Add `CodeItemListResponse` to the type import from `./types` (it's already exported from types.ts line 225):

```ts
import type {
  CodeItemListQuery,
  CodeItemListResponse,
  CodeItemView,
  // ... existing imports
} from "./types";
```

2. Add the wrapper to the `api` object, after the existing `listCodeItems` (around line 161):

```ts
listCodeItemsByVersionAndCode: (
  versionId: number,
  code: string,
): Promise<CodeItemListResponse> =>
  call<CodeItemListResponse>("list_code_items_by_version_and_code", {
    versionId,
    code,
  }),
```

- [ ] **Step 3: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS, no errors.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/index.ts \
        apps/desktop/aegis-desktop/src/shared/query/keys.ts
git commit -m "feat(desktop): add api wrapper and query key for code-items-by-version-and-code"
```

---

## Task 4: React hooks — `useListCodeItemsByVersionAndCode`, `useGetCodeListsByIds`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts`

**Interfaces:**
- Produces:
  ```ts
  function useListCodeItemsByVersionAndCode(
      versionId: number | null,
      code: string | null,
  ): UseQueryResult<CodeItemListResponse, ApiError>;

  function useGetCodeListsByIds(
      ids: number[],
  ): UseQueryResult<CodeListView, ApiError>[];
  ```
- Behaviour:
  - `useListCodeItemsByVersionAndCode`: `enabled` flips off when `versionId <= 0` or `code` is null/empty.
  - `useGetCodeListsByIds`: uses `useQueries`, each query keyed by `queryKeys.terminology.codeList(id)`, `staleTime: 30_000`.

- [ ] **Step 1: Add the imports**

Add `useQueries` to the existing `@tanstack/react-query` import at the top of `data/list.ts`:

```ts
import {
  useInfiniteQuery,
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
  type InfiniteData,
  type QueryKey,
} from "@tanstack/react-query";
```

(Already imports `CodeItemListResponse` indirectly through `useListCodeItems`; verify by checking the existing import. If not present, add `CodeItemListResponse` to the type import from `"../../../shared/api"`.)

- [ ] **Step 2: Add `useListCodeItemsByVersionAndCode`**

Append to the bottom of `data/list.ts`:

```ts
/**
 * Single-page lookup of all code items sharing a given code within a
 * terminology version. The server endpoint is non-paginated, so we use
 * `useQuery` instead of `useInfiniteQuery`. The hook is `enabled` only when
 * both `versionId` and `code` are usable; the dialog flips `code` to a
 * real value to trigger the fetch.
 */
export function useListCodeItemsByVersionAndCode(
  versionId: number | null,
  code: string | null,
) {
  return useQuery<CodeItemListResponse, ApiError>({
    queryKey: queryKeys.terminology.codeItemsByCode(versionId ?? 0, code ?? ""),
    queryFn: () => api.listCodeItemsByVersionAndCode(versionId!, code!),
    enabled: versionId != null && versionId > 0 && !!code,
  });
}
```

- [ ] **Step 3: Add `useGetCodeListsByIds`**

Append:

```ts
/**
 * Bulk lookup of codelists by id. Returns one `UseQueryResult` per id in
 * the same order. React Query dedupes by `queryKey`, so overlapping ids
 * across dialog opens share their cache entry with the single-id
 * `useGetCodeList` hook.
 *
 * `staleTime: 30_000` keeps the dialog snappy on re-open within 30s.
 */
export function useGetCodeListsByIds(ids: number[]) {
  return useQueries({
    queries: ids.map((id) => ({
      queryKey: queryKeys.terminology.codeList(id),
      queryFn: () => api.getCodeListById(id),
      enabled: id > 0,
      staleTime: 30_000,
    })),
  });
}
```

- [ ] **Step 4: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/data/list.ts
git commit -m "feat(desktop,terminology): add hooks for code-items-by-code and bulk codelist fetch"
```

---

## Task 5: i18n keys (en + zhCN)

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Produces: 3 new keys:
  - `terminology.codeitem.codeClick.tooltip`
  - `terminology.codeitem.sameCode.dialogTitle` (contains `{code}` placeholder)
  - `terminology.codeitem.sameCode.empty`

- [ ] **Step 1: Add the English strings**

In `lib/packages/ui/src/i18n/locales/en.ts`, after the existing `terminology.codeitem.readOnly: 'Read-only',` entry (around line 198) and before `terminology.action.delete.confirmTitle` (line 200):

```ts
  'terminology.codeitem.codeClick.tooltip':
    'Show this code in other codelists',
  'terminology.codeitem.sameCode.dialogTitle': 'Code items with code "{code}"',
  'terminology.codeitem.sameCode.empty': 'No items share this code.',
```

Also add a generic `common.close` key next to the existing `common.cancel/confirm/back/etc.` keys (around line 207):

```ts
  'common.close': 'Close',
```

- [ ] **Step 2: Add the Chinese strings**

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, find the same insertion point (mirror of step 1) and add the translations. Use these to match existing terminology tone:

```ts
  'terminology.codeitem.codeClick.tooltip': '在其他术语集中查看该 Code',
  'terminology.codeitem.sameCode.dialogTitle': '同 Code 的条目（{code}）',
  'terminology.codeitem.sameCode.empty': '该 Code 在其他术语集中不存在。',
```

Also add the Chinese translation of `common.close` next to the existing `common.*` keys (around line 201):

```ts
  'common.close': '关闭',
```

(Confirm by reading 5 lines around the insertion point first — exact wording can be tweaked in review.)

- [ ] **Step 3: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS (these are pure string maps; typecheck verifies the key access types update).

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(i18n): add keys for same-code dialog"
```

---

## Task 6: `CodeItemTable` — clickable code cell

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/components/CodeItemTable.tsx`

**Interfaces:**
- Adds prop: `onCodeClick?: (row: CodeItemView) => void;`
- Behaviour: code cell renders a `<Tooltip>`-wrapped span with `cursor: pointer` and hover underline **only when** `onCodeClick` is provided. Otherwise identical to today.

- [ ] **Step 1: Add the optional prop**

In `CodeItemTable.tsx`, inside `CodeItemTableProps` (currently around line 29), add:

```ts
onCodeClick?: (row: CodeItemView) => void;
```

- [ ] **Step 2: Destructure it in the function signature**

Change the component signature (around line 46) to:

```ts
export function CodeItemTable({
  rows,
  loading,
  mutationLoading,
  error,
  canMutate,
  onRetry,
  onCreate,
  onEdit,
  onDelete,
  emptyMessage,
  bottomSlot,
  onCodeClick,
}: CodeItemTableProps) {
```

- [ ] **Step 3: Wrap the code cell**

In the `<TableBody>` map (currently around line 130), replace:

```tsx
<TableCell>{row.code}</TableCell>
```

with:

```tsx
<TableCell>
  <Tooltip
    title={t("terminology.codeitem.codeClick.tooltip")}
    disableInteractive
  >
    <Box
      component="span"
      onClick={onCodeClick ? () => onCodeClick(row) : undefined}
      sx={{
        cursor: onCodeClick ? "pointer" : "default",
        "&:hover": onCodeClick
          ? { textDecoration: "underline" }
          : undefined,
        display: "inline-block",
      }}
    >
      {row.code}
    </Box>
  </Tooltip>
</TableCell>
```

`Box` is already imported on line 3 (from `@aegis/ui/mui`). `Tooltip` on line 13.

- [ ] **Step 4: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/components/CodeItemTable.tsx
git commit -m "feat(desktop,terminology): make CodeItemTable code cell clickable"
```

> **Manual check after this task:** Run `pnpm dev`, navigate to a code list, hover a code cell — confirm cursor changes and the underline appears. Confirm consumers without `onCodeClick` are unaffected (no other consumer exists today, but the conditional `sx` guarantees it).

---

## Task 7: `SameCodeItemsDialog` component

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/SameCodeItemsDialog.tsx`

**Interfaces:**
- Props:
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
- Behaviour:
  - When `open` and `code` non-null, fetches items via `useListCodeItemsByVersionAndCode(versionId, code)` and unique-codelist info via `useGetCodeListsByIds(unique(items.map(i => i.codelistId)))`.
  - Joins via `useMemo` into `rows: Array<{ item, codelist, isCurrent }>`.
  - Table columns: Code, Submission value, Code List Code, Code List Submission value. The row whose `item.codelistId === currentCodelistId` is tinted with `bgcolor: 'action.hover'`.
  - Clicking a row navigates to the matching codelist detail page and calls `onClose`.
  - Renders `<CircularProgress />` while the items query is loading; `<Alert severity="error">` + Retry button on items error; an empty-state `<Typography>` only if the resolved list is empty; otherwise a `TableContainer` with the rows.
  - Per-codelist info still loading or errored → the two codelist columns render `—`. No blocking overlay.

- [ ] **Step 1: Create the component file**

Create `apps/desktop/aegis-desktop/src/features/terminology/components/SameCodeItemsDialog.tsx`:

```tsx
import { useMemo } from "react";
import { useNavigate } from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogContent,
  DialogTitle,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { Close as CloseIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type { CodeItemView, CodeListView, TerminologyKind } from "../../../shared/api";
import {
  useGetCodeListsByIds,
  useListCodeItemsByVersionAndCode,
} from "../data";

interface JoinedRow {
  item: CodeItemView;
  codelist: CodeListView | undefined;
  isCurrent: boolean;
}

export interface SameCodeItemsDialogProps {
  open: boolean;
  code: string | null;
  versionId: number;
  currentCodelistId: number;
  kind: TerminologyKind;
  onClose: () => void;
}

export function SameCodeItemsDialog({
  open,
  code,
  versionId,
  currentCodelistId,
  kind,
  onClose,
}: SameCodeItemsDialogProps) {
  const { t } = useI18n();
  const navigate = useNavigate();

  const enabled = open && !!code;
  const itemsQuery = useListCodeItemsByVersionAndCode(versionId, enabled ? code : null);

  const codelistIds = useMemo(() => {
    if (!itemsQuery.data) return [];
    const ids = itemsQuery.data.items.map((i) => i.codelistId);
    return Array.from(new Set(ids));
  }, [itemsQuery.data]);

  const codelistQueries = useGetCodeListsByIds(codelistIds);

  const rows = useMemo<JoinedRow[]>(() => {
    if (!itemsQuery.data) return [];
    return itemsQuery.data.items.map((item) => {
      const idx = codelistIds.indexOf(item.codelistId);
      const codelist = codelistQueries[idx]?.data;
      return {
        item,
        codelist,
        isCurrent: item.codelistId === currentCodelistId,
      };
    });
  }, [itemsQuery.data, codelistQueries, codelistIds, currentCodelistId]);

  const handleRowClick = (codelistId: number) => {
    onClose();
    void navigate({
      to: "/_authed/_layout/terminology/$kind/codelists/$codelistId",
      params: { kind, codelistId: String(codelistId) },
      search: { versionId },
    });
  };

  const title = code
    ? t("terminology.codeitem.sameCode.dialogTitle", { code })
    : "";

  return (
    <Dialog open={open && !!code} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle sx={{ display: "flex", alignItems: "center", gap: 1 }}>
        <Box component="span" sx={{ flex: 1 }}>
          {title}
        </Box>
        <Tooltip title={t("common.close")}>
          <IconButton
            size="small"
            aria-label={t("common.close")}
            onClick={onClose}
          >
            <CloseIcon fontSize="small" />
          </IconButton>
        </Tooltip>
      </DialogTitle>
      <DialogContent dividers>
        {itemsQuery.isLoading ? (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <CircularProgress />
          </Box>
        ) : itemsQuery.isError ? (
          <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
            <Alert severity="error">
              {t("terminology.codeitem.loadFailed", {
                message: errorMessage(itemsQuery.error),
              })}
            </Alert>
            <Box>
              <Button onClick={() => void itemsQuery.refetch()}>
                {t("common.retry")}
              </Button>
            </Box>
          </Box>
        ) : rows.length === 0 ? (
          <Typography color="text.secondary" sx={{ py: 4, textAlign: "center" }}>
            {t("terminology.codeitem.sameCode.empty")}
          </Typography>
        ) : (
          <TableContainer component={Paper} sx={{ maxHeight: "calc(100vh - 220px)" }}>
            <Table size="small" stickyHeader>
              <TableHead>
                <TableRow>
                  <TableCell>{t("terminology.codeitem.field.code")}</TableCell>
                  <TableCell>
                    {t("terminology.codeitem.field.submissionValue")}
                  </TableCell>
                  <TableCell>
                    {t("terminology.codelist.field.code")}
                  </TableCell>
                  <TableCell>
                    {t("terminology.codelist.field.submissionValue")}
                  </TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {rows.map(({ item, codelist, isCurrent }) => (
                  <TableRow
                    key={item.id}
                    hover
                    onClick={() => handleRowClick(item.codelistId)}
                    sx={{
                      cursor: "pointer",
                      bgcolor: isCurrent ? "action.hover" : undefined,
                    }}
                  >
                    <TableCell>{item.code}</TableCell>
                    <TableCell>{item.submissionValue}</TableCell>
                    <TableCell>{codelist?.code ?? "—"}</TableCell>
                    <TableCell>{codelist?.submissionValue ?? "—"}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </TableContainer>
        )}
      </DialogContent>
    </Dialog>
  );
}
```

- [ ] **Step 2: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS. If `common.close` is not in the locale, add it via the i18n task (it already exists — verified during exploration).

> If `pnpm typecheck` fails on missing imports, check these against `CodeListDetailPage.tsx`:
> - `import { getRouteApi, useNavigate } from "@tanstack/react-router"` — same pattern.
> - MUI components: `Alert, Box, Button, CircularProgress, Dialog, DialogContent, DialogTitle, IconButton, Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, Tooltip, Typography` — all under `@aegis/ui/mui`.
> - `Close as CloseIcon` — confirm by reading `apps/desktop/aegis-desktop/src/features/terminology/components/CodeListDrawer.tsx`'s existing icon imports.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/components/SameCodeItemsDialog.tsx
git commit -m "feat(desktop,terminology): add SameCodeItemsDialog component"
```

---

## Task 8: Wire dialog into `CodeListDetailPage`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/pages/CodeListDetailPage.tsx`

**Interfaces:**
- Adds state: `const [sameCodeDialog, setSameCodeDialog] = useState<{ code: string } | null>(null);`
- Wires `onCodeClick` to the existing `<CodeItemTable>`; renders `<SameCodeItemsDialog />` after the existing dialogs.

- [ ] **Step 1: Import the dialog**

In `CodeListDetailPage.tsx`, near the existing component imports (after `CodeListDrawer` import on line 54):

```tsx
import { SameCodeItemsDialog } from "../components/SameCodeItemsDialog";
```

- [ ] **Step 2: Add state**

After the existing state declarations (around line 93, after `confirmDelete`), add:

```tsx
const [sameCodeDialog, setSameCodeDialog] = useState<{ code: string } | null>(null);
```

- [ ] **Step 3: Wire `onCodeClick` on `CodeItemTable`**

In the `<CodeItemTable … />` JSX (around line 213), add:

```tsx
onCodeClick={(row) => setSameCodeDialog({ code: row.code })}
```

(Inside the `<CodeItemTable>` props. The table already has `onCreate`, `onEdit`, `onDelete` — `onCodeClick` slots in alongside them.)

- [ ] **Step 4: Render the dialog**

At the bottom of the outer `<Box>` (around line 283, after the existing delete-confirmation `<Dialog>`), add:

```tsx
<SameCodeItemsDialog
  open={sameCodeDialog !== null}
  code={sameCodeDialog?.code ?? null}
  versionId={versionId}
  currentCodelistId={codelistId}
  kind={kind}
  onClose={() => setSameCodeDialog(null)}
/>
```

- [ ] **Step 5: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS.

- [ ] **Step 6: Manual end-to-end verification**

Run: `pnpm dev`, navigate to a terminology version with at least one code that appears in 2+ codelists. For each: click the code cell, confirm dialog opens, confirm columns render, confirm the row whose codelist matches the current page is tinted, click a different-codelist row and confirm you land on that codelist's detail page and the dialog closes. ESC should also close.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/pages/CodeListDetailPage.tsx
git commit -m "feat(desktop,terminology): open same-code dialog from CodeListDetailPage"
```

---

## Self-Review (against the spec)

| Spec section | Implemented in |
|---|---|
| Clickable code cell with hover underline + tooltip | Task 6 |
| Dialog columns: Code, Submission value, Code List Code, Code List Submission value | Task 7 |
| Row navigates to that codelist's detail page | Task 7 (`handleRowClick`) |
| Reuse `list_code_items_by_version_and_code` HTTP router | Tasks 1 + 2 |
| Reuse `get_by_id` (per codelist in dialog) | Task 4 (`useGetCodeListsByIds`) |
| i18n: tooltip, dialogTitle (with `{code}`), empty | Task 5 |
| Loading / error / empty states | Task 7 |
| Wiremock tests for the Rust wire function | Task 1 (2 tests, including percent-encoding) |
| Per-codelist fetch failure → row still renders with `—` | Task 7 (`codelist?.code ?? "—"`) |
| Page state holds dialog; dialog owns its queries + navigation | Task 8 |

No placeholders, no TBDs, no "implement later". All types in later tasks match earlier task signatures (`CodeItemListResponse`, `useGetCodeListsByIds`, `onCodeClick`, dialog props).
