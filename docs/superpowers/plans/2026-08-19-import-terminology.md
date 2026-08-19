# Import Terminology Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the placeholder `ImportButton` "coming soon" Snackbar on the SDTM / ADaM terminology pages with a real `ImportTerminology` page that lets an admin/root user pick or drop a CDISC terminology `.xls` / `.xlsx` workbook, runs the existing per-resource create handlers against the server (one call per code list, one batch call per code list's items), and refreshes the cached terminology versions on success.

**Architecture:** Frontend adds a `features/terminology/pages/ImportTerminologyPage.tsx` reachable via `/_authed/_layout/terminology/import?kind=sdtm|adam`. The page uses TanStack React Query to drive a single Tauri command `import_terminology(kind, filepath)`. The command runs `terminology::from_path` in `tokio::task::spawn_blocking`, then sequentially calls the existing `version::create`, `code_list::create`, and (new) `code_item::batch_create` HTTP wrappers. Page is hidden during the import; success/error is shown via a `Snackbar`. Terminology versions are invalidated on success.

**Tech Stack:** Tauri 2, Rust 2021, `terminology` git crate (calamine under the hood), wiremock (Rust tests), React 19, @tanstack/react-router 1, @tanstack/react-query 5, MUI 9 (`ToggleButtonGroup`, `Snackbar`, `Chip`, `CircularProgress`), `@tauri-apps/plugin-dialog`, Vitest 2, Testing Library.

**Spec:** [docs/superpowers/specs/2026-08-19-import-terminology-design.md](../specs/2026-08-19-import-terminology-design.md)

## Global Constraints

- Follow the existing `http/terminology/code_item.rs` and `commands/terminology/code_item.rs` patterns 1:1 — module names, file layout, serde `rename_all`, test style.
- Tauri HTTP DTOs use `#[serde(rename_all = "camelCase")]` to match the existing wire convention.
- The shared `TerminologyKind` enum already lives in `http/dto.rs`; do not redefine it.
- The frontend types mirror the wire shape (`id: i64` → `id: number`).
- The `terminology` git dependency is already declared in `Cargo.toml` at line 33 — no `Cargo.toml` change.
- `@tauri-apps/plugin-dialog` is already declared in `Cargo.toml` and registered as a Tauri plugin — no plugin change.
- `dialog:default` permission is already granted in `src-tauri/capabilities/default.json` — no capability change.
- Tests live in `#[cfg(test)] mod tests` blocks alongside the Rust code, or in `src/test/features/<feature>/` for Vitest tests.
- All work happens on the current branch (`feat/desktop_import-terminology`).
- Commit message prefix `feat(terminology):` for new behavior, `test(terminology):` for tests, `docs(terminology):` for doc-only.

---

## Task 1: Add `Parse` variant to `ApiError`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs:38-67` (the `ApiError` enum)
- Test: same file, inside `mod tests`

**Interfaces:**
- Produces: `ApiError::Parse { message: String }` — a new variant for xls workbook parse failures.

- [ ] **Step 1: Write the failing test**

In `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs`, at the bottom of the existing `#[cfg(test)] mod tests` block, append:

```rust
#[test]
fn parse_error_serializes_camel_case() {
    let e = super::ApiError::Parse { message: "no sheet".into() };
    let j = serde_json::to_string(&e).unwrap();
    assert_eq!(j, r#"{"kind":"parse","message":"no sheet"}"#);
}

#[test]
fn parse_error_roundtrips() {
    let e = super::ApiError::Parse { message: "bad row".into() };
    let j = serde_json::to_string(&e).unwrap();
    let back: super::ApiError = serde_json::from_str(&j).unwrap();
    assert_eq!(e, back);
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cd d:/project/aegis/apps/desktop/aegis-desktop && cargo test -p aegis-desktop parse_error`
Expected: FAIL with compile error "`ApiError::Parse` not found".

- [ ] **Step 3: Add the `Parse` variant to `ApiError`**

In `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs`, inside the `ApiError` enum, after the `Store { message: String }` variant (around line 66), add:

```rust
/// Workbook parse failure (the `terminology` git crate could not read the
/// .xls/.xlsx file). The frontend renders this through `errorMessage(err)`
/// the same way as the other variants.
#[error("workbook parse error: {message}")]
Parse { message: String },
```

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cd d:/project/aegis/apps/desktop/aegis-desktop && cargo test -p aegis-desktop parse_error`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs
git commit -m "feat(terminology): add Parse variant to ApiError for workbook failures"
```

