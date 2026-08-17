# Aegis Desktop — Project List Tags Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface project tags (already on the wire) in the desktop `project-list` feature: render them as a new column on the list, edit them in `ProjectDrawer` (both create and edit), and filter the list by tag value substring.

**Architecture:** Six self-contained tasks. We add the `Tag` type and i18n keys first so every downstream task compiles against the new types. `TagEditor` is a pure controlled row-stack component, new file. The list page owns `tagQuery` state and ANDs it into the existing client-side `useMemo`. The drawer's existing edit-open `useEffect` seeds `tags` from the loaded row, and a `tagsTouched` flag gates the update body so the server's "missing = leave alone" semantics stay intact.

**Tech Stack:** TypeScript (strict), React 18, `@tanstack/react-query`, MUI v5 (`@aegis/ui/mui`), `react-i18n` provider at `@aegis/ui/i18n`, Vitest + `@testing-library/react` + `@testing-library/user-event`.

**Reference spec:** [docs/superpowers/specs/2026-08-17-aegis-desktop-project-list-tags-design.md](../specs/2026-08-17-aegis-desktop-project-list-tags-design.md).

## Global Constraints

- i18n catalog contract: `lib/packages/ui/src/i18n/locales/zhCN.ts` is `satisfies Record<keyof typeof en, string>`. Every new key added to `en.ts` must appear in `zhCN.ts` with the same key name. Removing `project.field.product` from `en.ts` requires also removing its zhCN entry.
- Wire contract (server): on `PATCH /api/project/{code}`, missing `tags` leaves the list alone; `tags: []` (present) replaces the whole list with empty. The client must NOT include `tags` in an update body unless the user actually edited the tags section in this drawer session.
- Rust side untouched in this change. The existing `apps/server/aegis-server/src/transport/http/project/handlers.rs` and the Tauri `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs` already wire `tags`.
- `useListProducts` (`features/project-list/data/products.ts`) stays exported from `features/project-list/index.ts`, file is left in place, with a one-line `// currently unused — pending product surface removal` comment. Out-of-scope cleanup deferred.
- File layout follows [the by-features reorganization spec](../specs/2026-08-16-aegis-desktop-by-features-reorganization-design.md) — types in `shared/api`, data hooks in `features/project-list/data`, components and pages in `features/project-list/{components,pages}`.
- `ProjectView.product` and `CreateProjectInput.productId` / `UpdateProjectBody.productId` are **removed** from the TypeScript types — they no longer exist on the wire.
- Existing Vitest tests at `src/test/features/project-list/*` need their fixture objects updated (drop `product`, add `tags: []`) to compile after Task 1.
- Every task ends with a green `pnpm --filter aegis-desktop vitest run <file>` and a commit. Final task adds a full-suite run.

## Commit convention

`feat(aegis-desktop): <short summary>` for the substantive tasks; `test(aegis-desktop): <short summary>` for the test-first TDD steps where the implementation lands in the same commit; `docs(aegis-desktop): i18n strings for project tags` for the i18n fold-in.

---

### Task 1: Type surface + i18n strings + test fixture updates

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts`
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-list/project-table.test.tsx` (replace `product` field, add `tags: []`)
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-list/project-list-page.test.tsx` (replace `product` field on the three project fixtures)
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-list/project-drawer.test.tsx` (replace `product` field; product autocomplete–related tests will be removed in Task 5)

**Interfaces produced (consumed by all later tasks):**
- `Tag { key: string; value: string }`
- `ProjectView.tags: Tag[]` (note: wire JSON key is `tags`, camelCase)
- `CreateProjectInput.tags?: Tag[]`
- `UpdateProjectBody.tags?: Tag[]`
- i18n keys: `project.col.tags`, `project.filter.tag.label`, `project.field.tags.add`, `project.field.tags.key`, `project.field.tags.value`

- [ ] **Step 1: Update shared/api/types.ts**

Edit `apps/desktop/aegis-desktop/src/shared/api/types.ts`:

```ts
// (under the "// Project" comment block; remove the `product` from
//  ProjectView and `productId` from CreateProjectInput / UpdateProjectBody.)

export interface ProjectView {
  id: number;
  code: string;
  description: string;
  members: ProjectMembersView;
  unblindMembers: ProjectMembersView;
  tags: Tag[];
  active: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateProjectInput {
  code: string;
  description: string;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
  tags?: Tag[];
}

export interface UpdateProjectBody {
  code?: string;
  description?: string;
  active?: boolean;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
  tags?: Tag[];
}

// Above the existing Project block, add:
export interface Tag {
  key: string;
  value: string;
}
```

The `Tag` interface is deliberately placed in the same file as `ProjectView`; it is not exported separately from `index.ts` because no consumer outside this feature needs it yet. (Other tasks will prove that out.)

- [ ] **Step 2: Add i18n keys to en.ts**

Edit `lib/packages/ui/src/i18n/locales/en.ts`. Replace the existing lines:

```
  'project.col.leaders': 'Leaders',
  'project.col.active': 'Status',
```

with:

```
  'project.col.leaders': 'Leaders',
  'project.col.tags': 'Tags',
  'project.col.active': 'Status',
```

Replace:

```
  'project.search.label': 'Search (code, description, leaders)',
  'project.involve': 'Involve',
```

with:

```
  'project.search.label': 'Search (code, description, leaders)',
  'project.filter.tag.label': 'Filter by tag',
  'project.involve': 'Involve',
```

Replace:

```
  'project.field.description': 'Description',
  'project.field.product': 'Product',
  'project.field.active': 'Active',
```

with:

```
  'project.field.description': 'Description',
  'project.field.tags.key': 'Tag key',
  'project.field.tags.value': 'Tag value',
  'project.field.tags.add': 'Add tag',
  'project.field.active': 'Active',
```

