# Terminology — Global Term Search Page — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a new `GlobalTermSearch` page to the `terminology` feature that searches code lists and code items across an entire terminology version with a single fragment, navigable from `TerminologyPage` via a new `ManageSearch` icon button.

**Architecture:** Backend (`aegis-server`) already supports cross-codelist `list_code_items` with optional `version_id` and `codelist_id`. The Tauri HTTP + command surface must be widened to expose `version_id` as a new positional arg and `codelist_id` as optional, both passed through `list_paged`. The frontend gains new hooks `useSearchCodeLists` / `useSearchCodeItems`, a new `codeItemsGlobal` query key, and a new page with two private table components. `TerminologyPage` gains one new icon button.

**Tech Stack:** Tauri (Rust + React), TanStack Router/Query, MUI, react-i18n via `@aegis/ui/i18n`, wiremock (Rust tests).

## Global Constraints

- Spec: [`docs/superpowers/specs/2026-08-21-global-term-search-design.md`](../specs/2026-08-21-global-term-search-design.md)
- Code style: existing workspace conventions (snake_case Rust, camelCase TS, RFC-3339 datetime formatting).
- TS run scripts: `pnpm test`, `pnpm typecheck` (defined in `apps/desktop/aegis-desktop/package.json`).
- Rust test runner: `cargo test -p aegis-desktop`.
- Wire format: `serde(rename_all = "camelCase")` on responses, snake_case `arguments` for `%`-encoded queries (matches existing `list_paged`).
- All reused i18n keys exist in `lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts`. New keys go into the `terminology.search.*` and `terminology.codeitem.*` namespaces.
- **No new dependencies.** No server changes. No new TS DTO types.
- Branch: `feat/desktop_terminology-code-global-search`.

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs` | Modify | `CodeItemListQuery.codelist_id` → `Option<i64>`; add `version_id: Option<i64>`; `list_paged` skips both query params when `None`; update existing tests for `Some(...)`; add new tests. |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs` | Modify | `list_code_items` command: `codelist_id: i64` → `Option<i64>`; add `version_id: Option<i64>` arg; wire to updated `CodeItemListQuery`. |
| `apps/desktop/aegis-desktop/src/shared/api/index.ts` | Modify | `api.listCodeItems`: accept `codelistId: number \| null` and `options.versionId?: number \| null`. |
| `apps/desktop/aegis-desktop/src/shared/query/keys.ts` | Modify | Add `queryKeys.terminology.codeItemsGlobal(versionId, fragment)`. |
| `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts` | Modify | Add `useSearchCodeLists` and `useSearchCodeItems`. |
| `lib/packages/ui/src/i18n/locales/en.ts` | Modify | Add 8 keys under `terminology.search.*` and `terminology.codeitem.{field.codelist, loadFailed.search}`. |
| `lib/packages/ui/src/i18n/locales/zhCN.ts` | Modify | Same 8 keys, Chinese translations. |
| `apps/desktop/aegis-desktop/src/features/terminology/pages/GlobalTermSearchPage.tsx` | Create | New page with two private table components + `CodelistNameCell`. |
| `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/$kind/search.tsx` | Create | New route file. |
| `apps/desktop/aegis-desktop/src/features/terminology/pages/index.ts` | Modify | Re-export `GlobalTermSearchPage`. |
| `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx` | Modify | Add `ManageSearch` icon button to top action row. |

---