---

## Task 2: Add `BatchCodeItem*` DTOs and `batch_create` HTTP function

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs` (append DTOs after `UpdateCodeItemRequest`; append function after `create`)
- Test: same file, inside `mod tests`

**Interfaces:**
- Consumes: `crate::http::client::HttpClient`
- Produces:
  - `pub struct BatchCodeItemEntry { code, submission_value, synonym, definition, nci_preferred_term: String }`
  - `pub struct BatchCreateCodeItemsRequest { codelist_id: i64, version_id: i64, items: Vec<BatchCodeItemEntry> }`
  - `pub struct BatchCreateCodeItemsResponse { count: usize, codelist_id: i64, version_id: i64 }`
  - `pub async fn batch_create(c, body) -> Result<BatchCreateCodeItemsResponse, ApiError>` — POST `/api/terminology/code-items/batch`

- [ ] **Step 1: Write the failing wiremock + serialization tests**

In `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs`, inside the existing `#[cfg(test)] mod tests` block, append:

```rust
fn batch_response_json(codelist_id: i64, version_id: i64, count: usize) -> serde_json::Value {
    serde_json::json!({
        "count": count, "codelistId": codelist_id, "versionId": version_id
    })
}

#[tokio::test]
async fn batch_create_returns_count() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/terminology/code-items/batch"))
        .respond_with(ResponseTemplate::new(201)
            .set_body_json(batch_response_json(11, 7, 42)))
        .mount(&server)
        .await;
    let resp = batch_create(
        &client(&server),
        BatchCreateCodeItemsRequest {
            codelist_id: 11,
            version_id: 7,
            items: vec![BatchCodeItemEntry {
                code: "Y".into(),
                submission_value: "SV".into(),
                synonym: "syn".into(),
                definition: "def".into(),
                nci_preferred_term: "nci".into(),
            }],
        },
    )
    .await
    .unwrap();
    assert_eq!(resp.count, 42);
    assert_eq!(resp.codelist_id, 11);
    assert_eq!(resp.version_id, 7);
}

#[test]
fn batch_request_serializes_camel_case() {
    let body = BatchCreateCodeItemsRequest {
        codelist_id: 11,
        version_id: 7,
        items: vec![BatchCodeItemEntry {
            code: "Y".into(),
            submission_value: "SV".into(),
            synonym: "syn".into(),
            definition: "def".into(),
            nci_preferred_term: "nci".into(),
        }],
    };
    let j = serde_json::to_string(&body).unwrap();
    assert_eq!(
        j,
        r#"{"codelistId":11,"versionId":7,"items":[{"code":"Y","submissionValue":"SV","synonym":"syn","definition":"def","nciPreferredTerm":"nci"}]}"#
    );
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cd d:/project/aegis/apps/desktop/aegis-desktop && cargo test -p aegis-desktop batch_create`
Expected: FAIL with compile error "cannot find type `BatchCodeItemEntry`".

- [ ] **Step 3: Add the DTOs and the function**

In `apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs`, immediately after the `UpdateCodeItemRequest` struct (around line 67), add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCodeItemEntry {
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsRequest {
    pub codelist_id: i64,
    pub version_id: i64,
    pub items: Vec<BatchCodeItemEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchCreateCodeItemsResponse {
    pub count: usize,
    pub codelist_id: i64,
    pub version_id: i64,
}
```

Then immediately after the existing `create` function (around line 86), add:

```rust
pub async fn batch_create(
    c: &HttpClient,
    body: BatchCreateCodeItemsRequest,
) -> Result<BatchCreateCodeItemsResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        "/api/terminology/code-items/batch",
        Some(&body),
    )
    .await
}
```

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cd d:/project/aegis/apps/desktop/aegis-desktop && cargo test -p aegis-desktop batch_create`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/terminology/code_item.rs
git commit -m "feat(terminology): add batch code-item DTOs and HTTP wrapper"
```

---

## Task 3: Add `import_terminology` Tauri command and register it

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/import.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology.rs` (add `pub mod import;`)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs:45-61` (add the new command to `invoke_handler!`)

**Interfaces:**
- Produces: `#[tauri::command] pub async fn import_terminology(client: State<'_, HttpClient>, kind: TerminologyKind, filepath: String) -> Result<TerminologyVersionViewResponse, ApiError>` — see Section 7.3 of the spec for the full body.

- [ ] **Step 1: Create the command module**

Create `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/import.rs` with:

