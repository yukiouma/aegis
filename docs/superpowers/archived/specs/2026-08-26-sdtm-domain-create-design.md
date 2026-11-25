# SDTM domain create from list page

**Status:** Approved (revised 2026-08-26)
**Date:** 2026-08-26
**Branch:** `feat/desktop_domain-model-create-domain`

## Goal

Add the ability to create a new SDTM domain from the list page (`SdtmDomainList.tsx`). A `+` icon button in the operation-column header opens a drawer; submitting the drawer creates the domain. On success, the drawer closes, the list refetches, and the new row appears.

## Background

The Rust backend already exposes `create_sdtm_domain` (see [apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/domain.rs](../../apps/desktop/aegis-desktop/src-tauri/src/commands/domain_model/domain.rs) and [apps/desktop/aegis-desktop/src-tauri/src/http/domain_model/domain.rs](../../apps/desktop/aegis-desktop/src-tauri/src/http/domain_model/domain.rs)), but the desktop client has no wrapper, no input type, no `useCreateSdtmDomain` hook, and no UI to call it. `DomainTable` does not have a header-level action button — it is currently an empty `<TableCell />` — while the sibling `VariableTable` does, in the same position.

The existing `DomainEditDrawer` ([apps/desktop/aegis-desktop/src/features/domain-model/components/DomainEditDrawer.tsx](../../apps/desktop/aegis-desktop/src/features/domain-model/components/DomainEditDrawer.tsx)) handles the description-row management that the create drawer also needs. The sibling `VariableEditDrawer` already implements the create/edit dual-mode pattern we want to mirror.

## Decisions

1. **Extend `DomainEditDrawer` with a `mode` prop** instead of forking a parallel `DomainCreateDrawer`. Same rationale that produced `VariableEditDrawer`'s `mode: "create" | "edit"`.
2. **Description rows are pre-filled by language in create mode.** When the drawer opens in create mode and `availableLanguages` is non-empty, one row per language is created with `lang` pre-filled and `description` / `structure` blank for the user to complete. When `availableLanguages` is empty (no existing domains in the version), no rows are pre-filled and the "Add description" button is the only way to add rows. This mirrors the pre-fill behavior of `VariableEditDrawer` and lets the user fill out translations without first having to type language keys.
3. **After successful create, stay on the list.** No auto-navigate to the detail page.
4. **Reuse existing backend.** No Rust changes required; the `create_sdtm_domain` command already exists.

## File-level changes

### 1. Types — `apps/desktop/aegis-desktop/src/shared/api/types.ts`

Add after the `UpdateSdtmDomainInput` block:

```ts
export interface CreateSdtmDomainInput {
  versionId: number;
  name: string;
  category: DomainCategory;
  descriptions: SdtmDomainDescription[];
}
```

### 2. API wrapper — `apps/desktop/aegis-desktop/src/shared/api/index.ts`

- Import `CreateSdtmDomainInput`.
- Add wrapper:

  ```ts
  createSdtmDomain: (
    input: CreateSdtmDomainInput,
  ): Promise<SdtmDomainView> =>
    call<SdtmDomainView>("create_sdtm_domain", { input: { ...input } }),
  ```

  Uses `{ input: { ...input } }` to match the Rust command's `input` parameter name — same pattern as `createSdtmVariable`.
- Export `CreateSdtmDomainInput` from the re-export list.

### 3. Data hook — `apps/desktop/aegis-desktop/src/features/domain-model/data/list.ts`

Add:

```ts
export function useCreateSdtmDomain() {
  const qc = useQueryClient();
  return useMutation<SdtmDomainView, ApiError, CreateSdtmDomainInput>({
    mutationFn: (input) => api.createSdtmDomain(input),
    onSuccess: (created) => {
      qc.invalidateQueries({
        queryKey: ["domainModel", "sdtmDomains", created.versionId],
      });
    },
  });
}
```

### 4. `DomainEditDrawer` — `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainEditDrawer.tsx`

