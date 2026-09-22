# SDTMIG Version Dropdown — Project Configuration General Section Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an SDTMIG Version `<Select>` to the existing `ConfigurationGeneralSection`. Options come from `api.listSdtmVersions()`; the selection round-trips through the `configurations.sdtmig` field on the project's wire payload.

**Architecture:** Mirror the existing `languageTouched`/`tagsTouched` pattern: a new `sdtmTouchedRef` flips on first interaction; the save body extends from `{ language, tags }` to `{ language, tags, sdtmig }`. The dropdown is a third `<FormControl>` between Language and Tags, populated by the already-wired `useListSdtmVersions()` hook. Local selection state is `{ versionId, versionName } | null`, seeded from `initial.sdtmig` so the control is correctly set on first render even before the version list resolves.

**Tech Stack:** TypeScript + React + TanStack Query (no new deps). MUI components from `@aegis/ui/mui` (no new ones — the existing `Select`/`MenuItem`/`FormControl`/`InputLabel` are enough). vitest + @testing-library for tests.

## Global Constraints

- **Wire contract (already shipped server-side):** `dto::ModelVersionRequest { versionId: i64, versionName: String }` with `#[serde(rename_all = "camelCase")]` (`apps/server/aegis-server/src/transport/http/dto.rs:308-340`). Inner `ProjectConfigurationDataRequest.sdtmig` is `Option<ModelVersionRequest>` with `#[serde(skip_serializing_if = "Option::is_none")]` (line 388). Server-side work is **already merged** on `main` — no backend touch in this plan.
- **TS interface convention:** camelCase TS identifiers mirror the camelCase `#[serde(rename_all = "camelCase")]` wire keys. The existing `types.ts` header comment notes that for non-rename-all structs the wire is snake_case; for `ModelVersionRequest`/`ModelVersionResponse` the wire is camelCase, so the TS mirror matches `versionId` / `versionName` 1:1.
- **Existing patterns to follow:**
  - `ConfigurationGeneralSection` `touched` pattern: `languageTouchedRef` + `tagsTouchedRef` (`apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx:54-67`). Add `sdtmTouchedRef` the same way.
  - `useListSdtmVersions()` from `features/domain-model/data/list.ts` — `queryKey: queryKeys.domainModel.sdtmVersions()`, returns `SdtmVersionView[]`. Reuse as-is.
  - `useUpdateProject()` from `features/project-list/data/projects.ts` already invalidates `project.all` + `project.byCode(code)` on success; the existing `lastInitialRef` reseed effect in `ConfigurationGeneralSection` handles the post-save refetch.
  - i18n locale files are exhaustively keyed on `TranslationKey = keyof typeof en`; a typo or missing zh-CN entry fails `pnpm --filter @aegis/ui typecheck`.
- **Save semantics:** `Some(configurations)` on update means whole-replace. Always sending `{ language, tags, sdtmig }` matches the existing "always send all three fields" behavior; a `null` sdtmig is dropped on serialize by the server's `skip_serializing_if`, leaving the row unchanged.
- **No test fixture changes needed:** `makeProject` (`apps/desktop/aegis-desktop/src/test/helpers/project-fixture.ts`) uses `configurations: { language: null, tags: [] }`. `sdtmig` is optional + nullable, so omitting it from a fixture stays valid.
- **Verification gate** (every task):
  ```bash
  pnpm --filter aegis-desktop typecheck
  pnpm --filter aegis-desktop test
  pnpm --filter @aegis/ui typecheck
  ```

## File Structure

### Modified

- `apps/desktop/aegis-desktop/src/shared/api/types.ts` — extend `ProjectConfiguration` + add `ProjectConfigurationSdtmig`.
- `lib/packages/ui/src/i18n/locales/en.ts` — two new keys.
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — same two keys.
- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx` — new `<FormControl>` + touched ref + extended save body.
- `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx` — add SDTMIG-focused cases + extend two existing cases.

No new files.

---

## Task 1: TS wire mirror — add `sdtmig` to `ProjectConfiguration`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts`

**Interfaces:**
- Produces: `ProjectConfigurationSdtmig { versionId: number; versionName: string }` and `ProjectConfiguration { language, tags, sdtmig? }` consumed by Tasks 2 and 3.

- [ ] **Step 1: Add the `ProjectConfigurationSdtmig` interface and extend `ProjectConfiguration`**

In `apps/desktop/aegis-desktop/src/shared/api/types.ts`, replace the existing `ProjectConfiguration` block (currently lines 116-119):