```rust
//! Tauri command shim for bulk terminology import.
//!
//! Parses an xls/xlsx workbook, then orchestrates the existing
//! per-resource HTTP wrappers to create the version, its code lists,
//! and the items of each code list.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::{ApiError, TerminologyKind};
use crate::http::terminology::code_item::{
    self, BatchCodeItemEntry, BatchCreateCodeItemsRequest,
};
use crate::http::terminology::code_list::{self, CreateCodeListRequest};
use crate::http::terminology::version::{self, CreateTerminologyVersionRequest,
    TerminologyVersionViewResponse};

#[tauri::command]
pub async fn import_terminology(
    client: State<'_, HttpClient>,
    kind: TerminologyKind,
    filepath: String,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    // 1. Parse the workbook off-thread (calamine is sync / CPU-bound).
    let parsed = tokio::task::spawn_blocking(move || terminology::from_path(&filepath))
        .await
        .map_err(|e| ApiError::Parse {
            message: format!("join error: {e}"),
        })?
        .map_err(|e| ApiError::Parse {
            message: e.to_string(),
        })?;

    // 2. Create the version.
    let version_view = version::create(
        &client,
        CreateTerminologyVersionRequest {
            kind,
            name: parsed.name,
        },
    )
    .await?;

    // 3. For each code list, create the list and batch-create its items.
    for cl in parsed.codelist {
        let cl_view = code_list::create(
            &client,
            CreateCodeListRequest {
                version_id: version_view.id,
                code: cl.code,
                extensible: cl.extensible,
                name: cl.name,
                submission_value: cl.submission_value,
                synonym: cl.synonym,
                definition: cl.definition,
                nci_preferred_term: cl.nci_preferred_term,
            },
        )
        .await?;

        if cl.code_list.is_empty() {
            continue;
        }

        code_item::batch_create(
            &client,
            BatchCreateCodeItemsRequest {
                codelist_id: cl_view.id,
                version_id: version_view.id,
                items: cl
                    .code_list
                    .into_iter()
                    .map(|i| BatchCodeItemEntry {
                        code: i.code,
                        submission_value: i.submission_value,
                        synonym: i.synonym,
                        definition: i.definition,
                        nci_preferred_term: i.nci_preferred_term,
                    })
                    .collect(),
            },
        )
        .await?;
    }

    Ok(version_view)
}
```

- [ ] **Step 2: Register the module**

In `apps/desktop/aegis-desktop/src-tauri/src/commands/terminology.rs`, replace the body with:

```rust
//! Tauri command shims for the terminology HTTP layer.
pub mod code_item;
pub mod code_list;
pub mod import;
pub mod version;
```

- [ ] **Step 3: Register the command in `invoke_handler!`**

In `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`, in the `invoke_handler!` macro list, after `commands::terminology::code_item::search_code_items,` (around line 61), add:

```rust
            commands::terminology::import::import_terminology,
```

- [ ] **Step 4: Build to verify it compiles**

Run: `cd d:/project/aegis/apps/desktop/aegis-desktop && cargo build -p aegis-desktop`
Expected: SUCCESS with no errors. Warnings about unused imports are acceptable.

If the build fails with "cannot find function `from_path`" or similar, double-check the import — the `terminology` crate is declared in `Cargo.toml` as `terminology = { git = "...", package = "terminology" }`. The crate's public API is `terminology::from_path` (re-exported from `lib.rs`).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/commands/terminology/import.rs apps/desktop/aegis-desktop/src-tauri/src/commands/terminology.rs apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(terminology): add import_terminology Tauri command"
```

---

## Task 4: Add shared TypeScript batch DTOs and `importTerminology` wrapper

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts` (append after `UpdateCodeItemInput`)
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts` (add wrapper inside the `api` object)
- Test: `apps/desktop/aegis-desktop/src/shared/api.test.ts` (or `src/test/shared/api.test.ts`) — add type-level assertion

**Interfaces:**
- Produces:
  - `interface BatchCodeItemEntry { code, submissionValue, synonym, definition, nciPreferredTerm: string }`
  - `interface BatchCreateCodeItemsInput { codelistId, versionId: number, items: BatchCodeItemEntry[] }`
  - `interface BatchCreateCodeItemsResponse { count, codelistId, versionId: number }`
  - `api.importTerminology(kind, filepath): Promise<TerminologyVersionView>` — calls Tauri command `import_terminology`.

- [ ] **Step 1: Add the TypeScript types**

In `apps/desktop/aegis-desktop/src/shared/api/types.ts`, immediately after the `UpdateCodeItemInput` interface, add:

```ts
export interface BatchCodeItemEntry {
  code: string;
  submissionValue: string;
  synonym: string;
  definition: string;
  nciPreferredTerm: string;
}