## Task 1: Tauri HTTP layer — make `codelist_id` and `version_id` optional (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs`
- Test: same file, inside `#[cfg(test)] mod tests`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone)]
  pub struct CodeItemListQuery {
      pub codelist_id: Option<i64>,
      pub version_id: Option<i64>,
      pub fragment: Option<String>,
      pub offset: u32,
      pub limit: u32,
  }
  ```
  `list_paged` builds the path `/api/terminology/code-items?offset=…&limit=…`, then conditionally appends `&codelistId=…` and `&versionId=…` only when `Some`, then conditionally `&fragment=…` when non-empty (preserving the existing whitespace-trim rule).

- [ ] **Step 1: Add the failing tests**

Append to `mod tests` in `http/terminology/code_item.rs` (after the existing `list_paged_round_trips_camel_case_next_offset` test):

```rust
#[tokio::test]
async fn list_paged_with_none_codelist_id_omits_query_param() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/terminology/code-items"))
        .and(query_param("versionId", "7"))
        .and(query_param("fragment", "AE"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [item_json(1, "AE")]
        })))
        .mount(&server)
        .await;
    let page = list_paged(
        &client(&server),
        CodeItemListQuery {
            codelist_id: None,
            version_id: Some(7),
            fragment: Some("AE".into()),
            offset: 0,
            limit: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(page.items.len(), 1);
}

#[tokio::test]
async fn list_paged_with_some_codelist_id_includes_query_param() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/terminology/code-items"))
        .and(query_param("codelistId", "11"))
        .and(query_param("offset", "0"))
        .and(query_param("limit", "20"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [item_json(1, "X")]
        })))
        .mount(&server)
        .await;
    let page = list_paged(
        &client(&server),
        CodeItemListQuery {
            codelist_id: Some(11),
            version_id: None,
            fragment: None,
            offset: 0,
            limit: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(page.items.len(), 1);
}

#[tokio::test]
async fn list_paged_with_some_version_id_includes_query_param() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/terminology/code-items"))
        .and(query_param("versionId", "7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [item_json(1, "Y")]
        })))
        .mount(&server)
        .await;
    let page = list_paged(
        &client(&server),
        CodeItemListQuery {
            codelist_id: None,
            version_id: Some(7),
            fragment: None,
            offset: 0,
            limit: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(page.items.len(), 1);
}

#[tokio::test]
async fn list_paged_with_none_version_id_omits_query_param() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/terminology/code-items"))
        .and(query_param("codelistId", "11"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [item_json(1, "Z")]
        })))
        .mount(&server)
        .await;
    let page = list_paged(
        &client(&server),
        CodeItemListQuery {
            codelist_id: Some(11),
            version_id: None,
            fragment: None,
            offset: 0,
            limit: 20,
        },
    )
    .await
    .unwrap();
    assert_eq!(page.items.len(), 1);
}
```

- [ ] **Step 2: Run the new tests to confirm they fail**

Run: `cargo test -p aegis-desktop code_item::tests::list_paged_with`
Expected: compile error — `CodeItemListQuery` has no field `version_id`; existing tests fail because `codelist_id` is `i64` not `Option<i64>`.

- [ ] **Step 3: Update `CodeItemListQuery`**

In `http/terminology/code_item.rs`, change the struct definition (around line 38):

```rust
#[derive(Debug, Clone)]
pub struct CodeItemListQuery {
    pub codelist_id: Option<i64>,          // was i64
    pub version_id: Option<i64>,           // new
    pub fragment: Option<String>,
    pub offset: u32,
    pub limit: u32,
}
```

- [ ] **Step 4: Update `list_paged`**

Replace the function body (around line 136):

```rust
pub async fn list_paged(
    c: &HttpClient,
    q: CodeItemListQuery,
) -> Result<CodeItemPagedResponse, ApiError> {
    let mut path = String::from("/api/terminology/code-items?offset=");
    path.push_str(&q.offset.to_string());
    path.push_str("&limit=");
    path.push_str(&q.limit.to_string());
    if let Some(id) = q.codelist_id {
        path.push_str("&codelistId=");
        path.push_str(&id.to_string());
    }
    if let Some(v) = q.version_id {
        path.push_str("&versionId=");
        path.push_str(&v.to_string());
    }
    if let Some(f) = q.fragment.as_deref().filter(|s| !s.trim().is_empty()) {
        path.push_str("&fragment=");
        path.push_str(&percent_encode_fragment(f));
    }
    c.request(reqwest::Method::GET, &path, None::<&()>).await
}
```

- [ ] **Step 5: Update the existing tests to wrap `codelist_id` in `Some`**

There are 5 existing tests that pass `codelist_id: 11` (literal). Change each to `codelist_id: Some(11)` and add `version_id: None` to each `CodeItemListQuery` literal. The tests are:

- `list_paged_returns_first_page_with_next_offset`
- `list_paged_returns_no_next_offset_on_last_page`
- `list_paged_with_fragment_includes_fragment_query_param`
- `list_paged_with_whitespace_fragment_omits_query_param`
- `list_paged_round_trips_camel_case_next_offset`

None of these tests assert on `versionId` because the wiremock does not match it. Adding `version_id: None` keeps the wire shape identical (no `versionId=` in the URL).

- [ ] **Step 6: Run all `code_item` tests**

Run: `cargo test -p aegis-desktop code_item::`
Expected: all tests pass (5 updated + 4 new = 9 total).

- [ ] **Step 7: Run the whole `aegis-desktop` test suite**

Run: `cargo test -p aegis-desktop`
Expected: all tests pass; no regressions.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs
git commit -m "feat(desktop,tauri): make code-item list query accept optional codelist_id and version_id"
```