```ts
/** Project configuration payload: optional locale, the tag list, and
 *  an optional SDTM-IG version pointer. Server treats `Some(config)`
 *  on update as whole-replace. */
export interface ProjectConfigurationSdtmig {
  versionId: number;
  versionName: string;
}

export interface ProjectConfiguration {
  language: ProjectLanguage | null;
  tags: Tag[];
  sdtmig?: ProjectConfigurationSdtmig | null;
}
```

The `versionId` / `versionName` field names are camelCase to mirror the server's `#[serde(rename_all = "camelCase")]` on `dto::ModelVersionRequest` / `ModelVersionResponse` (`apps/server/aegis-server/src/transport/http/dto.rs:308-340`). `sdtmig` is `?` (optional) **and** nullable so existing fixtures that omit the field (`makeProject` in `test/helpers/project-fixture.ts`) keep compiling.

No other types change.

- [ ] **Step 2: Verify**

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter @aegis/ui typecheck
```

Expected: green. The `sdtmig` field is optional, so no existing caller breaks.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts
git commit -m "wire(desktop): mirror ProjectConfiguration.sdtmig

The TS mirror of the project's wire shape gains:
  - ProjectConfigurationSdtmig { versionId, versionName }
  - ProjectConfiguration.sdtmig?: ... | null

Mirrors the server-side dto::ModelVersionRequest /
ModelVersionResponse (#[serde(rename_all = \"camelCase\")]).
Optional + nullable so existing fixtures that omit the field stay
valid.

Spec coverage: TS wire mirror per the
project-config-sdtmig-dropdown design.

Verification: pnpm --filter aegis-desktop typecheck &&
pnpm --filter @aegis/ui typecheck"
```

---

## Task 2: i18n — add the two SDTMIG keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Produces: keys `project.configuration.general.sdtmig` and `project.configuration.general.sdtmig.none` consumed by Task 3.

- [ ] **Step 1: Add the en keys**

In `lib/packages/ui/src/i18n/locales/en.ts`, after the existing `'project.configuration.general.tags'` line (currently line 128) and before `'project.configuration.members.leadersHeading'`, insert:

```ts
  'project.configuration.general.sdtmig': 'SDTMIG Version',
  'project.configuration.general.sdtmig.none': '(unspecified)',
```

(Keep the trailing comma on the last added line.)

- [ ] **Step 2: Add the zh-CN keys**

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, in the same position (after `'project.configuration.general.tags'`, before `'project.configuration.members.leadersHeading'`, currently line 126-127), insert:

```ts
  'project.configuration.general.sdtmig': 'SDTM-IG 版本',
  'project.configuration.general.sdtmig.none': '（未指定）',
```

The `'（未指定）'` text matches the existing `project.configuration.general.language.none` value (line 65) for consistency.

- [ ] **Step 3: Verify**

```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter @aegis/ui test
```

Expected: green. `TranslationKey = keyof typeof en` enforces parity — a missing zh-CN entry would fail `typecheck`.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "i18n: add project-configuration-general-sdtmig keys

Two new keys for the SDTMIG Version dropdown label and its
'(unspecified)' / '（未指定）' menu item. Mirrors the existing
language.none convention.

Both en and zh-CN are added — the typed TranslationKey union on en
enforces parity at compile time.

Spec coverage: i18n per the project-config-sdtmig-dropdown design.

Verification: pnpm --filter @aegis/ui typecheck &&
pnpm --filter @aegis/ui test"
```

---

## Task 3: SDTMIG Select in `ConfigurationGeneralSection` (TDD)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx`

**Interfaces:**
- Consumes: `ProjectConfigurationSdtmig` from Task 1, `useListSdtmVersions()` from `features/domain-model/data/list.ts`, the i18n keys from Task 2.
- Produces: `ConfigurationGeneralSection` now renders an SDTMIG `<Select>` between the language `<Select>` and the `<TagEditor>`; the save payload now includes `sdtmig`.

### Sub-task 3.1 — Write the failing tests

- [ ] **Step 1: Extend the existing `mockCommands` blocks to provide `list_sdtm_versions`**

In `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx`, after the `leader` / `other` user constants (lines 17-35), add a shared sdtm-version fixture and a helper that builds the mocked command set:

```tsx
const sdtmVersions: { versions: { id: number; name: string; createdAt: string; updatedAt: string }[] } = {
  versions: [
    { id: 1, name: "SDTMIG v3.1.2", createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z" },
    { id: 2, name: "SDTMIG v3.2",   createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z" },
    { id: 3, name: "SDTMIG v3.3",   createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z" },
  ],
};

const projectWithSdtmig: ProjectView = makeProject({
  code: "alpha",
  members: {
    leaders: [{ code: "alice", name: "Alice" }],
    workers: [{ code: "bob", name: "Bob" }],
  },
  unblindMembers: { leaders: [], workers: [] },
  configurations: {
    language: "en",
    tags: [{ key: "Product", value: "DEMO-001" }],
    sdtmig: { versionId: 2, versionName: "SDTMIG v3.2" },
  },
});
```

Then change **every** existing `mockCommands({ ... })` block that does not already include `list_sdtm_versions` so it does. There are four call sites — three `beforeEach` blocks inside `describe("ProjectConfigurationPage — leader view", ...)` / `… non-leader view` / `… filepath placeholder`, plus the two `mockCommands` calls inside individual `it()` blocks (the "Save …" and "filepath" tests). Each needs `list_sdtm_versions: () => sdtmVersions` added.

- [ ] **Step 2: Extend the existing "Save fires update_project with configurations { language, tags }" test to also assert `sdtmig`**

Replace the existing `it("clicking Save fires update_project with configurations { language, tags }", ...)` body (currently lines 126-161 in `project-configuration-page.test.tsx`) so the final `expect.objectContaining` shape is `{ configurations: expect.objectContaining({ language, tags, sdtmig }) }`:

```tsx
it("clicking Save fires update_project with configurations { language, tags, sdtmig }", async () => {
  mockCommands({
    is_logged_in: () => true,
    current_user: () => leader,
    get_project_by_code: () => projectFixture,
    list_sdtm_versions: () => sdtmVersions,
    update_project: () => projectFixture,
  });
  await renderPage();

  // Edit the existing tag's value to flip `tagsTouched`. The whole
  // configurations object is then sent on save.
  const valueInput = await screen.findByLabelText(/tag value/i);
  await userEvent.clear(valueInput);
  await userEvent.type(valueInput, "DEMO-002");

  const save = await screen.findByTestId("config-general-save");
  await waitFor(() => expect(save).not.toBeDisabled());
  await userEvent.click(save);

  await waitFor(() =>
    expect(invoke).toHaveBeenCalledWith(
      "update_project",
      expect.objectContaining({
        code: "alpha",
        body: expect.objectContaining({
          configurations: expect.objectContaining({
            language: "en",
            tags: expect.arrayContaining([
              { key: "Product", value: "DEMO-002" },
            ]),
            sdtmig: null,
          }),
        }),
      }),
    ),
  );
});
```

The `sdtmig: null` assertion is correct here: the test's `projectFixture` has no `sdtmig` (it's optional + nullable), so the seeded local state is `null`, and the whole-replace sends `null`.

- [ ] **Step 3: Add the SDTMIG-specific tests**

After the modified test from Step 2, add the new cases. Each goes inside the existing `describe("ProjectConfigurationPage — leader view", ...)` block:

```tsx
  it("renders the SDTMIG Select with the seeded version preselected", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectWithSdtmig,
      list_sdtm_versions: () => sdtmVersions,
    });
    await renderPage();
    // The Select's rendered value reflects the saved version's name.
    expect(
      await screen.findByDisplayValue("SDTMIG v3.2"),
    ).toBeInTheDocument();
  });

  it("renders an '(unspecified)' item as the first option", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      list_sdtm_versions: () => sdtmVersions,
    });
    await renderPage();
    // Open the Select to render the MenuItem children into the DOM.
    await userEvent.click(await screen.findByLabelText(/sdtmig version/i));
    expect(
      await screen.findByRole("option", { name: /\(unspecified\)/i }),
    ).toBeInTheDocument();
  });

  it("picking a SDTMIG version enables Save", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      list_sdtm_versions: () => sdtmVersions,
    });
    await renderPage();
    const save = await screen.findByTestId("config-general-save");
    expect(save).toBeDisabled();

    await userEvent.click(await screen.findByLabelText(/sdtmig version/i));
    await userEvent.click(
      await screen.findByRole("option", { name: /sdtmig v3\.2/i }),
    );

    await waitFor(() => expect(save).not.toBeDisabled());
  });

  it("picking '(unspecified)' → Save → body.configurations.sdtmig === null", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectWithSdtmig,
      list_sdtm_versions: () => sdtmVersions,
      update_project: () => projectFixture,
    });
    await renderPage();

    await userEvent.click(await screen.findByLabelText(/sdtmig version/i));
    await userEvent.click(
      await screen.findByRole("option", { name: /\(unspecified\)/i }),
    );

    const save = await screen.findByTestId("config-general-save");
    await waitFor(() => expect(save).not.toBeDisabled());
    await userEvent.click(save);

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "update_project",
        expect.objectContaining({
          code: "alpha",
          body: expect.objectContaining({
            configurations: expect.objectContaining({
              sdtmig: null,
            }),
          }),
        }),
      ),
    );
  });
```