export interface BatchCreateCodeItemsInput {
  codelistId: number;
  versionId: number;
  items: BatchCodeItemEntry[];
}

export interface BatchCreateCodeItemsResponse {
  count: number;
  codelistId: number;
  versionId: number;
}
```

- [ ] **Step 2: Add the `api.importTerminology` wrapper**

In `apps/desktop/aegis-desktop/src/shared/api/index.ts`, inside the `api` object literal, add a new method. Place it after the last `search_code_items` wrapper:

```ts
  importTerminology: (
    kind: TerminologyKind,
    filepath: string,
  ): Promise<TerminologyVersionView> =>
    call<TerminologyVersionView>("import_terminology", { kind, filepath }),
```

- [ ] **Step 3: Verify with the existing test suite**

Run: `cd d:/project/aegis/apps/desktop/aegis-desktop && pnpm test -- --run src/shared/api.test.ts`
Expected: existing assertions still pass. If the test file does not exist, run the full suite: `pnpm test -- --run` and confirm no regressions.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts apps/desktop/aegis-desktop/src/shared/api/index.ts
git commit -m "feat(terminology): add batch DTOs and importTerminology wrapper to shared API"
```

---

## Task 5: Add i18n keys for `terminology.import.*` and `common.submit`

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts` (add 7 keys under `terminology.import.*` and the missing `common.submit`)
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts` (matching zh-CN translations)

**Interfaces:**
- Produces: 8 new i18n keys per locale.

- [ ] **Step 1: Add the English strings**

In `lib/packages/ui/src/i18n/locales/en.ts`, add these keys. Use the existing alphabetical position for each key (search for the existing `terminology.*` keys to find the right insertion point — e.g., after `terminology.importComingSoon` or after the last `terminology.*` key):

```ts
  'terminology.import.title': 'Import terminology',
  'terminology.import.subtitle': 'Upload a CDISC terminology workbook',
  'terminology.import.dropZone': 'Drop an .xls or .xlsx file here, or click to choose',
  'terminology.import.fileTypeHint': 'Only .xls or .xlsx files are supported',
  'terminology.import.importing': 'Importing terminology…',
  'terminology.import.success': 'Imported terminology version {name}',
  'terminology.import.failure': 'Import failed: {message}',

  'common.submit': 'Submit',
```

Also remove the existing `'terminology.importComingSoon': 'Terminology import is coming soon'` key (it is no longer used anywhere once `ImportButton` is wired up in Task 7).

- [ ] **Step 2: Add the zh-CN strings**

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, add at the same alphabetical positions:

```ts
  'terminology.import.title': '导入术语',
  'terminology.import.subtitle': '上传 CDISC 术语表',
  'terminology.import.dropZone': '将 .xls 或 .xlsx 文件拖到此处，或点击选择',
  'terminology.import.fileTypeHint': '仅支持 .xls 或 .xlsx 文件',
  'terminology.import.importing': '正在导入术语…',
  'terminology.import.success': '已导入术语版本 {name}',
  'terminology.import.failure': '导入失败：{message}',

  'common.submit': '提交',
```

Also remove the existing `'terminology.importComingSoon': '术语导入即将推出'` key.

- [ ] **Step 3: Verify with the locale type check**

Run: `cd d:/project/aegis/lib/packages/ui && pnpm typecheck` (or `pnpm build` if no typecheck script exists).
Expected: SUCCESS — both locale objects have matching key sets.

If a `pnpm typecheck` script does not exist, run `pnpm exec tsc --noEmit` from `lib/packages/ui`.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(terminology): add i18n keys for import flow and common.submit"
```

---

## Task 6: Add `ImportTerminologyPage` component (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/terminology/pages/ImportTerminologyPage.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/features/terminology/import-terminology-page.test.tsx`

**Interfaces:**
- Consumes: `api.importTerminology(kind, filepath)` from `shared/api`, `queryKeys.terminology.versions()` from `shared/query`, `useI18n` from `@aegis/ui/i18n`, `Route.useSearch()` from the parent route (page reads `kind` from search and lets the user change it).
- Produces: a page component that drives a React Query `useMutation`, renders the ButtonGroup / drop zone / submit / spinner / Snackbar, and invalidates the versions list on success.