---

## Task 2: Tauri command — add `version_id` arg, make `codelist_id` optional

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs`

**Interfaces:**
- Produces:
  ```rust
  #[tauri::command]
  pub async fn list_code_items(
      client: State<'_, HttpClient>,
      codelist_id: Option<i64>,
      version_id: Option<i64>,
      fragment: Option<String>,
      offset: u32,
      limit: u32,
  ) -> Result<CodeItemPagedResponse, ApiError>;
  ```
  Tauri command name: `list_code_items`.

- [ ] **Step 1: Update the command signature**

In `commands/terminology/code_item.rs`, replace the existing `list_code_items` function:

```rust
#[tauri::command]
pub async fn list_code_items(
    client: State<'_, HttpClient>,
    codelist_id: Option<i64>,
    version_id: Option<i64>,
    fragment: Option<String>,
    offset: u32,
    limit: u32,
) -> Result<CodeItemPagedResponse, ApiError> {
    code_item::list_paged(
        &client,
        CodeItemListQuery {
            codelist_id,
            version_id,
            fragment,
            offset,
            limit,
        },
    )
    .await
}
```

The existing `use` import for `CodeItemListQuery` is unchanged.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p aegis-desktop`
Expected: success. The command's new positional args are reflected in the auto-generated TypeScript binding (regenerated by `pnpm build` later — the frontend is updated in Task 3).

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/code_item.rs
git commit -m "feat(desktop,tauri): expose version_id arg on list_code_items"
```

---

## Task 3: TS API wrapper, query key, search hooks

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts`
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts`
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts`

**Interfaces:**
- Produces:
  ```ts
  api.listCodeItems(
      codelistId: number | null,
      options: CodeItemListQuery & { versionId?: number | null } = {},
  ): Promise<PagedCodeItemListResponse>

  queryKeys.terminology.codeItemsGlobal(versionId: number, fragment: string)
      : readonly ["terminology", "codeItemsGlobal", number, string]

  useSearchCodeLists(
      versionId: number | null,
      options: ListPagedOptions,
  ): UseInfiniteQueryResult<PagedCodeListListResponse, ApiError>

  useSearchCodeItems(
      versionId: number | null,
      options: ListPagedOptions,
  ): UseInfiniteQueryResult<PagedCodeItemListResponse, ApiError>
  ```

- [ ] **Step 1: Add the query key**

In `apps/desktop/aegis-desktop/src/shared/query/keys.ts`, inside `terminology:`, after the existing `codeItems` entry (around line 32), add:

```ts
codeItemsGlobal: (versionId: number, fragment: string) =>
  ["terminology", "codeItemsGlobal", versionId, fragment] as const,
```

- [ ] **Step 2: Update `api.listCodeItems`**

In `apps/desktop/aegis-desktop/src/shared/api/index.ts`, replace the `listCodeItems` wrapper (around line 161):

```ts
listCodeItems: (
  codelistId: number | null,
  options: CodeItemListQuery & { versionId?: number | null } = {},
): Promise<PagedCodeItemListResponse> =>
  call<PagedCodeItemListResponse>("list_code_items", {
    codelistId: codelistId ?? undefined,
    versionId: options.versionId ?? undefined,
    fragment: options.fragment,
    offset: options.offset,
    limit: options.limit,
  }),
```

Existing callers (`useListCodeItems`) pass a non-null `codelistId` and no `versionId`, so the wire shape for them is unchanged (both keys become `undefined` ⇒ omitted from the Tauri args object).

- [ ] **Step 3: Add `useSearchCodeLists` to the data hooks**

In `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts`, append to the bottom of the file (after `useListCodeItems`):

```ts
/**
 * Version-scoped code-list search. Shares the `codeLists` query key with
 * `useListCodeLists`, so a search query caches identically to a list query.
 * The hook is `enabled` only when `versionId` is a positive number; the page
 * additionally chooses not to render the table when the fragment is empty.
 */
export const useSearchCodeLists = (
  versionId: number | null,
  options: ListPagedOptions,
) =>
  useInfiniteQuery<PagedCodeListListResponse, ApiError>({
    queryKey: queryKeys.terminology.codeLists(versionId ?? 0, options.fragment ?? ""),
    queryFn: ({ pageParam }) =>
      api.listCodeLists(versionId as number, {
        fragment: options.fragment?.trim() || undefined,
        offset: pageParam as number,
        limit: PAGE_SIZE,
      }),
    initialPageParam: 0,
    getNextPageParam: (last) => last.nextOffset,
    enabled: versionId != null && versionId > 0,
  });
```