- Extend props:
  - `mode: "create" | "edit"` (default `"edit"`; existing call sites pass `"edit"` explicitly).
  - `versionId?: number` — required in create mode.
  - `availableLanguages?: string[]` — languages to pre-fill as empty description rows in create mode.
  - `onCreate?: (input: CreateSdtmDomainInput) => void`.
- Update `useEffect` reset to branch on mode:
  - `mode === "edit"` → existing behavior (rehydrate from `row`).
  - `mode === "create"` → `name=""`, `category="Special Purpose"`, `descriptions=(availableLanguages ?? []).map((lang) => ({ lang, details: { description: "", structure: "" } }))`.
- `handleSubmit`:
  - `create` → `onCreate({ versionId!, name: trimmed, category, descriptions: descriptions.filter((d) => d.lang.trim() !== "") })`.
  - `edit` → existing `onUpdate(row.id, body)` call.
- Title and submit label branch on mode (`t("domainModel.sdtm.create.title")` / `t("domainModel.sdtm.detail.editTitle")` for title; `t("common.create")` / `t("common.save")` for submit).
- Disable submit when `mode === "create"` and `versionId` is null.

### 5. `DomainTable` — `apps/desktop/aegis-desktop/src/features/domain-model/components/DomainTable.tsx`

- Add `onCreate?: () => void` prop.
- Replace the empty `<TableCell />` in the header with a right-aligned cell that, when `canMutate && onCreate`, renders an `IconButton` with `AddIcon` inside a `Tooltip` (`t("domainModel.sdtm.create.tooltip")`). Mirrors `VariableTable`'s header button.
- Import `AddIcon` from `@aegis/ui/icons` alongside the existing `OpenInNewIcon` and `DeleteIcon`.

### 6. Page wiring — `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainList.tsx`

- Add a discriminated-union state alongside `confirmDelete`:

  ```ts
  type DomainDrawerState =
    | { mode: "edit"; row: SdtmDomainView }
    | { mode: "create" }
    | null;
  const [domainDrawer, setDomainDrawer] = useState<DomainDrawerState>(null);
  ```

- Instantiate `useUpdateSdtmDomain()` and `useCreateSdtmDomain()`.
- Pass `onCreate={() => setDomainDrawer({ mode: "create" })}` to `DomainTable`.
- Render `<DomainEditDrawer>` with `mode`, `versionId={selectedVersionId}`, `availableLanguages={availableLanguages}`, `canMutate`, plus the matching `onCreate` / `onUpdate` handlers. On success of either mutation, `setDomainDrawer(null)`.
- Wire `mutationError` / `mutationPending` from both mutations (prefer the one matching the open mode).

### 7. i18n — `lib/packages/ui/src/i18n/locales/{en,zhCN}.ts`

Add three keys under `domainModel.sdtm`:

| Key | English | zh-CN |
| --- | --- | --- |
| `create.title` | `Create domain` | `新建域` |
| `create.tooltip` | `Create domain` | `新建域` |

Reuse `common.create` for the submit button.

## Out of scope

- Bulk-create / paste from CSV.
- Server-side validation tweaks (existing `CreateSdtmDomainRequest` semantics are kept).
- Removing or refactoring the edit-mode body of `DomainEditDrawer`.

## Testing

- Typecheck: `pnpm --filter aegis-desktop typecheck`.
- Existing frontend tests: `pnpm --filter aegis-desktop test`.
- Manual smoke:
  1. Sign in as `admin`/`root`.
  2. Pick a version that already has at least one domain (so `availableLanguages` is non-empty).
  3. Click the `+` icon in the operation-column header.
  4. Verify drawer opens with empty `Name`, default category, and one description row per existing language with the `lang` field pre-filled.
  5. Fill the description + structure fields for each pre-filled row.
  6. Submit. Drawer closes; new row appears in the table.
  7. Repeat on a version with zero domains — verify the drawer opens with no rows and the "Add description" button is the only way to add a row.