- [ ] **Step 1: Write the failing tests**

Create `apps/desktop/aegis-desktop/src/test/features/terminology/import-terminology-page.test.tsx` with:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";
import { TestQueryProvider } from "../../helpers/test-query-provider";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { ImportTerminologyPage } from
  "../../../features/terminology/pages/ImportTerminologyPage";
import { mockCommands } from "../../helpers/tauri-mock";
import { renderInRouter } from "../../helpers/file-route-utils";

const versionView = {
  id: 42,
  kind: "sdtm" as const,
  name: "2026-03-27",
  createdAt: "2026-03-27T00:00:00Z",
  updatedAt: "2026-03-27T00:00:00Z",
};

beforeEach(() => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockReset();
  (open as unknown as ReturnType<typeof vi.fn>).mockReset();
});
afterEach(() => cleanup());

function renderPage(opts: { initialEntries?: string[]; mockImport?: () => unknown } = {}) {
  mockCommands({
    import_terminology: () =>
      opts.mockImport ? opts.mockImport() : versionView,
  });
  return renderInRouter(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <ImportTerminologyPage />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
    { initialEntries: opts.initialEntries ?? ["/terminology/import"] },
  );
}

describe("ImportTerminologyPage — empty form", () => {
  it("renders the back arrow, title, ButtonGroup, drop zone, and disabled submit", async () => {
    await renderPage();
    expect(screen.getByRole("button", { name: /back/i })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /import terminology/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "SDTM" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "ADaM" })).toBeInTheDocument();
    expect(screen.getByText(/drop an \.xls or \.xlsx file here/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /submit/i })).toBeDisabled();
  });
});