- [ ] **Step 4: Add `useSearchCodeItems` to the data hooks**

Append after `useSearchCodeLists`:

```ts
/**
 * Cross-codelist code-item search scoped to a single version. Uses a
 * dedicated `codeItemsGlobal` key to avoid collisions with the per-codelist
 * `codeItems` cache. `versionId` is sent to the server; `codelistId` is
 * omitted (null) so the server applies only the version filter.
 */
export const useSearchCodeItems = (
  versionId: number | null,
  options: ListPagedOptions,
) =>
  useInfiniteQuery<PagedCodeItemListResponse, ApiError>({
    queryKey: queryKeys.terminology.codeItemsGlobal(versionId ?? 0, options.fragment ?? ""),
    queryFn: ({ pageParam }) =>
      api.listCodeItems(null, {
        versionId,
        fragment: options.fragment?.trim() || undefined,
        offset: pageParam as number,
        limit: PAGE_SIZE,
      }),
    initialPageParam: 0,
    getNextPageParam: (last) => last.nextOffset,
    enabled: versionId != null && versionId > 0,
  });
```

- [ ] **Step 5: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/index.ts \
        apps/desktop/aegis-desktop/src/shared/query/keys.ts \
        apps/desktop/aegis-desktop/src/features/terminology/data/list.ts
git commit -m "feat(desktop,terminology): add search hooks for cross-codelist term search"
```

---

## Task 4: i18n keys (en + zhCN)

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Produces: 8 new keys:
  - `terminology.search.open`
  - `terminology.search.backTooltip`
  - `terminology.search.placeholder`
  - `terminology.search.tab.codelists`
  - `terminology.search.tab.codeitems`
  - `terminology.search.emptyInput`
  - `terminology.codeitem.field.codelist`
  - `terminology.codeitem.loadFailed.search` (contains `{message}` placeholder)

- [ ] **Step 1: Read the insertion point in `en.ts`**

Open `lib/packages/ui/src/i18n/locales/en.ts`. Find the `terminology.import.*` block and identify the next `terminology.*` block (or the closing `};` of the object). Insert the new keys immediately after the last existing `terminology.search.*` neighbor if any, otherwise after `terminology.import.*`.

- [ ] **Step 2: Add the English strings**

Insert the following block into `en.ts`:

```ts
  'terminology.search.open': 'Search across this version',
  'terminology.search.backTooltip': 'Back to terminology',
  'terminology.search.placeholder': 'Search code lists and code items',
  'terminology.search.tab.codelists': 'Code Lists',
  'terminology.search.tab.codeitems': 'Code Items',
  'terminology.search.emptyInput':
    'Type a search term to find code lists or code items',
  'terminology.codeitem.field.codelist': 'Codelist',
  'terminology.codeitem.loadFailed.search': 'Failed to load code items: {message}',
```

(The leading 2-space indent matches the rest of the file.)

- [ ] **Step 3: Add the Chinese strings**

Insert the corresponding block into `zhCN.ts` at the same insertion point:

```ts
  'terminology.search.open': '跨版本搜索',
  'terminology.search.backTooltip': '返回术语',
  'terminology.search.placeholder': '搜索代码列表与代码项',
  'terminology.search.tab.codelists': '代码列表',
  'terminology.search.tab.codeitems': '代码项',
  'terminology.search.emptyInput': '请输入搜索关键字以查找代码列表或代码项',
  'terminology.codeitem.field.codelist': '代码列表',
  'terminology.codeitem.loadFailed.search': '加载代码项失败：{message}',
```

- [ ] **Step 4: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS — the `TranslationKey` union in `locales/index.ts` is derived from `en.ts`, so a missing zhCN entry would error.

- [ ] **Step 5: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(i18n): add keys for global term search page"
```

---

## Task 5: Route file + `GlobalTermSearchPage`

**Files:**
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/$kind/search.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/pages/GlobalTermSearchPage.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/pages/index.ts` (re-export)

**Interfaces:**
- Public path: `/terminology/{sdtm|adam}/search`
- Path params: `kind: TerminologyKind`
- Search params: `{ versionId?: number }`
- `GlobalTermSearchPage`: no props; reads `kind` from path params and `versionId` from search.

- [ ] **Step 1: Create the route file**

Create `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/$kind/search.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import type { TerminologyKind } from "../../../../../../shared/api";
import { GlobalTermSearchPage } from "../../../../../../features/terminology";

const KIND_VALUES: readonly TerminologyKind[] = ["sdtm", "adam"];