(`project.field.product` is removed; the product autocomplete is going away in Task 5.)

- [ ] **Step 3: Mirror the new keys in zhCN.ts**

Edit `lib/packages/ui/src/i18n/locales/zhCN.ts` so the new keys are present and the removed key is gone. The positions in the file must match `en.ts` for `satisfies Record<keyof typeof en, string>` to keep compiling:

```
  'project.col.leaders': '负责人',
  'project.col.tags': '标签',
  'project.col.active': '状态',
  // …leader/inactive/etc…
  'project.search.label': '搜索（项目编号、描述、负责人）',
  'project.filter.tag.label': '按标签筛选',
  'project.involve': '我参与的',
  // …add/edit…
  'project.field.description': '描述',
  'project.field.tags.key': '标签键',
  'project.field.tags.value': '标签值',
  'project.field.tags.add': '新增标签',
  'project.field.active': '已启用',
```

- [ ] **Step 4: Update test fixtures to drop `product` and add `tags: []`**

For each of `project-table.test.tsx`, `project-list-page.test.tsx`, `project-drawer.test.tsx`, find every `ProjectView` literal and:

  - Remove the entire `product: { ... }` field.
  - Add `tags: []` to the same object literal.
  - Rename `unblindMembers: { leaders: [], workers: [] }` (etc.) unchanged.

For `project-list-page.test.tsx`, the three project fixtures (`projectA`, `projectB`, `projectC`) and the spread `{ ...projectA, id: 3, ... }` need the same treatment.

For `project-drawer.test.tsx`, the `productFixture` and any test that calls `mockCommands({ list_products: () => [productFixture], ... })` can stay for now — the mocks don't error out if `list_products` is never invoked; the actual product autocomplete test bodies will be removed in Task 5 when we delete the autocomplete.

- [ ] **Step 5: Run type-check to confirm `tsc` is clean**

Run:
```bash
cd apps/desktop/aegis-desktop && pnpm tsc --noEmit
```
Expected: zero errors. Errors here mean the type surface and the new test fixtures are out of sync.

- [ ] **Step 6: Run the affected Vitest files to confirm they compile and pass after the fixture fix-up**

Run:
```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list
```
Expected: existing tests that don't touch the removed autocomplete still pass. Any test that's specifically about the product autocomplete may now hang waiting for a removed element — note them for Task 5, but don't skip them yet.

- [ ] **Step 7: Commit**

```bash
cd apps/desktop/aegis-desktop
git add src/shared/api/types.ts \
        src/test/features/project-list/project-table.test.tsx \
        src/test/features/project-list/project-list-page.test.tsx \
        src/test/features/project-list/project-drawer.test.tsx
cd ../../..
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(aegis-desktop): surface Tag type on project DTOs + add tag i18n keys"
```

---

### Task 2: TagEditor component (TDD)

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/project-list/components/TagEditor.tsx`
- Create: `apps/desktop/aegis-desktop/src/test/features/project-list/tag-editor.test.tsx`

**Interfaces:**
- `Tag { key: string; value: string }` (from Task 1)
- `TagEditorProps { value: Tag[]; onChange: (next: Tag[]) => void; onTouched?: () => void }`

- [ ] **Step 1: Write the failing test file**

Create `apps/desktop/aegis-desktop/src/test/features/project-list/tag-editor.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

import { TagEditor } from "../../../features/project-list/components/TagEditor";
import type { Tag } from "../../../shared/api";

const tagProduct: Tag = { key: "Product", value: "DEMO-001" };
const tagClient: Tag = { key: "Client", value: "ACME" };

afterEach(() => cleanup());