describe("ImportTerminologyPage — kind pre-selection", () => {
  it("pre-selects SDTM when ?kind=sdtm is in the URL", async () => {
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    expect(screen.getByRole("button", { name: "SDTM" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "ADaM" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });
});

describe("ImportTerminologyPage — file picker", () => {
  it("calls open() with the right filter and stores the resolved path", async () => {
    (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue("/tmp/sdtm.xls");
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    await userEvent.click(screen.getByText(/drop an \.xls or \.xlsx file here/i));
    await waitFor(() => {
      expect(open).toHaveBeenCalledWith({
        multiple: false,
        filters: [{ name: "Excel", extensions: ["xls", "xlsx"] }],
      });
    });
    await waitFor(() => {
      expect(screen.getByText("sdtm.xls")).toBeInTheDocument();
    });
  });
});

describe("ImportTerminologyPage — drop validation", () => {
  it("rejects a .pdf drop with a flash hint and no state change", async () => {
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    const zone = screen.getByText(/drop an \.xls or \.xlsx file here/i);
    const file = new File(["pdf"], "report.pdf", { type: "application/pdf" });
    fireEvent.drop(zone, { dataTransfer: { files: [file] } });
    await waitFor(() => {
      expect(screen.getByText(/only \.xls or \.xlsx files are supported/i)).toBeInTheDocument();
    });
    expect(screen.queryByText("report.pdf")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /submit/i })).toBeDisabled();
  });

  it("accepts a .xlsx drop and shows the basename", async () => {
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    const zone = screen.getByText(/drop an \.xls or \.xlsx file here/i);
    const file = new File(["x"], "sdtm.xlsx", { type: "" });
    fireEvent.drop(zone, { dataTransfer: { files: [file] } });
    await waitFor(() => {
      expect(screen.getByText("sdtm.xlsx")).toBeInTheDocument();
    });
  });
});
```

(Add `import { fireEvent } from "@testing-library/react";` to the imports.)

Append these additional tests inside the same file:

```tsx
describe("ImportTerminologyPage — submit", () => {
  it("invokes import_terminology, hides the form, then shows the success Snackbar on resolve", async () => {
    (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue("/tmp/sdtm.xls");
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    await userEvent.click(screen.getByText(/drop an \.xls or \.xlsx file here/i));
    await screen.findByText("sdtm.xls");
    await userEvent.click(screen.getByRole("button", { name: /submit/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("import_terminology", {
        kind: "sdtm",
        filepath: "/tmp/sdtm.xls",
      });
    });
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent(/imported terminology version 2026-03-27/i);
    });
  });

  it("switches kind before submit so the API call uses the user’s final choice", async () => {
    (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue("/tmp/adam.xls");
    await renderPage({ initialEntries: ["/terminology/import?kind=sdtm"] });
    await userEvent.click(screen.getByText(/drop an \.xls or \.xlsx file here/i));
    await screen.findByText("adam.xls");
    await userEvent.click(screen.getByRole("button", { name: "ADaM" }));
    await userEvent.click(screen.getByRole("button", { name: /submit/i }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("import_terminology", {
        kind: "adam",
        filepath: "/tmp/adam.xls",
      });
    });
  });

  it("shows the error Snackbar when the API rejects with Http 409", async () => {
    (open as unknown as ReturnType<typeof vi.fn>).mockResolvedValue("/tmp/sdtm.xls");
    await renderPage({
      initialEntries: ["/terminology/import?kind=sdtm"],
      mockImport: () => {
        throw { kind: "http", status: 409, code: "duplicate", message: "exists" };
      },
    });
    await userEvent.click(screen.getByText(/drop an \.xls or \.xlsx file here/i));
    await screen.findByText("sdtm.xls");
    await userEvent.click(screen.getByRole("button", { name: /submit/i }));
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent(/import failed/i);
    });
  });
});
```

- [ ] **Step 2: Run the tests and confirm they all fail**

Run: `cd d:/project/aegis/apps/desktop/aegis-desktop && pnpm test -- --run src/test/features/terminology/import-terminology-page.test.tsx`
Expected: FAIL — the file `ImportTerminologyPage.tsx` does not exist.

- [ ] **Step 3: Implement `ImportTerminologyPage`**

Create `apps/desktop/aegis-desktop/src/features/terminology/pages/ImportTerminologyPage.tsx` with the implementation below. The page reads `kind` from `Route.useSearch()` (so the route file from Task 7 must already be importing the page from the barrel — if not, fall back to reading `window.location.search`).

```tsx
import { useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
  IconButton,
  Snackbar,
  ToggleButton,
  ToggleButtonGroup,
  Tooltip,
  Typography,
} from "@aegis/ui/mui";
import { ArrowBack as ArrowBackIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { api, errorMessage } from "../../../shared/api";
import type { ApiError, TerminologyKind, TerminologyVersionView } from "../../../shared/api";
import { queryKeys } from "../../../shared/query";

type Kind = TerminologyKind | null;

export function ImportTerminologyPage() {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const { t } = useI18n();
  const search = useSearch({ strict: false }) as { kind?: TerminologyKind };

  const [kind, setKind] = useState<Kind>(search.kind ?? null);
  const [filepath, setFilepath] = useState<string | null>(null);
  const [dropError, setDropError] = useState(false);
  const [snackbar, setSnackbar] = useState<{
    open: boolean;
    severity: "success" | "error";
    message: string;
  }>({ open: false, severity: "success", message: "" });

  const importMutation = useMutation<
    TerminologyVersionView,
    ApiError,
    { kind: TerminologyKind; filepath: string }
  >({
    mutationFn: ({ kind, filepath }) => api.importTerminology(kind, filepath),
    onSuccess: (version) => {
      qc.invalidateQueries({ queryKey: queryKeys.terminology.versions() });
      setSnackbar({
        open: true,
        severity: "success",
        message: t("terminology.import.success", { name: version.name }),
      });
    },
    onError: (err) => {
      setSnackbar({
        open: true,
        severity: "error",
        message: t("terminology.import.failure", {
          message: errorMessage(err),
        }),
      });
    },
  });

  const backLink = kind === null ? "/terminology/sdtm" : `/terminology/${kind}`;

  async function pickFile() {
    const path = await open({
      multiple: false,
      filters: [{ name: "Excel", extensions: ["xls", "xlsx"] }],
    });
    if (typeof path === "string") setFilepath(path);
  }

  function onDrop(e: React.DragEvent) {
    e.preventDefault();
    const file = e.dataTransfer.files[0];
    if (!file) return;
    const lower = file.name.toLowerCase();
    if (!lower.endsWith(".xls") && !lower.endsWith(".xlsx")) {
      setDropError(true);
      window.setTimeout(() => setDropError(false), 1500);
      return;
    }
    // Tauri's webview gives us the path via `path` on the File when running
    // inside the desktop app. In dev mode the field is empty so fall back to
    // just the name — the click-picker covers the realistic happy path.
    setFilepath((file as unknown as { path?: string }).path ?? file.name);
  }

  const canSubmit = kind !== null && filepath !== null && !importMutation.isPending;
  const fileName = filepath ? filepath.replace(/^.*[\\/]/, "") : null;

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 3 }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1 }}>
        <Tooltip title={t("common.back")}>
          <span>
            <IconButton
              onClick={() => navigate({ to: backLink })}
              aria-label={t("common.back")}
            >
              <ArrowBackIcon />
            </IconButton>
          </span>
        </Tooltip>
        <Typography variant="h5">{t("terminology.import.title")}</Typography>
      </Box>

      {importMutation.isPending ? (
        <Box
          sx={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: 2,
            py: 8,
          }}
        >
          <CircularProgress />
          <Typography>{t("terminology.import.importing")}</Typography>
        </Box>
      ) : (
        <>
          <ToggleButtonGroup
            exclusive
            value={kind}
            onChange={(_, v) => setKind(v)}
          >
            <ToggleButton value="sdtm">SDTM</ToggleButton>
            <ToggleButton value="adam">ADaM</ToggleButton>
          </ToggleButtonGroup>

          {filepath === null ? (
            <Box
              role="button"
              tabIndex={0}
              onClick={pickFile}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") pickFile();
              }}
              onDragOver={(e) => e.preventDefault()}
              onDrop={onDrop}
              sx={(theme) => ({
                p: 4,
                border: "2px dashed",
                borderColor: dropError
                  ? theme.palette.error.main
                  : theme.palette.divider,
                borderRadius: 1,
                textAlign: "center",
                cursor: "pointer",
              }) }}
            >
              <Typography>
                {dropError
                  ? t("terminology.import.fileTypeHint")
                  : t("terminology.import.dropZone")}
              </Typography>
            </Box>
          ) : (
            <Chip
              label={fileName}
              onDelete={() => setFilepath(null)}
              sx={{ alignSelf: "flex-start" }}
            />
          )}

          <Button
            variant="contained"
            disabled={!canSubmit}
            onClick={() =>
              importMutation.mutate({ kind: kind!, filepath: filepath! })
            }
          >
            {t("common.submit")}
          </Button>
        </>
      )}

      <Snackbar
        open={snackbar.open}
        autoHideDuration={4000}
        onClose={() => setSnackbar((s) => ({ ...s, open: false }))}
      >
        <Alert
          severity={snackbar.severity}
          onClose={() => setSnackbar((s) => ({ ...s, open: false }))}
        >
          {snackbar.message}
        </Alert>
      </Snackbar>
    </Box>
  );
}
```

- [ ] **Step 4: Run the tests and confirm they pass**

Run: `cd d:/project/aegis/apps/desktop/aegis-desktop && pnpm test -- --run src/test/features/terminology/import-terminology-page.test.tsx`
Expected: all 8 tests PASS.

If `useSearch` from `@tanstack/react-router` complains about a missing route context in the test (it expects a typed parent route), switch the test renderer to `renderWithFullRouter` or pass `strict: false` (already done above). If the dialog plugin import throws because the mock path is wrong, verify the `vi.mock("@tauri-apps/plugin-dialog", ...)` call is placed after the imports per Vitest hoisting rules.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/pages/ImportTerminologyPage.tsx apps/desktop/aegis-desktop/src/test/features/terminology/import-terminology-page.test.tsx
git commit -m "feat(terminology): add ImportTerminologyPage with TDD"
```