export const Route = createFileRoute(
  "/_authed/_layout/terminology/$kind/search",
)({
  parseParams: (raw) => ({
    kind: KIND_VALUES.includes(raw.kind as TerminologyKind)
      ? (raw.kind as TerminologyKind)
      : "sdtm",
  }),
  stringifyParams: ({ kind }) => ({ kind }),
  validateSearch: (raw): { versionId?: number } => ({
    versionId:
      typeof raw.versionId === "string"
        ? raw.versionId === "" ? undefined : Number(raw.versionId)
        : typeof raw.versionId === "number" ? raw.versionId : undefined,
  }),
  component: GlobalTermSearchPage,
});
```

- [ ] **Step 2: Re-export the page from the feature barrel**

In `apps/desktop/aegis-desktop/src/features/terminology/pages/index.ts`, append:

```ts
export * from "./GlobalTermSearchPage";
```

- [ ] **Step 3: Create the page file with the page + sub-components**

Create `apps/desktop/aegis-desktop/src/features/terminology/pages/GlobalTermSearchPage.tsx`:

```tsx
import { useState, type ReactNode } from "react";
import { getRouteApi, useNavigate } from "@tanstack/react-router";
import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import {
  ArrowBack as ArrowBackIcon,
  Launch as LaunchIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import { InfiniteScrollSentinel } from "../../../shared/components/InfiniteScrollSentinel";
import { useDebouncedValue } from "../../../shared/hooks/useDebouncedValue";
import type {
  ApiError,
  CodeItemView,
  CodeListView,
  TerminologyKind,
} from "../../../shared/api";
import {
  useGetCodeList,
  useSearchCodeItems,
  useSearchCodeLists,
} from "../data";
import { DescriptionsCell } from "../components/DescriptionsCell";
import { TermFilterBar } from "../components/TermFilterBar";

type Tab = "codelists" | "codeitems";

const routeApi = getRouteApi("/_authed/_layout/terminology/$kind/search");

/**
 * One-cell rendering of a parent codelist's name. Falls back to the
 * numeric id while the codelist is loading or has errored, so the table
 * never breaks because of a single missing codelist.
 */
function CodelistNameCell({ codelistId }: { codelistId: number }) {
  const { data } = useGetCodeList(codelistId);
  return <>{data?.name ?? `#${codelistId}`}</>;
}

interface SearchCodeListTableProps {
  rows: CodeListView[];
  loading: boolean;
  error: ApiError | null;
  onRetry: () => void;
  onOpen: (row: CodeListView) => void;
  bottomSlot: (scrollEl: HTMLElement | null) => ReactNode;
}

function SearchCodeListTable({
  rows,
  loading,
  error,
  onRetry,
  onOpen,
  bottomSlot,
}: SearchCodeListTableProps) {
  const { t } = useI18n();
  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null);

  if (error && rows.length === 0) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Alert severity="error">
          {t("terminology.codelist.loadFailed", {
            message: errorMessage(error),
          })}
        </Alert>
        <Box>
          <Button onClick={onRetry}>{t("common.retry")}</Button>
        </Box>
      </Box>
    );
  }

  const showSpinner = loading && rows.length === 0;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      {showSpinner && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}
      <TableContainer
        component={Paper}
        ref={setScrollEl}
        sx={{ maxHeight: "calc(100vh - 240px)" }}
      >
        <Table size="small" stickyHeader>
          <TableHead>
            <TableRow>
              <TableCell>{t("terminology.codelist.field.code")}</TableCell>
              <TableCell>{t("terminology.codelist.field.name")}</TableCell>
              <TableCell>
                {t("terminology.codelist.field.submissionValue")}
              </TableCell>
              <TableCell>
                {t("terminology.codelist.field.descriptions")}
              </TableCell>
              <TableCell sx={{ width: 60 }} align="right" />
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={row.id} hover>
                <TableCell>
                  <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
                    <span>{row.code}</span>
                    {row.extensible && (
                      <Tooltip title={t("terminology.extensible")}>
                        <Chip label="EXT" size="small" />
                      </Tooltip>
                    )}
                  </Box>
                </TableCell>
                <TableCell>{row.name}</TableCell>
                <TableCell>{row.submissionValue}</TableCell>
                <TableCell>
                  <DescriptionsCell
                    synonym={row.synonym}
                    definition={row.definition}
                    nciPreferredTerm={row.nciPreferredTerm}
                  />
                </TableCell>
                <TableCell align="right">
                  <Tooltip title={t("common.open")}>
                    <IconButton
                      size="small"
                      aria-label={`open ${row.code}`}
                      onClick={() => onOpen(row)}
                    >
                      <LaunchIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="text.secondary">
              {t("terminology.codelist.noMatches")}
            </Typography>
          </Box>
        )}
        {bottomSlot?.(scrollEl)}
      </TableContainer>
    </Box>
  );
}