Then, inside the existing `describe("ProjectConfigurationPage — non-leader view", ...)` block (currently lines 171-208), add the disabled-Select sibling next to the existing language-disabled test:

```tsx
  it("SDTMIG Select is disabled for non-leaders", async () => {
    await renderPage();
    const select = await screen.findByLabelText(/sdtmig version/i);
    await waitFor(() => {
      const fc = select.closest(".MuiFormControl-root");
      const root = select.closest(".MuiInputBase-root");
      expect(
        fc!.classList.contains("Mui-disabled") ||
          root!.classList.contains("Mui-disabled"),
      ).toBe(true);
    });
  });
```

- [ ] **Step 4: Run the tests to verify they fail**

```bash
pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-configuration-page.test.tsx
```

Expected: the new tests fail with "Unable to find i/o element with label text matching `/sdtmig version/i`" (the `<Select>` doesn't exist yet). The two extended tests fail because the modified `expect.objectContaining({ ..., sdtmig: null })` doesn't match the current body shape (the current shape is `{ language, tags }` only).

Capture the failing test names in the commit body of Step 9.

### Sub-task 3.2 — Implement the SDTMIG Select

- [ ] **Step 5: Add the import + state + touched ref**

In `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx`, replace the existing imports block (currently lines 1-22) with:

```tsx
import { useEffect, useRef, useState } from "react";
import {
  Alert,
  Box,
  Button,
  FormControl,
  InputLabel,
  MenuItem,
  Select,
  Stack,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import {
  type ApiError,
  type ProjectConfiguration,
  type ProjectConfigurationSdtmig,
  type ProjectLanguage,
  type Tag,
} from "../../../shared/api";
import { errorMessage } from "../../../shared/api/error";
import { useListSdtmVersions } from "../../domain-model/data/list";
import { useUpdateProject } from "../../project-list";
import { TagEditor } from "../../project-list/components/TagEditor";
```

(Just three changes vs. the existing imports: drop nothing; add `ProjectConfigurationSdtmig` to the type import; add the `useListSdtmVersions` import.)

- [ ] **Step 6: Add the local state, ref, hook, and extend the dirty / save**

Inside `ConfigurationGeneralSection`, immediately after the existing `tagsTouchedRef` declaration (currently line 55, before the `useEffect` block), add:

```tsx
  const [sdtmig, setSdtmig] = useState<ProjectConfigurationSdtmig | null>(
    initial.sdtmig ?? null,
  );
  const sdtmTouchedRef = useRef(false);

  const versions = useListSdtmVersions();
```

Extend the existing reseed `useEffect` (currently lines 61-68) by appending two more lines inside its body:

```tsx
    setSdtmig(initial.sdtmig ?? null);
    sdtmTouchedRef.current = false;
```

(Effect body becomes: `setLanguage(initial.language); setTags(initial.tags); setSdtmig(initial.sdtmig ?? null); languageTouchedRef.current = false; tagsTouchedRef.current = false; sdtmTouchedRef.current = false;`.)

Update the `dirty` line (currently line 70) to:

```tsx
  const dirty =
    languageTouchedRef.current ||
    tagsTouchedRef.current ||
    sdtmTouchedRef.current;
```

Replace the `onSave` body (currently lines 73-82) with:

```tsx
  async function onSave() {
    const configurations: ProjectConfiguration = { language, tags, sdtmig };
    await update.mutateAsync({
      code: projectCode,
      body: { configurations },
    });
  }
```

- [ ] **Step 7: Add the SDTMIG `<FormControl>` to the rendered body**

Between the existing language `<FormControl>` (currently lines 86-109) and the `<Box>` wrapping the `<TagEditor>` (currently lines 111-122), insert the new `<FormControl>`:

```tsx
      <FormControl size="small" disabled={readonly}>
        <InputLabel id="config-sdtmig-label">
          {t("project.configuration.general.sdtmig")}
        </InputLabel>
        <Select<string>
          labelId="config-sdtmig-label"
          label={t("project.configuration.general.sdtmig")}
          value={sdtmig ? String(sdtmig.versionId) : ""}
          onChange={(e) => {
            const v = e.target.value;
            if (v === "") {
              setSdtmig(null);
            } else {
              const id = Number(v);
              const found = versions.data?.find((x) => x.id === id);
              setSdtmig({ versionId: id, versionName: found?.name ?? "" });
            }
            sdtmTouchedRef.current = true;
          }}
          inputProps={{ "data-testid": "config-sdtmig-input" }}
        >
          <MenuItem value="">
            {t("project.configuration.general.sdtmig.none")}
          </MenuItem>
          {(versions.data ?? []).map((v) => (
            <MenuItem key={v.id} value={String(v.id)}>
              {v.name}
            </MenuItem>
          ))}
        </Select>
      </FormControl>
```

The "find-or-blank-name" fallback handles the leading render before `useListSdtmVersions` resolves: `initial.sdtmig.versionName` is the seed, so even if the version isn't in `versions.data` (it always is, in practice), the selection still round-trips. After the query resolves, every option renders and picking one looks the name up in the resolved list.

No other JSX changes.

- [ ] **Step 8: Run the test suite to verify everything passes**

```bash
pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-configuration-page.test.tsx
```

Expected: green. The full configuration-page suite passes, including the four new SDTMIG cases and the two extended cases (the existing "Save fires update_project" test now asserts the three-field body; the existing "language Select is disabled" test is unchanged but the new "SDTMIG Select is disabled" sibling sits next to it).

Then run the full suite + typechecks to catch anything else:

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter @aegis/ui typecheck
```

Expected: green across the board.

- [ ] **Step 9: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx \
        apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx
git commit -m "feat(general): SDTMIG Version dropdown in ConfigurationGeneralSection

The General section now has three controls in this order:
Language -> SDTMIG Version -> Tags. The SDTMIG Select is populated
by useListSdtmVersions(); the seed state comes from initial.sdtmig
so a saved pointer renders correctly even before the version list
resolves. sdtmTouchedRef flips on first interaction; the save body
extends from { language, tags } to { language, tags, sdtmig } so
the server's whole-configuration-replace semantics stay
consistent.

Tests cover: preselected version renders, '(unspecified)' is the
first option, picking a version enables Save, picking
'(unspecified)' round-trips sdtmig: null, the save body now
asserts the three-field shape, and the Select is disabled for
non-leaders.

Spec coverage: ConfigurationGeneralSection dropdown per the
project-config-sdtmig-dropdown design.

Verification: pnpm --filter aegis-desktop test &&
pnpm --filter aegis-desktop typecheck &&
pnpm --filter @aegis/ui typecheck"
```

---

## Self-Review

**1. Spec coverage**

| Spec requirement | Plan task |
| --- | --- |
| Extend `ProjectConfiguration` with `sdtmig` | Task 1 |
| Add i18n keys (`sdtmig`, `sdtmig.none`) in en + zh-CN | Task 2 |
| `sdtmig` local state, `sdtmTouchedRef`, reseed on `initial` change | Task 3 (Steps 5-6) |
| `dirty` flag includes sdtm touched | Task 3 (Step 6) |
| Save body includes `sdtmig` | Task 3 (Step 6) |
| New `<FormControl>` between Language and Tags | Task 3 (Step 7) |
| Options from `useListSdtmVersions()` | Task 3 (Step 7) |
| `(unspecified)` MenuItem first | Task 3 (Step 7) |
| Find-or-blank-name fallback | Task 3 (Step 7) |
| `readonly` disables the Select | Task 3 (Step 7) |
| Tests: preselected render, "(unspecified)" option, save-enable, null round-trip, three-field body, non-leader disabled | Task 3 (Steps 2-3) |

No gaps.

**2. Placeholder scan** — no "TBD" / "TODO" / "implement later" anywhere. Every step shows concrete code or commands.

**3. Type consistency** — `ProjectConfigurationSdtmig` is defined in Task 1 and referenced by the same `{ versionId: number; versionName: string }` shape in Task 3 (Steps 5-7) and in the tests (Step 3). `sdtmig` state type `Task.ConfigurationSdtmig | null` matches both the seed `initial.sdtmig ?? null` and the save payload `{ language, tags, sdtmig }`. The `useListSdtmVersions()` hook is reused as-is across Steps 5-7 and the test mocks (Step 1).

**4. Task ordering** — wire (Task 1) → i18n (Task 2) → feature with TDD (Task 3). Task 3 is internally ordered test-first (Steps 1-4) → implementation (Steps 5-7) → verify (Step 8) → commit (Step 9), so the failing-test gate is captured before any implementation touches the component.

Plan complete and saved to `docs/superpowers/plans/2026-09-22-project-configuration-sdtmig-dropdown.md`.