---

## Task 7: Update `ImportButton`, `TerminologyPage`, barrel, and add the route file

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/components/ImportButton.tsx` (replace Snackbar with `useNavigate`)
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx` (pass `kind` prop)
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/pages/index.ts` (re-export `ImportTerminologyPage`)
- Create: `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/import.tsx` (route file)
- Create: `apps/desktop/aegis-desktop/src/test/features/terminology/import-button.test.tsx`

**Interfaces:**
- Produces: `ImportButton({ kind }: { kind: TerminologyKind })` navigates to `/terminology/import?kind={kind}`. `pages/index.ts` re-exports `ImportTerminologyPage`. The new route file registers the import page.

- [ ] **Step 1: Write the failing `ImportButton` test**

Create `apps/desktop/aegis-desktop/src/test/features/terminology/import-button.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { ImportButton } from "../../../features/terminology/components/ImportButton";
import { renderInRouter } from "../../helpers/file-route-utils";

afterEach(() => cleanup());

function renderBtn() {
  return renderInRouter(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <ImportButton kind="sdtm" />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("ImportButton", () => {
  it("navigates to /terminology/import?kind=sdtm on click", async () => {
    const { router } = await renderBtn();
    await userEvent.click(screen.getByRole("button", { name: /import terminology/i }));
    expect(router.state.location.pathname).toBe("/terminology/import");
    expect(router.state.location.search).toEqual({ kind: "sdtm" });
  });
});
```

- [ ] **Step 2: Run the test and confirm it fails**

Run: `cd d:/project/aegis/apps/desktop/aegis-desktop && pnpm test -- --run src/test/features/terminology/import-button.test.tsx`
Expected: FAIL — the test calls `useNavigate` which fires, but `ImportButton` still renders the placeholder `IconButton` with a "coming soon" Snackbar, so the `name: /import terminology/i` query still matches. The test will fail differently: either `navigate` is not called, or `router.state.location.pathname` stays at `/`.

- [ ] **Step 3: Update `ImportButton`**

In `apps/desktop/aegis-desktop/src/features/terminology/components/ImportButton.tsx`, replace the entire file body with:

```tsx
import { useNavigate } from "@tanstack/react-router";
import { IconButton, Tooltip } from "@aegis/ui/mui";
import { Add as AddIcon } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { TerminologyKind } from "../../../shared/api";

export function ImportButton({ kind }: { kind: TerminologyKind }) {
  const navigate = useNavigate();
  const { t } = useI18n();
  return (
    <Tooltip title={t("terminology.import.title")}>
      <IconButton
        aria-label={t("terminology.import.title")}
        onClick={() => navigate({ to: "/terminology/import", search: { kind } })}
      >
        <AddIcon />
      </IconButton>
    </Tooltip>
  );
}
```

- [ ] **Step 4: Update `TerminologyPage` to pass the `kind` prop**

In `apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx`, find the existing `<ImportButton />` self-closing tag and replace it with `<ImportButton kind={kind} />`.

- [ ] **Step 5: Re-export the page from the barrel**

In `apps/desktop/aegis-desktop/src/features/terminology/pages/index.ts`, append:

```ts
export * from "./ImportTerminologyPage";
```

- [ ] **Step 6: Add the route file**

Create `apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/import.tsx`:

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { z } from "zod";

import { ImportTerminologyPage } from "../../../../../features/terminology";

const kindSchema = z.object({
  kind: z.enum(["sdtm", "adam"]).optional(),
});

export const Route = createFileRoute(
  "/_authed/_layout/terminology/import",
)({
  validateSearch: kindSchema,
  component: () => <ImportTerminologyPage />,
});
```

- [ ] **Step 7: Run the full test suite to verify no regressions**

Run: `cd d:/project/aegis/apps/desktop/aegis-desktop && pnpm test -- --run`
Expected: all tests pass. If the route file is not picked up by the router-plugin (so `Route` is missing), run `pnpm build` (or `pnpm dev`) once to regenerate `routes/routeTree.gen.ts`.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/terminology/components/ImportButton.tsx apps/desktop/aegis-desktop/src/features/terminology/pages/TerminologyPage.tsx apps/desktop/aegis-desktop/src/features/terminology/pages/index.ts apps/desktop/aegis-desktop/src/routes/_authed/_layout/terminology/import.tsx apps/desktop/aegis-desktop/src/test/features/terminology/import-button.test.tsx
git commit -m "feat(terminology): wire ImportButton to ImportTerminology page and add route"
```

---

## Self-Review

After writing the plan I verified:

1. **Spec coverage** — every numbered goal in the spec is covered by a task:
   - Goal 1 (replace placeholder `ImportButton`) → Task 7.
   - Goal 2 (route at `/_authed/_layout/terminology/import` with `?kind=` query) → Task 7 (route file).
   - Goal 3 (render ButtonGroup / drop zone / submit) → Task 6.
   - Goal 4 (Tauri command orchestrating version + codelists + code-items) → Task 3.
   - Goal 5 (use `batch_create_code_items`) → Task 2 + Task 3.
   - Goal 6 (invalidate terminology versions on success) → Task 6 (`onSuccess` invalidates).
   - Goal 7 (Spinner + Snackbar + back arrow stays enabled) → Task 6.
   - Goal 8 (out-of-scope items) → not implemented.

2. **Placeholder scan** — searched for "TBD", "TODO", "fill in", etc. — none found. Every step has actual code.

3. **Type consistency** — `BatchCodeItemEntry`, `BatchCreateCodeItemsRequest`, `BatchCreateCodeItemsResponse`, `importTerminology`, `api.importTerminology` are defined in Task 2 and Task 4 and consumed in Tasks 3, 6, and 7 with matching names. The `useSearch({ strict: false })` in Task 6 matches the search schema in Task 7's route file.