interface SearchCodeItemTableProps {
  rows: CodeItemView[];
  loading: boolean;
  error: ApiError | null;
  onRetry: () => void;
  onOpen: (row: CodeItemView) => void;
  bottomSlot: (scrollEl: HTMLElement | null) => ReactNode;
}

function SearchCodeItemTable({
  rows,
  loading,
  error,
  onRetry,
  onOpen,
  bottomSlot,
}: SearchCodeItemTableProps) {
  const { t } = useI18n();
  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null);

  if (error && rows.length === 0) {
    return (
      <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
        <Alert severity="error">
          {t("terminology.codeitem.loadFailed.search", {
            message: errorMessage(error),
          })}
        </Alert>
        <Box>
          <Button onClick={onRetry}>{t("common.retry")}</Button>
        </Box>
      </Box>
    );
  }

  const showSpinner = loading && rows.length === 0;

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
      {showSpinner && (
        <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
          <CircularProgress />
        </Box>
      )}
      <TableContainer
        component={Paper}
        ref={setScrollEl}
        sx={{ maxHeight: "calc(100vh - 240px)" }}
      >
        <Table size="small" stickyHeader>
          <TableHead>
            <TableRow>
              <TableCell>{t("terminology.codeitem.field.code")}</TableCell>
              <TableCell>{t("terminology.codeitem.field.codelist")}</TableCell>
              <TableCell>
                {t("terminology.codeitem.field.submissionValue")}
              </TableCell>
              <TableCell>
                {t("terminology.codeitem.field.descriptions")}
              </TableCell>
              <TableCell sx={{ width: 60 }} align="right" />
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={row.id} hover>
                <TableCell>{row.code}</TableCell>
                <TableCell>
                  <CodelistNameCell codelistId={row.codelistId} />
                </TableCell>
                <TableCell>{row.submissionValue}</TableCell>
                <TableCell>
                  <DescriptionsCell
                    synonym={row.synonym}
                    definition={row.definition}
                    nciPreferredTerm={row.nciPreferredTerm}
                  />
                </TableCell>
                <TableCell align="right">
                  <Tooltip title={t("common.open")}>
                    <IconButton
                      size="small"
                      aria-label={`open ${row.code}`}
                      onClick={() => onOpen(row)}
                    >
                      <LaunchIcon fontSize="small" />
                    </IconButton>
                  </Tooltip>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
        {!showSpinner && rows.length === 0 && (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <Typography color="text.secondary">
              {t("terminology.codeitem.noMatches")}
            </Typography>
          </Box>
        )}
        {bottomSlot?.(scrollEl)}
      </TableContainer>
    </Box>
  );
}