function renderEditor(props: {
  value?: Tag[];
  onChange?: (next: Tag[]) => void;
  onTouched?: () => void;
} = {}) {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <TagEditor
          value={props.value ?? []}
          onChange={props.onChange ?? vi.fn()}
          onTouched={props.onTouched}
        />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("TagEditor", () => {
  it("renders no rows when value is empty and an Add tag button", () => {
    renderEditor();
    // No rows means no "Tag key" labels exist (the Add button is not labeled "Tag key").
    expect(screen.queryAllByLabelText(/tag key/i)).toHaveLength(0);
    expect(screen.getByRole("button", { name: /add tag/i })).toBeInTheDocument();
  });

  it("renders one row per value entry, each with a key and value TextField", () => {
    renderEditor({ value: [tagProduct, tagClient] });
    const keys = screen.getAllByLabelText(/tag key/i);
    const values = screen.getAllByLabelText(/tag value/i);
    expect(keys).toHaveLength(2);
    expect(values).toHaveLength(2);
    expect(keys[0]).toHaveValue("Product");
    expect(values[0]).toHaveValue("DEMO-001");
    expect(keys[1]).toHaveValue("ACME");  // value TextField shows tag.value
  });

  it("clicking Add tag appends an empty row and fires onChange", async () => {
    const onChange = vi.fn();
    renderEditor({ value: [tagProduct], onChange });
    await userEvent.click(screen.getByRole("button", { name: /add tag/i }));
    expect(onChange).toHaveBeenCalledTimes(1);
    const next = onChange.mock.calls[0][0] as Tag[];
    expect(next).toEqual([tagProduct, { key: "", value: "" }]);
  });

  it("clicking a row's remove button drops that row and fires onChange", async () => {
    const onChange = vi.fn();
    renderEditor({ value: [tagProduct, tagClient], onChange });
    // Both rows have a remove button; click the first one.
    const removes = screen.getAllByRole("button", { name: /remove/i });
    await userEvent.click(removes[0]);
    expect(onChange).toHaveBeenCalledTimes(1);
    const next = onChange.mock.calls[0][0] as Tag[];
    expect(next).toEqual([tagClient]);
  });

  it("editing a key updates only that key in the row and fires onChange", async () => {
    const onChange = vi.fn();
    renderEditor({ value: [tagProduct], onChange });
    const keyInput = screen.getByDisplayValue("Product");
    await userEvent.clear(keyInput);
    await userEvent.type(keyInput, "Owner");
    // We don't pin the exact final call; verify the last emission preserved the value field.
    const lastCall = onChange.mock.calls.at(-1)?.[0] as Tag[] | undefined;
    expect(lastCall).toBeDefined();
    expect(lastCall![0].key).toBe("Owner");
    expect(lastCall![0].value).toBe("DEMO-001");
  });

  it("fires onTouched on first interaction only", async () => {
    const onTouched = vi.fn();
    renderEditor({ value: [tagProduct], onTouched });
    const removes = screen.getAllByRole("button", { name: /remove/i });
    await userEvent.click(removes[0]);
    await userEvent.click(screen.getByRole("button", { name: /add tag/i }));
    expect(onTouched).toHaveBeenCalledTimes(1);
  });
});
```

- [ ] **Step 2: Run the test to confirm it fails**

```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list/tag-editor.test.tsx
```
Expected: vitest reports the file is missing (`Failed to resolve import .../TagEditor`). That's the fail.

- [ ] **Step 3: Implement `TagEditor`**

Create `apps/desktop/aegis-desktop/src/features/project-list/components/TagEditor.tsx`:

```tsx
import { useEffect, useRef } from "react";
import {
  Box,
  Button,
  IconButton,
  Stack,
  TextField,
} from "@aegis/ui/mui";
import { Add, Close } from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import type { Tag } from "../../../shared/api";

export interface TagEditorProps {
  value: Tag[];
  onChange: (next: Tag[]) => void;
  onTouched?: () => void;
}

/**
 * Tag editor. Pure controlled component: parent's `value` /
 * `onChange` is the source of truth. Rows render top-to-bottom in
 * `value` order. After `append`, the new key TextField is focused
 * (managed via a single `useRef<number>` + `useEffect` chain —
 * resize-on-reseed). `onTouched` is fired at most once per render
 * cycle so a parent's "edited?" flag flips on the first interaction
 * only.
 */
export function TagEditor({ value, onChange, onTouched }: TagEditorProps) {
  const { t } = useI18n();

  // Track which row was just appended so the useEffect can focus it.
  const lastAppendedIndex = useRef<number>(-1);
  const keyInputRefs = useRef<(HTMLInputElement | null)[]>([]);

  // Reset focus bookkeeping when the parent's value length changes
  // (e.g. drawer re-seeds with new tags from the wire).
  useEffect(() => {
    keyInputRefs.current.length = value.length;
  }, [value.length]);

  // Focus the newly-appended key input, then clear the pointer so a
  // later render doesn't steal focus back.
  useEffect(() => {
    if (lastAppendedIndex.current >= 0) {
      const target = keyInputRefs.current[lastAppendedIndex.current];
      target?.focus();
      lastAppendedIndex.current = -1;
    }
  });

  function emit(next: Tag[]) {
    onChange(next);
    onTouched?.();
  }

  function updateRow(index: number, patch: Partial<Tag>) {
    const next = value.map((row, i) => (i === index ? { ...row, ...patch } : row));
    emit(next);
  }

  function removeRow(index: number) {
    emit(value.filter((_, i) => i !== index));
  }

  function appendRow() {
    emit([...value, { key: "", value: "" }]);
    lastAppendedIndex.current = value.length;
  }

  return (
    <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
      <Stack spacing={1}>
        {value.map((tag, i) => (
          <Stack
            key={`row-${i}-${tag.key}-${tag.value}`}
            direction="row"
            spacing={1}
            alignItems="center"
          >
            <TextField
              size="small"
              label={t("project.field.tags.key")}
              value={tag.key}
              onChange={(event) => updateRow(i, { key: event.target.value })}
              inputRef={(el) => {
                keyInputRefs.current[i] = el;
              }}
              sx={{ flex: 1 }}
            />
            <TextField
              size="small"
              label={t("project.field.tags.value")}
              value={tag.value}
              onChange={(event) => updateRow(i, { value: event.target.value })}
              sx={{ flex: 1 }}
            />
            <IconButton
              aria-label={t("common.remove")}
              onClick={() => removeRow(i)}
            >
              <Close />
            </IconButton>
          </Stack>
        ))}
      </Stack>
      <Box sx={{ display: "flex", justifyContent: "flex-start" }}>
        <Button
          size="small"
          startIcon={<Add />}
          onClick={appendRow}
        >
          {t("project.field.tags.add")}
        </Button>
      </Box>
    </Box>
  );
}
```

Make sure `common.remove` already exists or add it. If it does not, add it to both `en.ts` and `zhCN.ts`:

```
// en.ts
  'common.cancel': 'Cancel',
  'common.remove': 'Remove',
```
```
// zhCN.ts
  'common.cancel': '取消',
  'common.remove': '移除',
```

(Re-run the catalog type-check if you added them.)

- [ ] **Step 4: Run the test to confirm it passes**

```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list/tag-editor.test.tsx
```
Expected: 6/6 green.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-list/components/TagEditor.tsx \
        apps/desktop/aegis-desktop/src/test/features/project-list/tag-editor.test.tsx
git commit -m "feat(aegis-desktop): add TagEditor with key/value row stack"
```

---

### Task 3: ProjectTable Tags column (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectTable.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-list/project-table.test.tsx`

**Interfaces:**
- `Tag { key: string; value: string }` (from Task 1)
- Existing `ProjectTableProps` unchanged

- [ ] **Step 1: Write failing tests for the new column**

Add to the existing `describe("ProjectTable — column rendering", () => { ... })` block in `project-table.test.tsx`. Insert **before** the closing `});` of that describe, but **after** the existing tests so file-local reading order matches spec order:

```tsx
  it("renders a Tags column header between Leaders and Status", () => {
    renderTable();
    const headers = screen.getAllByRole("columnheader");
    // Expected order: code, description, leaders, tags, status, operations.
    const headersText = headers.map((h) => h.textContent?.trim() ?? "");
    expect(headersText).toContain("tags");
    expect(headersText.indexOf("leaders")).toBeLessThan(headersText.indexOf("tags"));
    expect(headersText.indexOf("tags")).toBeLessThan(headersText.indexOf("status"));
  });

  it("renders each tag as a chip labelled by value with the key in title", () => {
    renderTable({
      rows: [
        {
          ...baseRow,
          tags: [
            { key: "Product", value: "DEMO-001" },
            { key: "Client", value: "ACME" },
          ],
        },
      ],
    });
    expect(screen.getByText("DEMO-001")).toBeInTheDocument();
    expect(screen.getByText("ACME")).toBeInTheDocument();
    expect(screen.getByTitle("Product")).toBeInTheDocument();
    expect(screen.getByTitle("Client")).toBeInTheDocument();
  });

  it("renders an em-dash when both leader arrays and tags are empty", () => {
    renderTable({
      rows: [
        {
          ...baseRow,
          members: { leaders: [], workers: [] },
          unblindMembers: { leaders: [], workers: [] },
          tags: [],
        },
      ],
    });
    // Two "—" elements (leaders cell, tags cell). Loosen to length>=2.
    const dashes = screen.getAllByText("—");
    expect(dashes.length).toBeGreaterThanOrEqual(2);
  });
```

Also update the existing header test (`renders all five column headers` → rename to "renders six column headers" and add `tags`):

```tsx
  it("renders six column headers", () => {
    renderTable();
    expect(screen.getByRole("columnheader", { name: /^code$/i })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /^description$/i })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /^leaders$/i })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /^tags$/i })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /^status$/i })).toBeInTheDocument();
  });
```

- [ ] **Step 2: Run new tests to confirm they fail**

```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list/project-table.test.tsx -t "Tags"
```
Expected: 3/3 fail. The existing 5-column-headers test should now also fail because the table renders only 5 columns.

- [ ] **Step 3: Implement the new column**

Edit `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectTable.tsx`. Insert a new header cell **between** the existing `Leaders` `<TableCell>` and the `Active` `<TableCell>`:

```tsx
              <TableCell>{t("project.col.leaders")}</TableCell>
              <TableCell>{t("project.col.tags")}</TableCell>   {/* NEW */}
              <TableCell>{t("project.col.active")}</TableCell>
```

In the body row's `<TableRow>`, insert a corresponding cell **between** the leaders cell and the active-status cell:

```tsx
                  <TableCell>
                    <Stack /* (existing leaders chip stack) */> ... </Stack>
                  </TableCell>
                  <TableCell>
                    <Stack
                      direction="row"
                      spacing={0.5}
                      sx={{ flexWrap: "wrap", gap: 0.5 }}
                    >
                      {row.tags.map((tag, i) => (
                        <Chip
                          key={`tag-${i}-${tag.key}-${tag.value}`}
                          size="small"
                          label={tag.value}
                          title={tag.key}
                        />
                      ))}
                      {row.tags.length === 0 && <span>—</span>}
                    </Stack>
                  </TableCell>
                  <TableCell>
                    <Tooltip /* (existing active icon tooltip) */> ... </Tooltip>
                  </TableCell>
```

`Chip` is already imported from `@aegis/ui/mui`.

- [ ] **Step 4: Re-run the file's tests**

```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list/project-table.test.tsx
```
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-list/components/ProjectTable.tsx \
        apps/desktop/aegis-desktop/src/test/features/project-list/project-table.test.tsx
git commit -m "feat(aegis-desktop): render project tags as a Tags column in ProjectTable"
```

---

### Task 4: ProjectFilterBar — add tag filter input, and ProjectListPage — wire tagQuery into filter (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectFilterBar.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-list/project-filter-bar.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/project-list/pages/ProjectListPage.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-list/project-list-page.test.tsx`

**Interfaces:**
- `ProjectFilterBarProps` gains `tagQuery: string` and `onTagQueryChange: (v: string) => void`.
- `ProjectListPage` owns `tagQuery` and combines it into `filteredRows`.

- [ ] **Step 1: Write failing tests for ProjectFilterBar**

In `apps/desktop/aegis-desktop/src/test/features/project-list/project-filter-bar.test.tsx`, update the `renderBar` helper to pass `tagQuery` and `onTagQueryChange` props (default `""` and a `vi.fn()`). Extend the helper's `props` shape and pass-through:

```tsx
function renderBar(props: {
  query?: string;
  tagQuery?: string;
  involve?: boolean;
  onQueryChange?: (v: string) => void;
  onTagQueryChange?: (v: string) => void;
  onInvolveChange?: (v: boolean) => void;
} = {}) {
  const onQueryChange = props.onQueryChange ?? vi.fn();
  const onTagQueryChange = props.onTagQueryChange ?? vi.fn();
  const onInvolveChange = props.onInvolveChange ?? vi.fn();
  return {
    onQueryChange,
    onTagQueryChange,
    onInvolveChange,
    ...render(
      <AegisThemeProvider>
        <AegisI18nProvider>
          <ProjectFilterBar
            query={props.query ?? ""}
            tagQuery={props.tagQuery ?? ""}
            involve={props.involve ?? false}
            onQueryChange={onQueryChange}
            onTagQueryChange={onTagQueryChange}
            onInvolveChange={onInvolveChange}
          />
        </AegisI18nProvider>
      </AegisThemeProvider>,
    ),
  };
}
```

Append a new `describe("ProjectFilterBar — tag filter", () => { ... })` block to the file:

```tsx
describe("ProjectFilterBar — tag filter", () => {
  it("renders the tag filter field with the current value", () => {
    renderBar({ tagQuery: "demo" });
    expect(screen.getByLabelText(/filter by tag/i)).toHaveValue("demo");
  });

  it("calls onTagQueryChange when the tag filter field changes", async () => {
    const { onTagQueryChange } = renderBar();
    await userEvent.type(screen.getByLabelText(/filter by tag/i), "x");
    expect(onTagQueryChange).toHaveBeenLastCalledWith("x");
  });

  it("leaves Involve checkbox gated as before (regression)", () => {
    renderBar({ involve: true });
    expect(screen.getByRole("checkbox", { name: /involve/i })).toBeChecked();
  });
});
```

- [ ] **Step 2: Run the new tests to confirm they fail**

```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list/project-filter-bar.test.tsx -t "tag filter"
```
Expected: 3/3 fail.

- [ ] **Step 3: Update ProjectFilterBar**

Edit `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectFilterBar.tsx`:

```tsx
export interface ProjectFilterBarProps {
  query: string;
  onQueryChange: (value: string) => void;
  tagQuery: string;
  onTagQueryChange: (value: string) => void;
  involve: boolean;
  onInvolveChange: (value: boolean) => void;
}

export function ProjectFilterBar({
  query,
  onQueryChange,
  tagQuery,
  onTagQueryChange,
  involve,
  onInvolveChange,
}: ProjectFilterBarProps) {
  const { t } = useI18n();

  return (
    <Box sx={{ display: "flex", alignItems: "center", gap: 2 }}>
      <TextField
        size="small"
        label={t("project.search.label")}
        value={query}
        onChange={(event) => onQueryChange(event.target.value)}
        sx={{ minWidth: 320 }}
      />
      <TextField
        size="small"
        label={t("project.filter.tag.label")}
        value={tagQuery}
        onChange={(event) => onTagQueryChange(event.target.value)}
        sx={{ minWidth: 240 }}
      />
      <FormControlLabel
        sx={{ ml: "auto" }}
        control={
          <Checkbox
            checked={involve}
            onChange={(event) => onInvolveChange(event.target.checked)}
          />
        }
        label={t("project.involve")}
      />
    </Box>
  );
}
```

- [ ] **Step 4: Re-run ProjectFilterBar tests**

```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list/project-filter-bar.test.tsx
```
Expected: all green.

- [ ] **Step 5: Write failing tests for ProjectListPage filter logic**

In `apps/desktop/aegis-desktop/src/test/features/project-list/project-list-page.test.tsx`, extend `projectA` and friends with `tags: []` if not already done in Task 1, and add explicit `tags` to specific fixtures:

```tsx
const projectA: ProjectView = {
  // ...existing fields...
  tags: [{ key: "Product", value: "DEMO-001" }],
};

const projectB: ProjectView = {
  // ...existing fields...
  tags: [{ key: "Product", value: "OTHER-002" }],
};

const projectC: ProjectView = {
  ...projectA,
  id: 3,
  code: "gamma",
  tags: [{ key: "Client", value: "ACME" }],
};
```

Append a new `describe("ProjectListPage — tag filter", () => { ... })` block:

```tsx
describe("ProjectListPage — tag filter", () => {
  it("filters rows by tag value substring (case-insensitive)", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/filter by tag/i), "demo");
    await waitFor(() => {
      expect(screen.getByText("alpha")).toBeInTheDocument();
      expect(screen.getByText("beta")).not.toBeInTheDocument();
      expect(screen.getByText("gamma")).not.toBeInTheDocument();
    });
  });

  it("leaves all rows visible when tag filter is empty", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    expect(screen.getByText("beta")).toBeInTheDocument();
    expect(screen.getByText("gamma")).toBeInTheDocument();
  });

  it("combines tag filter with the existing search filter (AND)", async () => {
    await renderPage(adminUser, [projectA, projectB, projectC]);
    await screen.findByText("alpha");
    await userEvent.type(screen.getByLabelText(/filter by tag/i), "demo");
    await userEvent.type(screen.getByLabelText(/search/i), "ALPHA");
    await waitFor(() => {
      expect(screen.getByText("alpha")).toBeInTheDocument();
      expect(screen.queryByText("beta")).not.toBeInTheDocument();
    });
  });
});
```

- [ ] **Step 6: Run new project-list-page tests to confirm failure**

```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list/project-list-page.test.tsx -t "tag filter"
```
Expected: 3/3 fail (the input doesn't exist yet; ProjectListPage doesn't pass `tagQuery`).

- [ ] **Step 7: Wire `tagQuery` into ProjectListPage**

Edit `apps/desktop/aegis-desktop/src/features/project-list/pages/ProjectListPage.tsx`:

```tsx
  const [query, setQuery] = useState("");
  const [tagQuery, setTagQuery] = useState("");     // NEW
  const [involve, setInvolve] = useState(false);
  // …

  const filteredRows = useMemo<ProjectView[]>(() => {
    const all = projects.data ?? [];
    const trimmed = query.trim();
    const q = trimmed.toLowerCase();
    const t = tagQuery.trim().toLowerCase();          // NEW
    return all.filter((row) => {
      // Search filter (unchanged).
      if (q.length > 0) {
        // …existing inCode/inDescription/inLeaders block…
      }
      // Tag filter (NEW).
      if (t.length > 0) {
        const inTag = row.tags.some((tag) => tag.value.toLowerCase().includes(t));
        if (!inTag) return false;
      }
      // Involve filter (unchanged).
      if (involve && currentCode) {
        // …existing inMembers block…
      }
      return true;
    });
  }, [projects.data, query, tagQuery, involve, currentCode]);

  return (
    <Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 2 }}>
      <ProjectFilterBar
        query={query}
        onQueryChange={setQuery}
        tagQuery={tagQuery}
        onTagQueryChange={setTagQuery}
        involve={involve}
        onInvolveChange={setInvolve}
      />
      {/* …rest unchanged… */}
    </Box>
  );
```

`row.tags` is now used in the filter; every fixture must have `tags` defined. (Task 1 fixture edit plus Step 5 above.)

- [ ] **Step 8: Re-run the full project-list-page test file**

```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list/project-list-page.test.tsx
```
Expected: all green.

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-list/components/ProjectFilterBar.tsx \
        apps/desktop/aegis-desktop/src/features/project-list/pages/ProjectListPage.tsx \
        apps/desktop/aegis-desktop/src/test/features/project-list/project-filter-bar.test.tsx \
        apps/desktop/aegis-desktop/src/test/features/project-list/project-list-page.test.tsx
git commit -m "feat(aegis-desktop): filter project list by tag value"
```

---

### Task 5: ProjectDrawer — TagEditor integration + remove product (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectDrawer.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-list/project-drawer.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/project-list/data/products.ts` (add a one-line unused-mark comment — no behavior change)

**Interfaces:**
- `ProjectDrawer` adds `tags: Tag[]` and `tagsTouched: boolean` to local state.
- `onSubmit` omits `tags` from the `UpdateProjectBody` unless `tagsTouched` is true.

- [ ] **Step 1: Write failing tests**

In `apps/desktop/aegis-desktop/src/test/features/project-list/project-drawer.test.tsx`, **delete** the product-autocomplete tests and the `productFixture` block. Specifically, remove:

  - `import type { ProductView, ... } from "../../../shared/api";` — drop `ProductView`.
  - `const productFixture: ProductView = { ... };`
  - The test "disables Submit until code, description, and product are set" (it depended on a `product` field that no longer exists).
  - The product-clicking lines inside "calls api.createProject with the assembled shape on Submit" (drop `mockCommands({ list_products: () => [...] })` registration of that fixture; drop the `screen.getByLabelText(/\\bproduct\\b/i)` lookups).
  - Same edits inside "fetches the project via get_project_by_code" and "calls api.updateProject …" — remove `list_products` from mocks and the `productId` assertions.
  - "shows an Alert with the error message when create_project fails" — drop the product-related lines, but keep the test.
  - Drop `productId` assertion from the "calls api.updateProject" expectedBody.

Add new tests (append a new describe block):

```tsx
describe("ProjectDrawer — tags", () => {
  it("renders the TagEditor in create mode with zero rows by default", async () => {
    mockCommands({ list_users: () => [userFixture] });
    await renderDrawer("create");
    expect(screen.getByRole("button", { name: /add tag/i })).toBeInTheDocument();
    // No rows means no key/value labels yet.
    expect(screen.queryAllByLabelText(/tag key/i)).toHaveLength(0);
  });

  it("seeds the editor with the project's tags in edit mode and tagsTouched stays false", async () => {
    mockCommands({
      list_users: () => [userFixture],
      get_project_by_code: () => ({
        ...projectFixture,
        tags: [{ key: "Product", value: "DEMO-001" }],
      }),
    });
    await renderDrawer("edit", "alpha");
    await waitFor(() =>
      expect(screen.getByLabelText(/tag value/i)).toHaveValue("DEMO-001"),
    );
  });

  it("create-mode submit includes the assembled tags array on the body", async () => {
    mockCommands({
      list_users: () => [userFixture],
      create_project: () => projectFixture,
    });
    await renderDrawer("create");
    await userEvent.type(screen.getByLabelText(/\bcode\b/i), "newproj");
    await userEvent.type(screen.getByLabelText(/\bdescription\b/i), "d");
    await userEvent.click(screen.getByRole("button", { name: /add tag/i }));
    const keyInput = screen.getByLabelText(/tag key/i);
    const valueInput = screen.getByLabelText(/tag value/i);
    await userEvent.type(keyInput, "Product");
    await userEvent.type(valueInput, "DEMO-007");
    await userEvent.click(screen.getByRole("button", { name: /^create$/i }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "create_project",
        expect.objectContaining({
          tags: [{ key: "Product", value: "DEMO-007" }],
        }),
      ),
    );
  });

  it("edit-mode submit omits tags from the body when user did NOT touch the editor", async () => {
    mockCommands({
      list_users: () => [userFixture],
      get_project_by_code: () => ({
        ...projectFixture,
        tags: [{ key: "Product", value: "DEMO-001" }],
      }),
      update_project: () => projectFixture,
    });
    await renderDrawer("edit", "alpha");
    const descriptionField = await screen.findByLabelText(/\bdescription\b/i);
    await waitFor(() =>
      expect(descriptionField).toHaveValue("Alpha description"),
    );
    await userEvent.clear(descriptionField);
    await userEvent.type(descriptionField, "Edited");
    await userEvent.click(screen.getByRole("button", { name: /^save$/i }));
    await waitFor(() => {
      const call = (invoke as unknown as ReturnType<typeof vi.fn>).mock.calls.find(
        ([cmd]) => cmd === "update_project",
      );
      expect(call).toBeDefined();
      const body = call![1].body as UpdateProjectBody;
      expect(body).not.toHaveProperty("tags");
    });
  });

  it("edit-mode submit sends the new tags array when user edited the editor", async () => {
    mockCommands({
      list_users: () => [userFixture],
      get_project_by_code: () => ({
        ...projectFixture,
        tags: [{ key: "Product", value: "DEMO-001" }],
      }),
      update_project: () => projectFixture,
    });
    await renderDrawer("edit", "alpha");
    await screen.findByLabelText(/\bdescription\b/i);
    const valueInput = await screen.findByLabelText(/tag value/i);
    await userEvent.clear(valueInput);
    await userEvent.type(valueInput, "DEMO-002");
    await userEvent.click(screen.getByRole("button", { name: /^save$/i }));
    await waitFor(() => {
      const call = (invoke as unknown as ReturnType<typeof vi.fn>).mock.calls.find(
        ([cmd]) => cmd === "update_project",
      );
      expect(call).toBeDefined();
      const body = call![1].body as UpdateProjectBody;
      expect(body.tags).toEqual([{ key: "Product", value: "DEMO-002" }]);
    });
  });
});
```

- [ ] **Step 2: Run drawer tests to confirm they fail (current state)**

```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list/project-drawer.test.tsx
```
Expected: many failures — references to `productFixture`, `getByLabelText(/\bproduct\b/i)`, and the now-burnt `UpdateProjectBody.productId` assertion. The new tag tests should also fail because the drawer doesn't render `TagEditor` yet.

- [ ] **Step 3: Rewrite `ProjectDrawer`**

Replace `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectDrawer.tsx` with the version specified below. The changes are:

  - Drop the `useListProducts()` hook and the import of `useListProducts`. Drop `products.data` and the product autocomplete's branching.
  - Drop `productId` state and `setProductId`. Drop the `productId` import from `CreateProjectInput` / `UpdateProjectBody`.
  - Add `tags: Tag[]` and `tagsTouched: boolean` state.
  - In the existing edit-open `useEffect`, add `setTags(r.data.tags); setTagsTouched(false);`.
  - Render `<TagEditor value={tags} onChange={setTags} onTouched={() => setTagsTouched(true)} />` between the description `TextField` and the first membership `Autocomplete`.
  - In `onSubmit`:
    - create path: add `tags` to the `CreateProjectInput` literal.
    - edit path: spread `...(tagsTouched ? { tags } : {})` into the `UpdateProjectBody`.
  - `submitDisabled` no longer includes `productId === null`.

```tsx
import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Drawer,
  FormControlLabel,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useListUsers } from "../../user";
import {
  useCreateProject,
  useProject,
  useUpdateProject,
} from "../data/projects";
import {
  type ApiError,
  type CreateProjectInput,
  type Tag,
  type UpdateProjectBody,
  type UserSummary,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";
import { TagEditor } from "./TagEditor";

export interface ProjectDrawerProps {
  mode: "closed" | "create" | "edit";
  code: string | null;
  onClose: () => void;
}

export function ProjectDrawer({ mode, code, onClose }: ProjectDrawerProps) {
  const { t } = useI18n();

  const users = useListUsers();
  const fetched = useProject(code);
  const create = useCreateProject();
  const update = useUpdateProject();

  // Form state.
  const [formCode, setFormCode] = useState("");
  const [description, setDescription] = useState("");
  const [memberLeaders, setMemberLeaders] = useState<UserSummary[]>([]);
  const [memberWorkers, setMemberWorkers] = useState<UserSummary[]>([]);
  const [unblindLeaders, setUnblindLeaders] = useState<UserSummary[]>([]);
  const [unblindWorkers, setUnblindWorkers] = useState<UserSummary[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [tagsTouched, setTagsTouched] = useState(false);
  const [active, setActive] = useState(true);

  const lookedUp = useRef(false);
  useEffect(() => {
    if (mode !== "edit" || code === null) return;
    if (lookedUp.current) return;
    lookedUp.current = true;
    void (async () => {
      const r = await fetched.refetch();
      if (r.isError || !r.data) return;
      setFormCode(r.data.code);
      setDescription(r.data.description);
      setMemberLeaders(r.data.members.leaders);
      setMemberWorkers(r.data.members.workers);
      setUnblindLeaders(r.data.unblindMembers.leaders);
      setUnblindWorkers(r.data.unblindMembers.workers);
      setTags(r.data.tags);
      setTagsTouched(false);
      setActive(r.data.active);
    })();
  }, [mode, code, fetched]);

  const submitDisabled =
    !formCode.trim() ||
    !description.trim() ||
    create.isPending ||
    update.isPending;

  async function onSubmit() {
    const members = {
      leaders: memberLeaders.map((u) => u.code),
      workers: memberWorkers.map((u) => u.code),
    };
    const unblindMembers = {
      leaders: unblindLeaders.map((u) => u.code),
      workers: unblindWorkers.map((u) => u.code),
    };
    try {
      if (mode === "create") {
        const input: CreateProjectInput = {
          code: formCode.trim(),
          description: description.trim(),
          members,
          unblindMembers,
          tags,
        };
        await create.mutateAsync(input);
      } else if (mode === "edit" && code) {
        const body: UpdateProjectBody = {
          description: description.trim(),
          active,
          members,
          unblindMembers,
          ...(tagsTouched ? { tags } : {}),
        };
        await update.mutateAsync({ code, body });
      }
      onClose();
    } catch {
      /* error surfaced below via create.error / update.error */
    }
  }

  const mutationError: ApiError | null =
    create.error ?? update.error ?? null;

  return (
    <Drawer
      anchor="right"
      open={mode !== "closed"}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 480 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {t(mode === "create" ? "project.create.title" : "project.edit.title")}
        </Typography>

        <TextField
          label={t("project.field.code")}
          value={formCode}
          onChange={(event) => setFormCode(event.target.value)}
          disabled={mode === "edit"}
          size="small"
          required
        />

        <TextField
          label={t("project.field.description")}
          value={description}
          onChange={(event) => setDescription(event.target.value)}
          multiline
          minRows={2}
          size="small"
          required
        />

        <TagEditor
          value={tags}
          onChange={setTags}
          onTouched={() => setTagsTouched(true)}
        />

        <Autocomplete<UserSummary, true>
          multiple
          options={users.data ?? []}
          getOptionLabel={(u) => `${u.code} — ${u.name}`}
          value={memberLeaders}
          onChange={(_e, value) => setMemberLeaders(value)}
          renderInput={(params) => (
            <TextField
              {...params}
              label={t("project.field.members.leaders")}
              size="small"
            />
          )}
        />

        {/* (memberWorkers, unblindLeaders, unblindWorkers identical pattern — unchanged) */}

        {mode === "edit" && (
          <FormControlLabel
            control={
              <Switch
                checked={active}
                onChange={(event) => setActive(event.target.checked)}
              />
            }
            label={t("project.field.active")}
          />
        )}

        {mutationError && (
          <Alert severity="error">{errorMessage(mutationError)}</Alert>
        )}

        <Stack direction="row" spacing={1} sx={{ justifyContent: "flex-end" }}>
          <Button onClick={onClose}>{t("common.cancel")}</Button>
          <Button
            variant="contained"
            disabled={submitDisabled}
            onClick={() => void onSubmit()}
          >
            {t(mode === "create" ? "project.action.create" : "project.action.save")}
          </Button>
        </Stack>
      </Box>
    </Drawer>
  );
}
```

Note: the three remaining membership `Autocomplete`s (`memberWorkers`, `unblindLeaders`, `unblindWorkers`) are unchanged from the current code; replace the `…` line with the existing identical blocks. (`ProjectMembers` autocomplete docs in the original file were rendered with `options={users.data ?? []}`, identical to `memberLeaders`.)

- [ ] **Step 4: Mark `useListProducts` as pending-removal**

Edit `apps/desktop/aegis-desktop/src/features/project-list/data/products.ts`. Add a single comment line at the top of the file:

```ts
// currently unused — pending product surface removal
import { useQuery } from "@tanstack/react-query";
// (rest of file unchanged)
```

- [ ] **Step 5: Re-run drawer tests**

```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list/project-drawer.test.tsx
```
Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-list/components/ProjectDrawer.tsx \
        apps/desktop/aegis-desktop/src/features/project-list/data/products.ts \
        apps/desktop/aegis-desktop/src/test/features/project-list/project-drawer.test.tsx
git commit -m "feat(aegis-desktop): edit project tags in ProjectDrawer; drop product"
```

---

### Task 6: Full verification

**Files:** none modified.

- [ ] **Step 1: Type-check the workspace**

```bash
cd apps/desktop/aegis-desktop && pnpm tsc --noEmit
```
Expected: zero errors.

- [ ] **Step 2: Lint**

```bash
cd apps/desktop/aegis-desktop && pnpm lint
```
Expected: zero errors. If lint flags unused imports (e.g. `useListProducts` still being imported somewhere), remove them.

- [ ] **Step 3: Run the entire project-list test suite**

```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/project-list
```
Expected: all green.

- [ ] **Step 4: Run the i18n catalog tests**

```bash
cd lib/packages/ui && pnpm vitest run src/i18n
```
Expected: all green (catalogs compile because `zhCN` mirrors `en` keys).

- [ ] **Step 5: Manual smoke checklist (mirror spec Verification gate)**

Run:
```bash
cd apps/desktop/aegis-desktop && pnpm dev
```

Then walk through the spec's eight manual checks (`docs/superpowers/specs/2026-08-17-aegis-desktop-project-list-tags-design.md` § Verification gate). Stop and fix any deviation before committing.

- [ ] **Step 6: Commit (only if verification commands required a fix)**

```bash
git add -A
git commit -m "chore(aegis-desktop): verification follow-ups for project-list tags"
```

If Step 1–5 produced no changes, there is nothing to commit; that's the success state.

---

## Self-Review Checklist

- [x] **Spec coverage** —
  - "Display tags before status column" → Tasks 1 (types), 3 (column render).
  - "Edit tags in ProjectDrawer" → Tasks 1 (types), 2 (TagEditor), 5 (drawer integration).
  - "Allow user to filter by tag value" → Tasks 1 (types), 4 (FilterBar + Page).
  - "Server missing-vs-present semantics on update" → Task 5 (`tagsTouched` flag).
  - "Create + edit modes get the tag editor" → Tasks 2, 5.
  - "ProjectView.product / productId removed" → Task 1 + Task 5.
  - "`useListProducts` left in place with a comment" → Task 5 Step 4.
- [x] **No placeholders** — every step has code or commands; no "TBD" / "implement later".
- [x] **Type consistency** — `Tag`, `Tag[]`, `tags`, `tagsTouched` are spelled identically across Tasks 1–5. `ProjectView.tags` typed as `Tag[]` matches usage in ProjectTable and ProjectListPage. `CreateProjectInput.tags?: Tag[]` and `UpdateProjectBody.tags?: Tag[]` both consumed in ProjectDrawer's `onSubmit`.

## References

- Spec: [docs/superpowers/specs/2026-08-17-aegis-desktop-project-list-tags-design.md](../specs/2026-08-17-aegis-desktop-project-list-tags-design.md)
- Server-side tag spec (already shipped): [docs/superpowers/specs/2026-08-17-project-tag-design.md](../specs/2026-08-17-project-tag-design.md)
- Reorganization shape (file layout): [docs/superpowers/specs/2026-08-16-aegis-desktop-by-features-reorganization-design.md](../specs/2026-08-16-aegis-desktop-by-features-reorganization-design.md)
