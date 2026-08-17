# Aegis Desktop — Project List Tag Display / Edit / Filter Design

## Goal

Surface the project tag data the server already returns (Rust wire DTOs are wired; TypeScript and components are not) across the `project-list` feature:

1. Display each project's tags in the list table, immediately before the status column.
2. Let the user add / remove / edit `key:value` tags from the `ProjectDrawer`, in both create and edit mode.
3. Add a free-text filter so the user can narrow the list to projects whose tag **values** contain a substring.

Server-side contracts and the Tauri `src-tauri/src/http/dto.rs` are already in place — see [the server-side tag spec](2026-08-17-project-tag-design.md) — and require no changes from this design.

## Scope

In scope:

- `apps/desktop/aegis-desktop/src/shared/api/types.ts` — surface `tags` on `ProjectView`, `CreateProjectInput`, `UpdateProjectBody`; add a `Tag` type.
- `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectTable.tsx` — new `Tags` column.
- `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectFilterBar.tsx` — new tag-value filter input.
- `apps/desktop/aegis-desktop/src/features/project-list/components/TagEditor.tsx` — new file, controllable row-stack editor.
- `apps/desktop/aegis-desktop/src/features/project-list/components/ProjectDrawer.tsx` — integrate `TagEditor`, gate "send tags" on a `tagsTouched` flag.
- `apps/desktop/aegis-desktop/src/features/project-list/pages/ProjectListPage.tsx` — own `tagQuery` state, AND it into the existing client-side filter `useMemo`.

Out of scope:

- Server, Tauri Rust, or `apis` crate changes — already done.
- Removing `Product`-related UI (no `Product` was ever rendered on the desktop; the only `product` reference left in the project drawer is the `product: ProductView` field on `ProjectView`, which the server no longer sends — handled in Section 1 as a removal, not a deferral).
- I18n additions beyond the minimum key constants needed for the new column and add-tag button. We will add three keys (`project.col.tags`, `project.filter.tag.label`, `project.field.tags.add`) and reuse the existing `project.field.tags.key` / `project.field.tags.value` pattern style.
- Tests beyond manual smoke tests listed in the Verification gate.

## Constraints (confirmed during brainstorming)