export function GlobalTermSearchPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const params = routeApi.useParams();
  const search = routeApi.useSearch();

  const kind = (params.kind as TerminologyKind) ?? "sdtm";
  const versionId = search.versionId ?? null;

  const [query, setQuery] = useState("");
  const [tab, setTab] = useState<Tab>("codelists");

  const debouncedFragment = useDebouncedValue(query, {
    delayMs: 300,
    maxWaitMs: 1000,
  });
  const trimmedFragment = debouncedFragment.trim();
  const showTables = trimmedFragment.length > 0;

  const codeListsQuery = useSearchCodeLists(versionId, {
    fragment: debouncedFragment,
  });
  const codeItemsQuery = useSearchCodeItems(versionId, {
    fragment: debouncedFragment,
  });

  const goBack = () => {
    void navigate({
      to: kind === "sdtm" ? "/terminology/sdtm" : "/terminology/adam",
      search: versionId != null ? { versionId } : undefined,
    });
  };

  const openCodelist = (codelistId: number) => {
    void navigate({
      to: "/terminology/$kind/codelists/$codelistId",
      params: { kind, codelistId: String(codelistId) },
      search: versionId != null ? { versionId } : undefined,
    });
  };

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
        <Tooltip title={t("terminology.search.backTooltip")}>
          <IconButton
            aria-label={t("terminology.search.backTooltip")}
            onClick={goBack}
          >
            <ArrowBackIcon />
          </IconButton>
        </Tooltip>
        <Box sx={{ flex: 1 }}>
          <TermFilterBar
            query={query}
            onQueryChange={setQuery}
            placeholder={t("terminology.search.placeholder")}
          />
        </Box>
      </Box>

      <Box>
        <ToggleButtonGroup
          exclusive
          value={tab}
          onChange={(_, v: Tab | null) => {
            if (v) setTab(v);
          }}
          size="small"
          aria-label={t("terminology.search.open")}
        >
          <ToggleButton value="codelists">
            {t("terminology.search.tab.codelists")}
          </ToggleButton>
          <ToggleButton value="codeitems">
            {t("terminology.search.tab.codeitems")}
          </ToggleButton>
        </ToggleButtonGroup>
      </Box>

      {!showTables ? (
        <Box sx={{ display: "flex", justifyContent: "center", py: 8 }}>
          <Typography color="text.secondary">
            {t("terminology.search.emptyInput")}
          </Typography>
        </Box>
      ) : tab === "codelists" ? (
        <SearchCodeListTable
          rows={codeListsQuery.data?.pages.flatMap((p) => p.items) ?? []}
          loading={codeListsQuery.isLoading}
          error={codeListsQuery.error}
          onRetry={() => void codeListsQuery.refetch()}
          onOpen={(row) => openCodelist(row.id)}
          bottomSlot={(scrollEl) => (
            <InfiniteScrollSentinel
              root={scrollEl}
              onIntersect={() => void codeListsQuery.fetchNextPage()}
              hasMore={codeListsQuery.hasNextPage ?? false}
              loading={codeListsQuery.isFetchingNextPage}
            />
          )}
        />
      ) : (
        <SearchCodeItemTable
          rows={codeItemsQuery.data?.pages.flatMap((p) => p.items) ?? []}
          loading={codeItemsQuery.isLoading}
          error={codeItemsQuery.error}
          onRetry={() => void codeItemsQuery.refetch()}
          onOpen={(row) => openCodelist(row.codelistId)}
          bottomSlot={(scrollEl) => (
            <InfiniteScrollSentinel
              root={scrollEl}
              onIntersect={() => void codeItemsQuery.fetchNextPage()}
              hasMore={codeItemsQuery.hasNextPage ?? false}
              loading={codeItemsQuery.isFetchingNextPage}
            />
          )}
        />
      )}
    </Box>
  );
}
```

> Notes on imports:
> - `useGetCodeList`, `useSearchCodeItems`, `useSearchCodeLists` — exported from `../data` (re-exported from `data/list.ts` via `data/index.ts`).
> - `DescriptionsCell`, `TermFilterBar` — already exist; check imports match the existing paths.
> - MUI icons — `ArrowBack` and `Launch` already aliased in the existing codebase; `ManageSearch` is a standard `@mui/icons-material` entry and is exported via the `@aegis/ui/icons` barrel.

- [ ] **Step 4: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS. If `pnpm typecheck` complains about missing `useSearchCodeLists` / `useSearchCodeItems` exports, Task 3 was not completed — return there.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/\$kind/search.tsx \
        apps/desktop/aegis-desktop/src/features/terminology/pages/GlobalTermSearchPage.tsx \
        apps/desktop/aegis-desktop/src/features/terminology/pages/index.ts
git commit -m "feat(desktop,terminology): add GlobalTermSearchPage with search route"
```

> **Manual check after this task:** Run `pnpm dev`, navigate to `/terminology/sdtm?versionId=1`, click an (as-yet-nonexistent) ManageSearch button if visible, OR manually visit `/terminology/sdtm/search?versionId=1`. Type a known code-list code, confirm both tabs render, click LaunchIcon and confirm navigation to the detail page preserves version.

---

## Task 6: Add `ManageSearch` icon button to `TerminologyPage`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx`

**Interfaces:**
- Adds one icon button to the top action row, immediately before `<ImportButton>`, gated on `selectedVersionId != null`. Always visible (read by all users, not gated on `canMutate`).

- [ ] **Step 1: Add `IconButton` and `Tooltip` to the MUI import**

In `TerminologyPage.tsx`, replace the existing `@aegis/ui/mui` import (lines 3-11) with:

```ts
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  IconButton,
  Tooltip,
} from "@aegis/ui/mui";
```

- [ ] **Step 2: Add the `ManageSearch` icon import**

Add (next to the existing `@aegis/ui/i18n` import):

```ts
import { ManageSearch as ManageSearchIcon } from "@aegis/ui/icons";
```

- [ ] **Step 3: Insert the icon button before `<ImportButton>`**

In the top action row `<Box>` (around line 128-144), insert this block immediately before `{canMutate && <ImportButton kind={kind} />}`:

```tsx
<Tooltip title={t("terminology.search.open")}>
  <IconButton
    aria-label={t("terminology.search.open")}
    onClick={() =>
      navigate({
        to: "/terminology/$kind/search",
        params: { kind },
        search:
          selectedVersionId != null ? { versionId: selectedVersionId } : undefined,
      })
    }
    disabled={selectedVersionId == null}
  >
    <ManageSearchIcon />
  </IconButton>
</Tooltip>
```

The `navigate` and `selectedVersionId` bindings already exist in `TerminologyPage` (lines 48, 70). `t` is destructured from `useI18n` on line 47.

- [ ] **Step 4: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx
git commit -m "feat(desktop,terminology): add ManageSearch icon to TerminologyPage"
```

---

## Task 7: Final verification

**Files:** none — verification only.

- [ ] **Step 1: Run the full Tauri test suite**

Run: `cargo test -p aegis-desktop`
Expected: all tests pass (the 4 new wiremock tests from Task 1 plus all pre-existing tests).

- [ ] **Step 2: Run the Tauri type check**

Run: `cargo check -p aegis-desktop`
Expected: success.

- [ ] **Step 3: Run the desktop TS type check**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS.

- [ ] **Step 4: Run the desktop build**

Run: `cd apps/desktop/aegis-desktop && pnpm build`
Expected: success. The build regenerates `routeTree.gen.ts` so the new `/terminology/$kind/search` route is registered.

- [ ] **Step 5: Manual smoke test**

Run: `cd apps/desktop/aegis-desktop && pnpm dev`

Verify:
1. From `/terminology/sdtm?versionId=1`, the new `ManageSearch` icon appears in the top action row, immediately to the left of `Import`.
2. Clicking the icon lands on `/terminology/sdtm/search?versionId=1`.
3. The back-arrow returns to `/terminology/sdtm?versionId=1`.
4. With the search input empty, the hint `terminology.search.emptyInput` is shown.
5. Type a known code-list code; the Code Lists tab shows matching rows with Code / Name / Submission value / Descriptions columns.
6. Switch to the Code Items tab; matching items across all codelists in version 1 appear. The Codelist column shows each item's parent codelist name (after a brief moment).
7. Click LaunchIcon on a Code Lists row → lands on `/terminology/sdtm/codelists/{id}?versionId=1`.
8. Click LaunchIcon on a Code Items row → lands on `/terminology/sdtm/codelists/{row.codelistId}?versionId=1`.
9. Scroll to the bottom of either table and confirm more rows load.
10. Repeat 1-9 from `/terminology/adam`.

---

## Self-Review (against the spec)

| Spec section | Implemented in |
|---|---|
| ManageSearch icon on TerminologyPage top action row, before ImportButton | Task 6 |
| Back-arrow icon button + search field + ToggleButtonGroup | Task 5 |
| Two private tables (Code Lists / Code Items) | Task 5 |
| Code Items table includes Codelist column with parent name | Task 5 (`CodelistNameCell`) |
| Empty input → hint, no table | Task 5 (`!showTables` branch) |
| Loading / error / empty / infinite-scroll states on both tables | Task 5 |
| Operation column LaunchIcon → CodeListDetailPage, preserves versionId | Task 5 (`openCodelist`) |
| Route `/terminology/$kind/search` with `versionId` search param | Task 5 |
| `useSearchCodeLists` / `useSearchCodeItems` infinite hooks | Task 3 |
| `api.listCodeItems` accepts `null` codelistId + `versionId` | Task 3 |
| `codeItemsGlobal` query key | Task 3 |
| Tauri `list_code_items` accepts `version_id` + optional `codelist_id` | Tasks 1 + 2 |
| Tauri HTTP `CodeItemListQuery` widens + URL builder skips None | Task 1 |
| Wiremock tests for None/Some cases | Task 1 (4 new + 5 updated) |
| i18n keys (8) in both en and zhCN | Task 4 |
| Manual verification recipe | Task 7 |

No placeholders, no TBDs. Types match across tasks:
- `CodeItemListQuery` Rust struct field order matches what Task 2 wires into `list_paged`.
- `api.listCodeItems(codelistId, options.versionId, …)` signature matches what Task 3's hooks call.
- `useSearchCodeLists` / `useSearchCodeItems` signatures match what Task 5 imports.
- `GlobalTermSearchPage` props (none) match what Task 5's route file wires via `component:`.
- `CodelistNameCell` is a private helper in the same file as its only caller (Task 5).