- Filter UX is a free-text input, matching the existing `query` field (substring, case-insensitive) — NOT a multi-select or autocomplete.
- Tag editor is a stack of `{ key, value }` rows, each with its own two `TextField`s and a remove button plus an "+ Add tag" appender.
- The tag editor shows in **both** create and edit modes (the server's `CreateProjectRequest.tags` already accepts tags; UX parity is desirable).
- Server's missing-vs-present semantics are preserved: an `update` body must NOT include `tags` unless the user actually edited the editor in this drawer session. A `tagsTouched` flag on the drawer gates that.

## Architecture

The feature still follows the by-feature shape settled in the [reorganization design spec](2026-08-16-aegis-desktop-by-features-reorganization-design.md): types live in `shared/api`, data hooks live in `features/project-list/data`, components and pages live in `features/project-list/{components,pages}`. No new module boundaries are needed — `TagEditor` fits alongside `ProjectTable` and `ProjectFilterBar`.

```
shared/api/types.ts                           ← adds Tag, surfaces tags on project types
features/project-list/
  data/projects.ts                            ← unchanged (Create/Update hooks already take the input/body types)
  components/
    ProjectTable.tsx                          ← inserts Tags column
    ProjectFilterBar.tsx                      ← adds tagQuery input (controlled)
    TagEditor.tsx (new)                       ← {key,value} row stack, pure controlled
    ProjectDrawer.tsx                         ← wires TagEditor, gates body on tagsTouched
  pages/ProjectListPage.tsx                   ← owns tagQuery, ANDs it into filteredRows
```

No data hooks change — `useListProjects`, `useCreateProject`, `useUpdateProject`, `useProject` already invalidate the right caches on success.

## Data Model

### Wire shape (mirrors what `src-tauri/src/http/dto.rs` already parses)

```ts
// shared/api/types.ts

export interface Tag {
  key: string;
  value: string;
}

export interface ProjectView {
  id: number;
  code: string;
  description: string;
  // product: ProductView — REMOVED. Server no longer sends it; leaving it
  //   here would either silently hang reads off `undefined.product` or
  //   require a runtime fallback. Type-correctness wins; clean removal.
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
  // productId: number — REMOVED for the same reason as above.
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
  tags?: Tag[];
}

export interface UpdateProjectBody {
  code?: string;
  description?: string;
  // productId?: number — REMOVED.
  active?: boolean;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
  tags?: Tag[];
}
```

### Editor local state (ProjectDrawer)

```ts
const [tags, setTags] = useState<Tag[]>([]);
const [tagsTouched, setTagsTouched] = useState(false);
```

The `tagsTouched` flag flips to `true` on the first `onChange` and is **not** reset by intermediate append/remove — only the user "touching" the editor (the React `onChange` of any inner TextField, the add button, or the remove button) flips it. The edit-open `useEffect` resets both: `setTags(r.data.tags); setTagsTouched(false);`.

### Filter state (ProjectListPage)

```ts
const [tagQuery, setTagQuery] = useState("");
```

The three filter dimensions (`query`, `involve`, `tagQuery`) compose with AND in the same `useMemo`.

## Components

### `TagEditor` (new)

```tsx
export interface TagEditorProps {
  value: Tag[];
  onChange: (next: Tag[]) => void;
  onTouched?: () => void;
}
```

- Pure controlled component. No `useEffect`-mirroring internal state.
- Renders one row per `value` entry:
  - `<TextField size="small" label={t("project.field.tags.key")} value={tag.key} onChange={...} sx={{ flex: 1 }} />`
  - `<TextField size="small" label={t("project.field.tags.value")} value={tag.value} onChange={...} sx={{ flex: 1 }} />`
  - `<IconButton aria-label={t("common.remove")} onClick={() => remove(i)}><Close /></IconButton>`
- Row layout: `Stack direction="row" spacing={1} alignItems="center"`.
- A trailing `Button startIcon={<Add />} size="small" onClick={append}>` reads `t("project.field.tags.add")`. `append` pushes `{ key: "", value: "" }`.
- `onTouched` (optional) fires once per user-driven interaction — the drawer uses this to flip its `tagsTouched` flag.
- After `append`, the editor focuses the new row's key `TextField` via an internal `useRef<number>` (last-appended index) → `useEffect` chain. Resets on every `value` prop change so a re-seed from the drawer also resets the focus pointer. No state for the value itself; it's purely a focus-management hook.
- No debouncing, no validation. Empty key or value rows are allowed through; the server rejects them with `validation_failed`, surfaced via the existing `Alert` on the drawer.

### `ProjectTable` (modify)

- Insert a `<TableCell>{t("project.col.tags")}</TableCell>` header **between** the `Leaders` and `Active` headers.
- Insert the matching body cell. Inside: a `Stack direction="row" spacing={0.5} sx={{ flexWrap: "wrap", gap: 0.5 }}` of MUI `Chip size="small"`. Each chip:
  - `label` = `tag.value`.
  - `title` = `tag.key` (hover-to-discover — key is not part of the task wording, but it's free metadata and matches the existing leaders pattern of revealing more on demand).
- Empty case: render `<span>—</span>` (matches the existing "no leaders" pattern).
- No column resize / truncation; the table does not impose column widths beyond what MUI does by default.

### `ProjectFilterBar` (modify)

- New prop pair: `tagQuery: string`, `onTagQueryChange: (value: string) => void`.
- Renders a second `TextField size="small" label={t("project.filter.tag.label")} value={tagQuery} onChange={...} sx={{ minWidth: 240 }}` to the right of the existing search field.
- Wrap with the same `Box sx={{ display: "flex", alignItems: "center", gap: 2 }}` — the existing `Involve` checkbox stays right-aligned via the existing `sx={{ ml: "auto" }}`.
- Pure controlled component, same pattern as today.

### `ProjectDrawer` (modify)

**Seed on edit-open:**

```ts
setTags(r.data.tags);
setTagsTouched(false);
```

**Render the editor** below the description `TextField`, above the first membership `Autocomplete` (which today is `memberLeaders`):

```tsx
<TagEditor
  value={tags}
  onChange={setTags}
  onTouched={() => setTagsTouched(true)}
/>
```

The product `Autocomplete` and its surrounding `productId` state, `setProductId` setter, the `useListProducts` import, and the `submitDisabled`'s `productId === null` clause are **all deleted** — the server no longer takes a product. This is consistent with the "remove the now-dead `product` field" decision above.

**onSubmit:**

```ts
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
```

`submitDisabled` does NOT consider `tags` — half-typed rows are surfaced via the server's validation error and the existing `Alert`.

**Imports:** `useListProducts` import is removed. `CreateProjectInput` / `UpdateProjectBody` continue to flow through; the field added is `tags`.

### `ProjectListPage` (modify)

- New state: `const [tagQuery, setTagQuery] = useState("")`.
- Pass `tagQuery` / `setTagQuery` down to `ProjectFilterBar`.
- Inside `filteredRows = useMemo(...)`, append an AND clause:
  - `if (t.length > 0 && !row.tags.some(tag => tag.value.toLowerCase().includes(t))) return false;`
  - where `t = tagQuery.trim().toLowerCase()`.
- Dependency array: add `tagQuery`.
- All other behavior (search filter over code/description/leaders, Involve filter) is unchanged.

## Tests

Per the project's existing pattern, this change ships with manual smoke tests documented in the Verification gate, not a new automated test file. The closest existing tests live in `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs` (Rust) and `apps/server/aegis-server/src/transport/http/project/handlers.rs` (Rust server) — both already cover the wire behavior. The frontend hook layer (`features/project-list/data/projects.ts`) is exercised end-to-end by the `useListProjects` cache invalidation flow.

## Cross-cutting ripple

- `features/project-list/data/products.ts` — `useListProducts` has zero remaining consumers after this change (the drawer's product picker goes away). It remains exported from `index.ts` (per the existing public API surface) but with no internal callers. **Decision:** leave the export and the file in place; removing it is out of scope for "match the server update", and unannounced removals expand the diff. Mark with a `// currently unused — pending product surface removal` comment so future cleanup can find it.

## Verification gate

Manual:

1. `pnpm --filter aegis-desktop dev` (or the local equivalent per `apps/desktop/aegis-desktop/package.json`) — confirm type-check passes and dev server starts.
2. Smoke: open `/projects`. Each project's tags render as chips immediately before the green check / gray cancel of the status column. Hover a chip — its key is shown.
3. Type `demo` in the new tag filter — projects whose tag values include "demo" remain; others drop. Clear the filter — all rows return.
4. Open the create drawer. The product autocomplete is gone. The new tag editor appears between description and the membership multi-autocompletes. Add a `Product / DEMO-001` pair, submit. The new row appears with the chip labelled "DEMO-001" and `title="Product"`.
5. Open edit on a project that already has a tag. The tag editor seeds with the existing array and `tagsTouched === false`. Submit without touching the editor — refresh, tags are unchanged.
6. Open edit, remove a tag, submit. Refresh, the chip is gone. (This is the `tagsTouched === true` path.)
7. Open edit, edit a tag's value in-place, submit. Refresh, the chip label reflects the new value but the key matches.
8. Type into the key half of a row but leave the value empty, submit. The server rejects with `validation_failed`; the existing `Alert` surfaces the message; the drawer stays open.

Automated (regression only — no new tests added):

```bash
pnpm --filter aegis-desktop tsc --noEmit
pnpm --filter aegis-desktop lint
```

Rust side untouched, so no Rust verification is needed for this change.

## Open questions

None at design time. Implementation-time unknowns (e.g. the exact i18n file path that holds the project-related keys) are surfaced during the plan step.
