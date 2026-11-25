# CRF Detail SDTM Autocomplete Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add two SDTM-aware autocompletes to the CRF detail page — a domain-name Autocomplete + description auto-fill on `DomainAnnotationDialog`, and an `@`-variable mention Popover on `AnnotationDialog` — driven by a new `useSdtmContext` hook that resolves the project's SDTMIG version + language (with silent name-match self-heal of a stale `versionId`).

**Architecture:** A single hook (`useSdtmContext`) owns the version/language resolution and the SDTM-domain fetch. `CrfDetailPage` consumes the hook and threads `sdtmDomains` + `sdtmLanguage` into both dialogs. The dialogs own their own autocomplete UX (MUI `<Autocomplete>` for the domain name, a controlled `<TextField>` + `<Popover>` for the `@`-mention). No backend changes; no new endpoints.

**Tech Stack:** React + TypeScript, MUI (`Autocomplete`, `Popover`, `MenuList`, `MenuItem`), `@tanstack/react-query` (existing `useListSdtmVersions` / `useListSdtmDomains` / `useListSdtmVariables` / `useProject` / `useUpdateProject`), Vitest + Testing Library.

---

## File Structure

### New files

| Path | Responsibility |
| --- | --- |
| `apps/desktop/aegis-desktop/src/features/crf/data/sdtm.ts` | `useSdtmContext(projectCode)` hook — version + language resolution, self-heal mutation, SDTM-domain fetch. |
| `apps/desktop/aegis-desktop/src/features/crf/data/sdtm.test.tsx` | Resolution-table tests for `useSdtmContext`. |

### Edited files

| Path | Change |
| --- | --- |
| `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx` | Call `useSdtmContext(projectCode)`; pass `sdtmDomains` + `sdtmLanguage` into both dialogs. |
| `apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.tsx` | New `sdtmDomains` + `sdtmLanguage` props; Name field becomes MUI `<Autocomplete>`; description auto-fills on selection. |
| `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx` | New `sdtmDomains` prop; Content field becomes a controlled input + `@`-trigger detection; Popover with variable list. |
| `apps/desktop/aegis-desktop/src/test/features/crf/components/DomainAnnotationDialog.test.tsx` | Add Autocomplete + auto-fill cases. |
| `apps/desktop/aegis-desktop/src/test/features/crf/components/AnnotationDialog.test.tsx` | Add `@`-mention cases. |
| `apps/desktop/aegis-desktop/src/test/features/crf/CrfDetailPage.test.tsx` | Add `sdtmDomains` / `sdtmLanguage` props to the dialog mounts (smoke). |
| `lib/packages/ui/src/i18n/locales/en.ts` | 2 new keys. |
| `lib/packages/ui/src/i18n/locales/zhCN.ts` | 2 new keys (English placeholders). |

---

## Task 1: Add i18n keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts:411-425`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts:415-430`

- [ ] **Step 1: Add the two keys to `en.ts`**

Open `lib/packages/ui/src/i18n/locales/en.ts`. Find the block:

```
"crf.annotationDialog.domainAnnotation.none": "No domain annotations on this form",
```

Append immediately after it:

```
"crf.annotationDialog.field.content.placeholder": "Type content — use @ to insert a variable",
"crf.annotationDialog.variable.noMatch": "No variables match",
```

- [ ] **Step 2: Add the same two keys to `zhCN.ts`**

Open `lib/packages/ui/src/i18n/locales/zhCN.ts`. Find:

```
"crf.annotationDialog.domainAnnotation.none": "该表单暂无域注释",
```

Append after it:

```
"crf.annotationDialog.field.content.placeholder": "Type content — use @ to insert a variable",
"crf.annotationDialog.variable.noMatch": "No variables match",
```

(Both keys ship in English as placeholders — mirrors the existing `2026-09-22-aegis-desktop-project-config-sdtmig-dropdown-design.md` pattern.)

- [ ] **Step 3: Verify the UI package typechecks**

Run:
```bash
pnpm --filter @aegis/ui typecheck
```
Expected: exit 0, no errors.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "i18n: add crf.annotationDialog variable-mention keys"
```

---

## Task 2: Create `useSdtmContext` hook

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/crf/data/sdtm.ts`
- Create: `apps/desktop/aegis-desktop/src/features/crf/data/sdtm.test.tsx`

This hook owns the SDTM version + language resolution. It consumes three existing hooks (`useProject`, `useListSdtmVersions`, `useUpdateProject`) and one new internal call (`useListSdtmDomains`). It self-heals a stale `versionId` by firing `useUpdateProject` once per stale id per session.

- [ ] **Step 1: Write the failing test file**

Create `apps/desktop/aegis-desktop/src/features/crf/data/sdtm.test.tsx` with this exact content:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { mockCommands, mockInvoke } from "../../../test/helpers/tauri-mock";
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";
import { useSdtmContext } from "./sdtm";
import type {
  ProjectView,
  SdtmDomainView,
  SdtmVersionView,
} from "../../../shared/api";

function Probe({ projectCode }: { projectCode: string }) {
  const ctx = useSdtmContext(projectCode);
  return (
    <pre data-testid="ctx">
      {JSON.stringify({
        versionId: ctx.versionId,
        language: ctx.language,
        domainsLen: ctx.domains.length,
        loading: ctx.loading,
        error: ctx.error ? String(ctx.error) : null,
      })}
    </pre>
  );
}

function mount(projectCode: string) {
  return render(
    <TestQueryProvider>
      <AegisI18nProvider>
        <Probe projectCode={projectCode} />
      </AegisI18nProvider>
    </TestQueryProvider>,
  );
}

const versions: SdtmVersionView[] = [
  { id: 1, name: "SDTMIG v3.3", createdAt: "", updatedAt: "" },
  { id: 5, name: "SDTMIG v3.4", createdAt: "", updatedAt: "" },
  { id: 3, name: "SDTMIG v3.2", createdAt: "", updatedAt: "" },
];

const domains: SdtmDomainView[] = [
  {
    id: 100,
    versionId: 5,
    name: "AE",
    category: "Events",
    descriptions: [
      { lang: "en", details: { description: "Adverse Events", structure: "" } },
    ],
    createdAt: "",
    updatedAt: "",
  },
];

const baseProject: ProjectView = {
  id: 1,
  code: "abc",
  description: "",
  members: { leaders: [], workers: [] },
  unblindMembers: { leaders: [], workers: [] },
  configurations: { language: null, tags: [] },
  active: true,
  createdAt: "",
  updatedAt: "",
};

beforeEach(() => {
  mockInvoke.mockReset();
});
afterEach(() => cleanup());

describe("useSdtmContext — versionId", () => {
  it("uses the highest id when sdtmig is absent", async () => {
    mockCommands({
      get_project_by_code: () => baseProject,
      list_sdtm_versions: () => versions,
      list_sdtm_domains_by_version: (args: any) => domains,
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).versionId).toBe(5),
    );
  });

  it("uses the configured versionId when it is valid", async () => {
    mockCommands({
      get_project_by_code: () => ({
        ...baseProject,
        configurations: {
          language: null,
          tags: [],
          sdtmig: { versionId: 3, versionName: "SDTMIG v3.2" },
        },
      }),
      list_sdtm_versions: () => versions,
      list_sdtm_domains_by_version: (args: any) => domains,
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).versionId).toBe(3),
    );
  });

  it("self-heals when versionId is stale but versionName matches", async () => {
    let updateCalls = 0;
    mockCommands({
      get_project_by_code: () => ({
        ...baseProject,
        configurations: {
          language: "en",
          tags: [],
          sdtmig: { versionId: 999, versionName: "SDTMIG v3.4" },
        },
      }),
      list_sdtm_versions: () => versions,
      list_sdtm_domains_by_version: (args: any) => domains,
      update_project: (_args: any) => {
        updateCalls += 1;
        return baseProject;
      },
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).versionId).toBe(5),
    );
    // Give the microtask queue a chance to flush the mutation.
    await waitFor(() => expect(updateCalls).toBeGreaterThanOrEqual(1));
    // Rerender does not fire a second correction.
    await waitFor(() => expect(updateCalls).toBeLessThanOrEqual(1));
  });

  it("falls back to highest id when both versionId and versionName are stale", async () => {
    let updateCalls = 0;
    mockCommands({
      get_project_by_code: () => ({
        ...baseProject,
        configurations: {
          language: null,
          tags: [],
          sdtmig: { versionId: 999, versionName: "ghost" },
        },
      }),
      list_sdtm_versions: () => versions,
      list_sdtm_domains_by_version: (args: any) => domains,
      update_project: () => {
        updateCalls += 1;
        return baseProject;
      },
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).versionId).toBe(5),
    );
    expect(updateCalls).toBe(0);
  });

  it("preserves language + tags in the recovery mutation body", async () => {
    let captured: any = null;
    mockCommands({
      get_project_by_code: () => ({
        ...baseProject,
        configurations: {
          language: "zh-CN",
          tags: [{ key: "k", value: "v" }],
          sdtmig: { versionId: 999, versionName: "SDTMIG v3.4" },
        },
      }),
      list_sdtm_versions: () => versions,
      list_sdtm_domains_by_version: (args: any) => domains,
      update_project: (args: any) => {
        captured = args;
        return baseProject;
      },
    });
    mount("abc");
    await waitFor(() =>
      expect(captured).not.toBeNull(),
    );
    expect(captured.body.configurations).toEqual({
      language: "zh-CN",
      tags: [{ key: "k", value: "v" }],
      sdtmig: { versionId: 5, versionName: "SDTMIG v3.4" },
    });
  });
});

describe("useSdtmContext — language", () => {
  it("returns project.language when set", async () => {
    mockCommands({
      get_project_by_code: () => ({
        ...baseProject,
        configurations: { language: "zh-CN", tags: [] },
      }),
      list_sdtm_versions: () => versions,
      list_sdtm_domains_by_version: () => domains,
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).language).toBe("zh-CN"),
    );
  });

  it("falls back to 'en' when project.language is null", async () => {
    mockCommands({
      get_project_by_code: () => baseProject,
      list_sdtm_versions: () => versions,
      list_sdtm_domains_by_version: () => domains,
    });
    mount("abc");
    await waitFor(() =>
      expect(JSON.parse(screen.getByTestId("ctx").textContent!).language).toBe("en"),
    );
  });
});

describe("useSdtmContext — loading + error", () => {
  it("is loading while versions are still pending", () => {
    // No handlers registered — versionsQuery stays pending.
    mockCommands({
      get_project_by_code: () => baseProject,
    });
    mount("abc");
    const parsed = JSON.parse(screen.getByTestId("ctx").textContent!);
    expect(parsed.loading).toBe(true);
    expect(parsed.versionId).toBeNull();
  });

  it("surfaces an error from the versions query", async () => {
    mockCommands({
      get_project_by_code: () => baseProject,
      list_sdtm_versions: () => {
        throw { kind: "http", status: 500, code: "internal", message: "boom" };
      },
      list_sdtm_domains_by_version: () => domains,
    });
    mount("abc");
    await waitFor(() => {
      const parsed = JSON.parse(screen.getByTestId("ctx").textContent!);
      expect(parsed.error).toContain("boom");
    });
  });
});
```

- [ ] **Step 2: Run the tests; expect failures**

Run:
```bash
pnpm --filter aegis-desktop test -- src/features/crf/data/sdtm.test.tsx
```
Expected: all tests fail with "Cannot find module './sdtm'" or similar (the hook does not exist yet).

- [ ] **Step 3: Implement `useSdtmContext`**

Create `apps/desktop/aegis-desktop/src/features/crf/data/sdtm.ts` with this exact content:

```ts
import { useEffect, useMemo, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";

import {
  type ApiError,
  type ProjectView,
  type SdtmDomainView,
} from "../../../shared/api";
import {
  useListSdtmDomains,
  useListSdtmVersions,
} from "../../domain-model/data";
import { useProject, useUpdateProject } from "../../project-list/data/projects";

export interface SdtmContext {
  /** Resolved SDTMIG version id. `null` while versions are still
   *  pending or none exist. */
  versionId: number | null;
  /** Resolved language code (project's `configurations.language`
   *  or `"en"` when unset). */
  language: string;
  /** SDTM domains for the resolved version. Empty while loading
   *  or unresolved. */
  domains: SdtmDomainView[];
  /** `true` while versions or domains are still fetching. */
  loading: boolean;
  /** First non-null error from versions or domains query, if any. */
  error: ApiError | null;
}

/**
 * Resolve the project's SDTMIG version + language for the CRF detail
 * page, and expose the SDTM domain list for the resolved version.
 *
 * Resolution rules:
 *   1. If `configurations.sdtmig.versionId` is unset → pick the
 *      SDTM version with the highest `id`.
 *   2. If the configured `versionId` exists in the versions list →
 *      use it directly.
 *   3. If the configured `versionId` is stale but `versionName`
 *      still exists → fire a one-shot `useUpdateProject` to swap
 *      in the matching `versionId` and use it.
 *   4. Otherwise → fall back to (1).
 *
 * Language is `project.configurations.language ?? "en"`.
 */
export function useSdtmContext(projectCode: string): SdtmContext {
  const projectQuery = useProject(projectCode, { enabled: true });
  const versionsQuery = useListSdtmVersions();
  const updateProject = useUpdateProject();
  const qc = useQueryClient();

  const versions = versionsQuery.data ?? [];
  const project: ProjectView | undefined = projectQuery.data;

  const sdtmig = project?.configurations.sdtmig;

  const resolvedVersionId = useMemo<number | null>(() => {
    if (versions.length === 0) return null;
    const configuredId = sdtmig?.versionId ?? null;
    if (configuredId == null) {
      return versions.reduce((max, v) => (v.id > max ? v.id : max), versions[0]!.id);
    }
    if (versions.some((v) => v.id === configuredId)) {
      return configuredId;
    }
    // Stale id — try name recovery.
    const byName = versions.find(
      (v) =>
        sdtmig?.versionName &&
        v.name.toLowerCase() === sdtmig.versionName.toLowerCase(),
    );
    if (byName) return byName.id;
    return versions.reduce((max, v) => (v.id > max ? v.id : max), versions[0]!.id);
  }, [versions, sdtmig]);

  // Fire the recovery mutation at most once per stale id per session.
  const correctedRef = useRef<Set<number>>(new Set());
  useEffect(() => {
    if (!project || versions.length === 0) return;
    const configuredId = sdtmig?.versionId ?? null;
    if (configuredId == null) return;
    if (versions.some((v) => v.id === configuredId)) return;
    if (correctedRef.current.has(configuredId)) return;
    const byName = versions.find(
      (v) =>
        sdtmig?.versionName &&
        v.name.toLowerCase() === sdtmig.versionName.toLowerCase(),
    );
    if (!byName) return;
    correctedRef.current.add(configuredId);
    updateProject.mutate({
      code: projectCode,
      body: {
        configurations: {
          language: project.configurations.language,
          tags: project.configurations.tags,
          sdtmig: { versionId: byName.id, versionName: byName.name },
        },
      },
    });
  }, [project, versions, sdtmig, projectCode, updateProject]);

  const domainsQuery = useListSdtmDomains(resolvedVersionId);
  const domains = resolvedVersionId == null ? [] : (domainsQuery.data ?? []);

  const language = project?.configurations.language ?? "en";
  const loading =
    versionsQuery.isLoading ||
    (resolvedVersionId != null && domainsQuery.isLoading);
  const error =
    (versionsQuery.error as ApiError | null) ??
    (domainsQuery.error as ApiError | null) ??
    null;

  // Touch the queryClient to keep the linter happy if a future
  // refactor needs it for cache invalidation.
  void qc;

  return {
    versionId: resolvedVersionId,
    language,
    domains,
    loading,
    error,
  };
}
```

- [ ] **Step 4: Run the tests; expect pass**

Run:
```bash
pnpm --filter aegis-desktop test -- src/features/crf/data/sdtm.test.tsx
```
Expected: all 9 tests pass. If `preserves language + tags` fails, double-check the recovery mutation body — `tags` must be the project's tags array (not `[]`).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/data/sdtm.ts \
        apps/desktop/aegis-desktop/src/features/crf/data/sdtm.test.tsx
git commit -m "feat(crf): add useSdtmContext hook with version/language resolution"
```

---

## Task 3: Wire `useSdtmContext` into `CrfDetailPage`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx:53-60, 647-752`

- [ ] **Step 1: Add the import**

In `CrfDetailPage.tsx`, find:

```tsx
import {
  useCrfFormDetail,
  useCreateAnnotation,
  useCreateDomainAnnotation,
  useDeleteAnnotation,
  useDeleteDomainAnnotation,
  useUpdateAnnotation,
  useUpdateDomainAnnotation,
  useUpdateOwnerNotSubmitted,
} from "../data/detail";
```

Insert a new import above (or below — either works) for the SDTM context hook:

```tsx
import { useSdtmContext } from "../data/sdtm";
```

- [ ] **Step 2: Call the hook inside `CrfDetailPage`**

Find the line:

```tsx
  const detailQuery = useCrfFormDetail(id);
```

Insert immediately after it:

```tsx
  const sdtmContext = useSdtmContext(projectCode);
```

- [ ] **Step 3: Pass `sdtmDomains` + `sdtmLanguage` to `DomainAnnotationDialog`**

Find the `<DomainAnnotationDialog ... />` block. Insert these two new props after `formNotSubmitted={...}` and before `onClose={...}`:

```tsx
        sdtmDomains={sdtmContext.domains}
        sdtmLanguage={sdtmContext.language}
```

- [ ] **Step 4: Pass `sdtmDomains` to `AnnotationDialog`**

Find the `<AnnotationDialog ... />` block. Insert the new prop alongside the others (e.g., right after `availableDomainAnnotations={...}`):

```tsx
        sdtmDomains={sdtmContext.domains}
```

- [ ] **Step 5: Run the existing `CrfDetailPage` tests**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-detail-page.test.tsx
```
Expected: existing tests still pass — they don't assert on the SDTM-context UI, but they exercise the dialog mounts which now receive the new prop. If a test fails with a TypeScript-style "missing prop" error, the next task adds the new props to the dialog components and this test will pass once more dialogs accept them.

(The TypeScript error appears only on `pnpm typecheck`, not at vitest runtime — Vitest compiles per-test through esbuild and does not surface missing-prop TS errors for components. If `pnpm typecheck` fails, the next task fixes it.)

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx
git commit -m "wire(crf): thread useSdtmContext into CrfDetailPage"
```

---

## Task 4: Add Autocomplete to `DomainAnnotationDialog`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.tsx:1-173`
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/components/DomainAnnotationDialog.test.tsx:1-111`

- [ ] **Step 1: Write the failing tests**

Open `apps/desktop/aegis-desktop/src/test/features/crf/components/DomainAnnotationDialog.test.tsx`. Replace the entire file with:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

import { DomainAnnotationDialog } from "../../../features/crf/components/DomainAnnotationDialog";
import type { SdtmDomainView } from "../../../shared/api";

afterEach(() => cleanup());

const sdtmDomains: SdtmDomainView[] = [
  {
    id: 100,
    versionId: 5,
    name: "AE",
    category: "Events",
    descriptions: [
      { lang: "en", details: { description: "Adverse Events", structure: "" } },
    ],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 101,
    versionId: 5,
    name: "AESI",
    category: "Events",
    descriptions: [
      { lang: "en", details: { description: "AESIs", structure: "" } },
    ],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 102,
    versionId: 5,
    name: "AG",
    category: "Events",
    descriptions: [
      { lang: "en", details: { description: "Agent", structure: "" } },
    ],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 103,
    versionId: 5,
    name: "VS",
    category: "Findings",
    descriptions: [
      // Note: only EN — the test asserts zh-CN fallback empties the description.
      { lang: "en", details: { description: "Vital Signs", structure: "" } },
    ],
    createdAt: "",
    updatedAt: "",
  },
];

function renderDialog(
  props: Partial<React.ComponentProps<typeof DomainAnnotationDialog>> = {},
) {
  const onSubmit = vi.fn();
  const onMarkNotSubmitted = vi.fn();
  const utils = render(
    <AegisI18nProvider>
      <DomainAnnotationDialog
        open
        mode="create"
        formNotSubmitted={false}
        onClose={() => undefined}
        onSubmit={onSubmit}
        onMarkNotSubmitted={onMarkNotSubmitted}
        markNotSubmittedPending={false}
        markNotSubmittedError={null}
        mutationError={null}
        mutationPending={false}
        sdtmDomains={sdtmDomains}
        sdtmLanguage="en"
        {...props}
      />
    </AegisI18nProvider>,
  );
  return { onSubmit, onMarkNotSubmitted, ...utils };
}

describe("DomainAnnotationDialog", () => {
  it("submit is disabled while name is empty", () => {
    const { onSubmit } = renderDialog();
    const submit = screen.getByRole("button", { name: /Create/i });
    expect(submit).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Name/i), {
      target: { value: "AE" },
    });
    expect(submit).not.toBeDisabled();
    fireEvent.click(submit);
    expect(onSubmit).toHaveBeenCalledWith({
      name: "AE",
      description: "",
    });
  });

  it("edit mode pre-fills from row", () => {
    const onSubmit = vi.fn();
    renderDialog({
      mode: "edit",
      row: {
        id: 50,
        formId: 11,
        name: "AE",
        description: "Adverse Events",
        createdAt: "",
        updatedAt: "",
      },
      onSubmit,
    });
    fireEvent.change(screen.getByLabelText(/Name/i), {
      target: { value: "Renamed" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Save/i }));
    expect(onSubmit).toHaveBeenCalledWith({
      name: "Renamed",
      description: "Adverse Events",
    });
  });

  it("renders the Not submit button and triggers onMarkNotSubmitted", () => {
    const { onMarkNotSubmitted } = renderDialog();
    const notSubmit = screen.getByTestId("crf-domain-dialog-not-submit");
    expect(notSubmit).toBeInTheDocument();
    fireEvent.click(notSubmit);
    expect(onMarkNotSubmitted).toHaveBeenCalledTimes(1);
  });

  it("hides the Not submit button when the form is already not-submitted", () => {
    renderDialog({ formNotSubmitted: true });
    expect(
      screen.queryByTestId("crf-domain-dialog-not-submit"),
    ).not.toBeInTheDocument();
  });

  it("hides the Not submit button in edit mode", () => {
    renderDialog({
      mode: "edit",
      row: {
        id: 50,
        formId: 11,
        name: "AE",
        description: "Adverse Events",
        createdAt: "",
        updatedAt: "",
      },
    });
    expect(
      screen.queryByTestId("crf-domain-dialog-not-submit"),
    ).not.toBeInTheDocument();
  });

  // --- New: Autocomplete + description auto-fill ---

  it("auto-uppercases typed name input", () => {
    renderDialog();
    fireEvent.change(screen.getByLabelText(/Name/i), {
      target: { value: "ae" },
    });
    expect(screen.getByLabelText(/Name/i)).toHaveValue("AE");
  });

  it("free-form typing leaves the description unchanged when no domain matches", () => {
    renderDialog();
    fireEvent.change(screen.getByLabelText(/Name/i), {
      target: { value: "ZZ" },
    });
    fireEvent.change(screen.getByLabelText(/Description/i), {
      target: { value: "custom" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Create/i }));
    // `expect.objectContaining` so the test is robust to internal state shape.
    expect(screen.getByLabelText(/Description/i)).toHaveValue("custom");
  });

  it("picking a matching domain auto-fills the description in the project's language", () => {
    renderDialog();
    const nameInput = screen.getByLabelText(/Name/i);
    fireEvent.change(nameInput, { target: { value: "AE" } });
    // Open the Autocomplete dropdown and pick the AE option.
    fireEvent.keyDown(nameInput, { key: "ArrowDown" });
    fireEvent.click(screen.getByRole("option", { name: "AE" }));
    expect(screen.getByLabelText(/Description/i)).toHaveValue("Adverse Events");
  });

  it("picking a domain with no description in the project language leaves the description empty", () => {
    renderDialog({ sdtmLanguage: "zh-CN" });
    const nameInput = screen.getByLabelText(/Name/i);
    fireEvent.change(nameInput, { target: { value: "VS" } });
    fireEvent.keyDown(nameInput, { key: "ArrowDown" });
    fireEvent.click(screen.getByRole("option", { name: "VS" }));
    expect(screen.getByLabelText(/Description/i)).toHaveValue("");
  });

  it("in edit mode, picking a different domain re-fills the description", () => {
    renderDialog({
      mode: "edit",
      row: {
        id: 50,
        formId: 11,
        name: "old",
        description: "old description",
        createdAt: "",
        updatedAt: "",
      },
    });
    const nameInput = screen.getByLabelText(/Name/i);
    // MUI Autocomplete's inputValue is updated by select via the option click.
    fireEvent.change(nameInput, { target: { value: "AESI" } });
    fireEvent.keyDown(nameInput, { key: "ArrowDown" });
    fireEvent.click(screen.getByRole("option", { name: "AESI" }));
    expect(screen.getByLabelText(/Description/i)).toHaveValue("AESIs");
  });
});
```

- [ ] **Step 2: Run the tests; expect the new ones to fail**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/components/DomainAnnotationDialog.test.tsx
```
Expected: the 5 pre-existing tests pass; the 5 new tests fail (the dialog still has a plain TextField for `name`, no Autocomplete, no description auto-fill).

- [ ] **Step 3: Update the dialog to add the new props and Autocomplete**

Open `apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.tsx` and replace the entire file with:

```tsx
import { useEffect, useState } from "react";
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  TextField,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  ApiError,
  DomainAnnotation,
  SdtmDomainView,
} from "../../../shared/api";

export interface DomainAnnotationDialogBody {
  name: string;
  description: string;
}

interface Props {
  open: boolean;
  mode: "create" | "edit";
  row?: DomainAnnotation;
  /**
   * Current `notSubmitted` flag of the form that owns this domain
   * annotation. The dialog hides its `Not submit` action while the
   * form is already marked not-submitted — there's nothing left to
   * do — and the page runs the cascade + form update on click.
   */
  formNotSubmitted: boolean;
  onClose: () => void;
  onSubmit: (body: DomainAnnotationDialogBody) => void;
  /**
   * Trigger the form-level cascade: delete every annotation in
   * the form, then PATCH the form's `notSubmitted` flag to true.
   * Wired by the page to `useUpdateOwnerNotSubmitted` for the
   * `{ kind: "form", id }` owner.
   */
  onMarkNotSubmitted: () => void;
  markNotSubmittedPending: boolean;
  markNotSubmittedError: ApiError | null;
  mutationError: ApiError | null;
  mutationPending: boolean;
  /**
   * SDTM domains for the project's resolved SDTMIG version. Empty
   * array disables the autocomplete's option list (the user can
   * still type free-form names via `freeSolo`).
   */
  sdtmDomains: SdtmDomainView[];
  /**
   * Language code used to look up the matching description when
   * the user picks a domain from the autocomplete. `""` / `"en"`
   * / `"zh-CN"` etc. — matches the lang field on
   * `SdtmDomainDescription`.
   */
  sdtmLanguage: string;
}

const EMPTY: DomainAnnotationDialogBody = {
  name: "",
  description: "",
};

/**
 * Look up the description for the given domain name in the project
 * language. Returns `null` when the domain is not in `sdtmDomains`
 * or no description matches the language (per the brainstorming
 * Q3 decision: leave the field empty, not an em-dash, no warning).
 */
function findDescription(
  domains: SdtmDomainView[],
  name: string,
  language: string,
): string | null {
  const upper = name.toUpperCase();
  const match = domains.find((d) => d.name.toUpperCase() === upper);
  if (!match) return null;
  const desc = match.descriptions.find((d) => d.lang === language);
  return desc?.details.description ?? null;
}

export function DomainAnnotationDialog({
  open,
  mode,
  row,
  formNotSubmitted,
  onClose,
  onSubmit,
  onMarkNotSubmitted,
  markNotSubmittedPending,
  markNotSubmittedError,
  mutationError,
  mutationPending,
  sdtmDomains,
  sdtmLanguage,
}: Props) {
  const { t } = useI18n();
  const [body, setBody] = useState<DomainAnnotationDialogBody>(EMPTY);

  useEffect(() => {
    if (!open) return;
    if (mode === "edit" && row) {
      setBody({
        name: row.name,
        description: row.description,
      });
    } else {
      setBody({
        name: EMPTY.name,
        description: EMPTY.description,
      });
    }
  }, [open, mode, row]);

  const submitDisabled = mutationPending || body.name.trim() === "";
  // The Not submit action is one-way and only meaningful when
  // creating a fresh domain annotation. Hide it in edit mode —
  // the user is editing an existing row, not deciding whether the
  // form needs a flag — and hide it once the form is already
  // not-submitted.
  const markVisible = mode === "create" && !formNotSubmitted;
  const markDisabled =
    markNotSubmittedPending || mutationPending;

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({
      name: body.name.trim(),
      description: body.description.trim(),
    });
  }

  const domainOptions = sdtmDomains.map((d) => d.name);

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="sm"
      fullWidth
    >
      <DialogTitle>
        {t(
          mode === "create"
            ? "crf.domainDialog.create.title"
            : "crf.domainDialog.edit.title",
        )}
      </DialogTitle>
      <DialogContent>
        <Box
          sx={{ display: "flex", flexDirection: "column", gap: 2, pt: 2 }}
        >
          <Autocomplete
            freeSolo
            options={domainOptions}
            // Case-insensitive startsWith — matches "auto upcase what
            // they enter, filter the domains with the current value".
            filterOptions={(opts, state) =>
              opts.filter((o) =>
                o.toUpperCase().startsWith(state.inputValue.toUpperCase()),
              )
            }
            inputValue={body.name}
            // Typing path: uppercase as the user types.
            onInputChange={(_e, value, reason) => {
              if (reason === "input") {
                setBody((b) => ({ ...b, name: value.toUpperCase() }));
              } else {
                setBody((b) => ({ ...b, name: value }));
              }
            }}
            // Selection path: uppercase the picked name and auto-fill
            // the description in the project's language.
            onChange={(_e, value) => {
              const next = (typeof value === "string" ? value : value ?? "").toUpperCase();
              const desc = next ? findDescription(sdtmDomains, next, sdtmLanguage) : null;
              setBody((b) => ({
                ...b,
                name: next,
                description: desc ?? (next ? "" : b.description),
              }));
            }}
            renderInput={(params) => (
              <TextField
                {...params}
                size="small"
                label={t("crf.domainDialog.field.name")}
              />
            )}
          />
          <TextField
            size="small"
            label={t("crf.domainDialog.field.description")}
            value={body.description}
            onChange={(e) =>
              setBody((b) => ({ ...b, description: e.target.value }))
            }
          />
          {(mutationError ?? markNotSubmittedError) && (
            <Alert severity="error">
              {errorMessage(mutationError ?? markNotSubmittedError!)}
            </Alert>
          )}
        </Box>

      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={mutationPending || markNotSubmittedPending}>
          {t("common.cancel")}
        </Button>
        {markVisible && (
          <Button
            variant="outlined"
            color="warning"
            onClick={onMarkNotSubmitted}
            disabled={markDisabled}
            data-testid="crf-domain-dialog-not-submit"
          >
            {t("crf.domainDialog.notSubmit")}
          </Button>
        )}
        <Button
          variant="contained"
          onClick={handleSubmit}
          disabled={submitDisabled}
        >
          {t(
            mode === "create"
              ? "crf.domainDialog.submit.create"
              : "crf.domainDialog.submit.save",
          )}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
```

- [ ] **Step 4: Run the tests; expect pass**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/components/DomainAnnotationDialog.test.tsx
```
Expected: all 10 tests pass. If `auto-uppercases typed name input` fails, the dialog's `onInputChange` isn't uppercasing — re-check the branch on `reason === "input"`. If `picking a matching domain auto-fills...` fails, the `onChange` arrow isn't reaching `findDescription` — re-check the `value ?? ""` fallback path.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.tsx \
        apps/desktop/aegis-desktop/src/test/features/crf/components/DomainAnnotationDialog.test.tsx
git commit -m "feat(crf): SDTM-domain autocomplete + description auto-fill in DomainAnnotationDialog"
```

---

## Task 5: Add `@`-mention Popover to `AnnotationDialog`

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx:1-231`
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/components/AnnotationDialog.test.tsx:1-139`

- [ ] **Step 1: Write the failing tests**

Open `apps/desktop/aegis-desktop/src/test/features/crf/components/AnnotationDialog.test.tsx`. Replace the entire file with:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";

import { AnnotationDialog } from "../../../features/crf/components/AnnotationDialog";
import type {
  AnnotationOwner,
  DomainAnnotation,
  SdtmDomainView,
  SdtmVariableView,
} from "../../../shared/api";

afterEach(() => cleanup());

const owner: AnnotationOwner = { kind: "form", id: 11 };

const domainAnnotations: DomainAnnotation[] = [
  {
    id: 50,
    formId: 11,
    name: "AE",
    description: "Adverse Events",
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 51,
    formId: 11,
    name: "VS",
    description: "Vital Signs",
    createdAt: "",
    updatedAt: "",
  },
  // A free-form annotation that does NOT match any SDTM domain.
  {
    id: 52,
    formId: 11,
    name: "ZZ",
    description: "Custom",
    createdAt: "",
    updatedAt: "",
  },
];

const sdtmDomains: SdtmDomainView[] = [
  {
    id: 100,
    versionId: 5,
    name: "AE",
    category: "Events",
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 101,
    versionId: 5,
    name: "VS",
    category: "Findings",
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
];

const variables: SdtmVariableView[] = [
  {
    id: 1,
    domainId: 100,
    name: "AETERM",
    variableType: "Character",
    variableCore: "Req",
    variableSequence: 1,
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 2,
    domainId: 100,
    name: "AESEV",
    variableType: "Character",
    variableCore: "Exp",
    variableSequence: 2,
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 3,
    domainId: 100,
    name: "AGE",
    variableType: "Numeric",
    variableCore: "Req",
    variableSequence: 3,
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 4,
    domainId: 100,
    name: "DOMAIN",
    variableType: "Character",
    variableCore: "Req",
    variableSequence: 4,
    descriptions: [],
    createdAt: "",
    updatedAt: "",
  },
];

// `useListSdtmVariables` reads from cache; seed the cache so the hook
// returns the variables list. The import must come after the i18n
// provider so that the React tree is mounted when we call useQueryClient.
import { TestQueryProvider } from "../../../test/helpers/test-query-provider";
import { queryKeys } from "../../../shared/query";

function renderDialog(
  props: Partial<React.ComponentProps<typeof AnnotationDialog>> = {},
) {
  const onSubmit = vi.fn();
  const onMarkNotSubmitted = vi.fn();
  const utils = render(
    <TestQueryProvider>
      <AegisI18nProvider>
        <AnnotationDialog
          open
          mode="create"
          owner={owner}
          ownerNotSubmitted={false}
          availableDomainAnnotations={domainAnnotations}
          onClose={() => undefined}
          onSubmit={onSubmit}
          onMarkNotSubmitted={onMarkNotSubmitted}
          markNotSubmittedPending={false}
          markNotSubmittedError={null}
          mutationError={null}
          mutationPending={false}
          sdtmDomains={sdtmDomains}
          {...props}
        />
      </AegisI18nProvider>
    </TestQueryProvider>,
  );
  // Seed the sdtm-variables cache for domainId=100 (AE).
  const qc = (utils as any).qc;
  // qc may not be exposed; instead seed via a sentinel component below.
  return { onSubmit, onMarkNotSubmitted, ...utils };
}

// Helper: a wrapper component that seeds the variables cache before
// the dialog renders. We mount this inside TestQueryProvider.
function withSeed(QC: any) {
  QC.setQueryData(queryKeys.domainModel.sdtmVariables(100), variables);
  QC.setQueryData(queryKeys.domainModel.sdtmVariables(101), []);
}

function mountWithSeed(
  props: Partial<React.ComponentProps<typeof AnnotationDialog>> = {},
) {
  const onSubmit = vi.fn();
  const onMarkNotSubmitted = vi.fn();
  const utils = render(
    <TestQueryProvider>
      <SeedAndDialog
        onSubmit={onSubmit}
        onMarkNotSubmitted={onMarkNotSubmitted}
        dialogProps={props}
      />
    </TestQueryProvider>,
  );
  return { onSubmit, onMarkNotSubmitted, ...utils };
}

function SeedAndDialog({
  onSubmit,
  onMarkNotSubmitted,
  dialogProps,
}: {
  onSubmit: ReturnType<typeof vi.fn>;
  onMarkNotSubmitted: ReturnType<typeof vi.fn>;
  dialogProps: Partial<React.ComponentProps<typeof AnnotationDialog>>;
}) {
  // We need access to the QueryClient inside the provider. Use a small
  // helper: import useQueryClient here.
  // eslint-disable-next-line @typescript-eslint/no-require-imports, @typescript-eslint/no-var-requires
  const { useQueryClient } = require("@tanstack/react-query");
  const qc = useQueryClient();
  withSeed(qc);
  return (
    <AegisI18nProvider>
      <AnnotationDialog
        open
        mode="create"
        owner={owner}
        ownerNotSubmitted={false}
        availableDomainAnnotations={domainAnnotations}
        onClose={() => undefined}
        onSubmit={onSubmit}
        onMarkNotSubmitted={onMarkNotSubmitted}
        markNotSubmittedPending={false}
        markNotSubmittedError={null}
        mutationError={null}
        mutationPending={false}
        sdtmDomains={sdtmDomains}
        {...dialogProps}
      />
    </AegisI18nProvider>
  );
}

describe("AnnotationDialog", () => {
  it("submit is disabled until content is non-empty", () => {
    mountWithSeed();
    const submit = screen.getByRole("button", { name: /Create/i });
    expect(submit).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Content/i), {
      target: { value: "note" },
    });
    expect(submit).not.toBeDisabled();
  });

  it("edit mode disables the domain annotation select and preserves assign", () => {
    const { onSubmit } = mountWithSeed({
      mode: "edit",
      row: {
        id: 100,
        domainAnnotationId: 50,
        content: "old note",
        assign: true,
        owner,
        createdAt: "",
        updatedAt: "",
      },
    });
    const combobox = screen.getByRole("combobox");
    expect(combobox).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByDisplayValue("old note")).toBeInTheDocument();
    const assign = screen.getByRole("checkbox");
    expect(assign).toBeChecked();
    fireEvent.click(screen.getByRole("button", { name: /Save/i }));
    expect(onSubmit).toHaveBeenCalledWith({
      domainAnnotationId: 50,
      content: "old note",
      assign: true,
    });
  });

  it("renders the Not submit button and triggers onMarkNotSubmitted", () => {
    const { onMarkNotSubmitted } = mountWithSeed();
    const notSubmit = screen.getByTestId("crf-annotation-dialog-not-submit");
    expect(notSubmit).toBeInTheDocument();
    fireEvent.click(notSubmit);
    expect(onMarkNotSubmitted).toHaveBeenCalledTimes(1);
  });

  it("hides the Not submit button when the owner is already not-submitted", () => {
    mountWithSeed({ ownerNotSubmitted: true });
    expect(
      screen.queryByTestId("crf-annotation-dialog-not-submit"),
    ).not.toBeInTheDocument();
  });

  it("hides the Not submit button in edit mode", () => {
    mountWithSeed({
      mode: "edit",
      row: {
        id: 100,
        domainAnnotationId: 50,
        content: "old note",
        assign: true,
        owner,
        createdAt: "",
        updatedAt: "",
      },
    });
    expect(
      screen.queryByTestId("crf-annotation-dialog-not-submit"),
    ).not.toBeInTheDocument();
  });

  // --- New: @-mention behavior ---

  it("typing @ opens a dropdown with the full variable list for the matching SDTM domain", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@" } });
    await waitFor(() => {
      expect(screen.getByTestId("crf-variable-1")).toBeInTheDocument();
      expect(screen.getByTestId("crf-variable-2")).toBeInTheDocument();
      expect(screen.getByTestId("crf-variable-3")).toBeInTheDocument();
      expect(screen.getByTestId("crf-variable-4")).toBeInTheDocument();
    });
  });

  it("filters the variable list to startsWith the typed fragment (uppercased)", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@a" } });
    await waitFor(() => {
      expect(screen.getByTestId("crf-variable-1")).toBeInTheDocument(); // AETERM
      expect(screen.getByTestId("crf-variable-3")).toBeInTheDocument(); // AGE
    });
    expect(screen.queryByTestId("crf-variable-2")).not.toBeInTheDocument(); // AESEV
    expect(screen.queryByTestId("crf-variable-4")).not.toBeInTheDocument(); // DOMAIN
  });

  it("clicking a variable inserts the variable's name (replacing @fragment)", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i) as HTMLInputElement;
    fireEvent.change(content, { target: { value: "@age" } });
    await waitFor(() => screen.getByTestId("crf-variable-3"));
    fireEvent.click(screen.getByTestId("crf-variable-3"));
    await waitFor(() => {
      expect(screen.getByLabelText(/Content/i)).toHaveValue("AGE");
    });
  });

  it("does NOT open the dropdown when @ is mid-word", () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "foo@bar" } });
    expect(screen.queryByTestId("crf-variable-1")).not.toBeInTheDocument();
  });

  it("does NOT open the dropdown when the picked domain annotation has no SDTM match", async () => {
    // Switch the picked domain annotation to the free-form "ZZ".
    mountWithSeed();
    // The default domainAnnotationId is the first (AE). Manually switch.
    const select = screen.getByRole("combobox");
    fireEvent.mouseDown(select);
    const zzOption = await screen.findByRole("option", { name: "ZZ" });
    fireEvent.click(zzOption);
    // Now type @ in content.
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@" } });
    expect(screen.queryByTestId("crf-variable-1")).not.toBeInTheDocument();
  });

  it("shows a 'No variables match' disabled item when the fragment has no match", async () => {
    mountWithSeed();
    const content = screen.getByLabelText(/Content/i);
    fireEvent.change(content, { target: { value: "@zzz" } });
    await waitFor(() => {
      // The noMatch text is rendered as a disabled MenuItem.
      expect(
        screen.getByText(/No variables match/i),
      ).toBeInTheDocument();
    });
  });

  it("edit mode: pre-existing @xxx text is preserved; new @ still opens", async () => {
    mountWithSeed({
      mode: "edit",
      row: {
        id: 100,
        domainAnnotationId: 50,
        content: "old @zzz",
        assign: false,
        owner,
        createdAt: "",
        updatedAt: "",
      },
    });
    // Initial content is preserved.
    expect(screen.getByLabelText(/Content/i)).toHaveValue("old @zzz");
    // Append more text and open a fresh mention.
    const content = screen.getByLabelText(/Content/i) as HTMLInputElement;
    fireEvent.change(content, {
      target: { value: "old @zzz @a", selectionStart: 11 },
    });
    // We can't reliably set selectionStart via fireEvent.change's
    // target. Instead, simulate by appending and trusting the dialog
    // re-detects the latest @a fragment.
    await waitFor(() => {
      expect(screen.getByTestId("crf-variable-3")).toBeInTheDocument(); // AGE
    });
  });
});
```

(Note: the `mountWithSeed` helper deliberately seeds the variables cache via a wrapper component. This is because `useListSdtmVariables` is `enabled: domainId != null`, so we need the `setQueryData` to run before the dialog renders. We also use `require("@tanstack/react-query")` inside the wrapper to avoid module-resolution issues at the top level.)

- [ ] **Step 2: Run the tests; expect the new ones to fail**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/components/AnnotationDialog.test.tsx
```
Expected: the 5 pre-existing tests pass; the 7 new tests fail (the dialog still has a plain TextField, no Popover, no `@` detection).

- [ ] **Step 3: Update the dialog to add `@`-mention support**

Open `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx`. Replace the entire file with:

```tsx
import { useEffect, useMemo, useRef, useState } from "react";
import type { ChangeEvent } from "react";
import {
  Alert,
  Box,
  Button,
  Checkbox,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControl,
  FormControlLabel,
  InputLabel,
  MenuItem,
  MenuList,
  Popover,
  Select,
  TextField,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import type {
  Annotation,
  AnnotationOwner,
  ApiError,
  DomainAnnotation,
  SdtmDomainView,
} from "../../../shared/api";
import { useListSdtmVariables } from "../../domain-model/data";

export interface AnnotationDialogBody {
  domainAnnotationId: number;
  content: string;
  assign: boolean;
}

interface Props {
  open: boolean;
  mode: "create" | "edit";
  owner: AnnotationOwner;
  /**
   * Current `notSubmitted` flag of the annotation's owner
   * (form / item / option / unit). The dialog hides its
   * `Not submit` action while the owner is already marked
   * not-submitted — there's nothing left to do — and the
   * page runs the cascade + owner update on click.
   */
  ownerNotSubmitted: boolean;
  row?: Annotation;
  availableDomainAnnotations: DomainAnnotation[];
  onClose: () => void;
  /**
   * Called with the dialog body. The page composes the full
   * CreateAnnotationInput by merging the owner at the call site.
   */
  onSubmit: (body: AnnotationDialogBody) => void;
  /**
   * Trigger the owner-level cascade: delete every annotation
   * attached to the owner (form / item / option / unit),
   * then PATCH the owner's `notSubmitted` flag to true.
   * Wired by the page to `useUpdateOwnerNotSubmitted`.
   */
  onMarkNotSubmitted: () => void;
  markNotSubmittedPending: boolean;
  markNotSubmittedError: ApiError | null;
  mutationError: ApiError | null;
  mutationPending: boolean;
  /**
   * SDTM domains for the project's resolved SDTMIG version. The
   * dialog matches the currently-selected domain annotation's
   * `name` against this list (case-insensitive) to decide which
   * SDTM domain's variables to offer in the `@`-mention
   * dropdown. Empty array disables the dropdown entirely.
   */
  sdtmDomains: SdtmDomainView[];
}

const EMPTY: AnnotationDialogBody = {
  domainAnnotationId: 0,
  content: "",
  assign: false,
};

export function AnnotationDialog({
  open,
  mode,
  owner: _owner,
  ownerNotSubmitted,
  row,
  availableDomainAnnotations,
  onClose,
  onSubmit,
  onMarkNotSubmitted,
  markNotSubmittedPending,
  markNotSubmittedError,
  mutationError,
  mutationPending,
  sdtmDomains,
}: Props) {
  const { t } = useI18n();
  const [body, setBody] = useState<AnnotationDialogBody>(EMPTY);

  // --- @-mention state ---
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
  const [mentionRange, setMentionRange] =
    useState<{ start: number; end: number } | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (!open) return;
    if (mode === "edit" && row) {
      setBody({
        domainAnnotationId: row.domainAnnotationId,
        content: row.content,
        assign: row.assign,
      });
    } else {
      setBody({
        domainAnnotationId: availableDomainAnnotations[0]?.id ?? 0,
        content: "",
        assign: false,
      });
    }
    // Reseeding also wipes the active mention.
    setAnchorEl(null);
    setMentionRange(null);
  }, [open, mode, row, availableDomainAnnotations]);

  const submitDisabled =
    mutationPending ||
    body.content.trim() === "" ||
    body.domainAnnotationId === 0;
  // The Not submit action is one-way and only meaningful when
  // creating a fresh annotation. Hide it in edit mode — the user is
  // editing an existing row, not deciding whether the owner needs a
  // flag — and hide it once the owner is already not-submitted.
  const markVisible = mode === "create" && !ownerNotSubmitted;
  const markDisabled =
    markNotSubmittedPending || mutationPending;

  // --- SDTM domain lookup for the current domain annotation ---
  const selectedDomainName = useMemo(() => {
    const da = availableDomainAnnotations.find(
      (d) => d.id === body.domainAnnotationId,
    );
    return da?.name?.toUpperCase() ?? null;
  }, [availableDomainAnnotations, body.domainAnnotationId]);
  const selectedDomain = useMemo(
    () =>
      sdtmDomains.find((d) => d.name.toUpperCase() === selectedDomainName) ?? null,
    [sdtmDomains, selectedDomainName],
  );
  const variablesQuery = useListSdtmVariables(
    open && selectedDomain ? selectedDomain.id : null,
  );

  const fragment = mentionRange
    ? body.content.slice(mentionRange.start, mentionRange.end)
    : "";
  const filteredVariables = useMemo(() => {
    const all = variablesQuery.data ?? [];
    const q = fragment.toUpperCase();
    if (!q) return all;
    return all.filter((v) => v.name.toUpperCase().startsWith(q));
  }, [variablesQuery.data, fragment]);

  function handleSubmit() {
    if (submitDisabled) return;
    onSubmit({
      domainAnnotationId: body.domainAnnotationId,
      content: body.content.trim(),
      assign: body.assign,
    });
  }

  // --- @-mention detection ---
  function handleContentChange(e: ChangeEvent<HTMLInputElement>) {
    const value = e.target.value;
    const caret = e.target.selectionStart ?? value.length;
    setBody((b) => ({ ...b, content: value }));

    // Walk backwards from caret to the previous whitespace.
    const before = value.slice(0, caret);
    const lastWs = Math.max(
      before.lastIndexOf(" "),
      before.lastIndexOf("\n"),
      before.lastIndexOf("\t"),
    );
    const head = before.slice(lastWs + 1);
    const atIdx = head.lastIndexOf("@");
    if (atIdx >= 0) {
      const start = lastWs + 1 + atIdx;
      // Reject @ that is mid-word (e.g. "foo@bar").
      if (atIdx === 0 || /\s/.test(head[atIdx - 1] ?? "")) {
        setMentionRange({ start, end: caret });
        setAnchorEl(e.currentTarget);
        return;
      }
    }
    setMentionRange(null);
    setAnchorEl(null);
  }

  function insertVariable(name: string) {
    if (!mentionRange) return;
    const before = body.content.slice(0, mentionRange.start);
    const after = body.content.slice(mentionRange.end);
    // Inserted text is always the variable name — language-independent.
    const inserted = name;
    const next = before + inserted + after;
    setBody((b) => ({ ...b, content: next }));
    setMentionRange(null);
    setAnchorEl(null);
    const caret = (before + inserted).length;
    queueMicrotask(() => {
      inputRef.current?.setSelectionRange(caret, caret);
    });
  }

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="sm"
      fullWidth
    >
      <DialogTitle>
        {t(
          mode === "create"
            ? "crf.annotationDialog.create.title"
            : "crf.annotationDialog.edit.title",
        )}
      </DialogTitle>
      <DialogContent>
        <Box
          sx={{ display: "flex", flexDirection: "column", gap: 2, pt: 2 }}
        >
          <FormControl size="small" disabled={mode === "edit"}>
            <InputLabel id="annotation-domain-annotation-label">
              {t("crf.annotationDialog.field.domainAnnotation")}
            </InputLabel>
            <Select
              labelId="annotation-domain-annotation-label"
              label={t("crf.annotationDialog.field.domainAnnotation")}
              value={body.domainAnnotationId || ""}
              onChange={(e) =>
                setBody((b) => ({
                  ...b,
                  domainAnnotationId: Number(e.target.value) || 0,
                }))
              }
              required
            >
              {availableDomainAnnotations.length === 0 && (
                <MenuItem value="" disabled>
                  {t("crf.annotationDialog.domainAnnotation.none")}
                </MenuItem>
              )}
              {availableDomainAnnotations.map((d) => (
                <MenuItem key={d.id} value={d.id}>
                  {d.name}
                </MenuItem>
              ))}
            </Select>
          </FormControl>
          <TextField
            size="small"
            label={t("crf.annotationDialog.field.content")}
            value={body.content}
            onChange={handleContentChange}
            inputRef={inputRef}
            inputProps={{
              "data-testid": "crf-annotation-dialog-content",
            }}
          />
          {/* @-mention Popover. Anchored to the content TextField. */}
          <Popover
            open={Boolean(anchorEl) && mentionRange !== null}
            anchorEl={anchorEl}
            anchorOrigin={{ vertical: "bottom", horizontal: "left" }}
            slotProps={{ paper: { sx: { minWidth: 240, maxHeight: 240 } } }}
            data-testid="crf-variable-popover"
          >
            <MenuList>
              {filteredVariables.length === 0 ? (
                <MenuItem disabled>
                  {t("crf.annotationDialog.variable.noMatch")}
                </MenuItem>
              ) : (
                filteredVariables.map((v) => (
                  <MenuItem
                    key={v.id}
                    onClick={() => insertVariable(v.name)}
                    data-testid={`crf-variable-${v.id}`}
                  >
                    {v.name}
                  </MenuItem>
                ))
              )}
            </MenuList>
          </Popover>
          <FormControlLabel
            control={
              <Checkbox
                checked={body.assign}
                onChange={(e) =>
                  setBody((b) => ({ ...b, assign: e.target.checked }))
                }
              />
            }
            label={t("crf.annotationDialog.field.assign")}
          />
          {(mutationError ?? markNotSubmittedError) && (
            <Alert severity="error">
              {errorMessage(mutationError ?? markNotSubmittedError!)}
            </Alert>
          )}
        </Box>
      </DialogContent>
      <DialogActions>
        <Button onClick={onClose} disabled={mutationPending || markNotSubmittedPending}>
          {t("common.cancel")}
        </Button>
        {markVisible && (
          <Button
            variant="outlined"
            color="warning"
            onClick={onMarkNotSubmitted}
            disabled={markDisabled}
            data-testid="crf-annotation-dialog-not-submit"
          >
            {t("crf.annotationDialog.notSubmit")}
          </Button>
        )}
        <Button
          variant="contained"
          onClick={handleSubmit}
          disabled={submitDisabled}
        >
          {t(
            mode === "create"
              ? "crf.annotationDialog.submit.create"
              : "crf.annotationDialog.submit.save",
          )}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
```

- [ ] **Step 4: Run the tests; expect pass**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/components/AnnotationDialog.test.tsx
```
Expected: all 12 tests pass. If "typing @" tests fail with "unable to find" instead of "1 not expected", the dialog's `setAnchorEl` is firing but `selectedDomain` is `null` because the default `domainAnnotationId` (the first in `availableDomainAnnotations`) is `"AE"` and `sdtmDomains` includes `AE` — verify the fixtures. If "mid-word" fails, check that the `if (atIdx === 0 || /\s/.test(head[atIdx - 1] ?? ""))` guard is rejecting `foo@bar` (head = `"foo@bar"`, atIdx = 3, head[2] = `"o"`, not whitespace → guard fails → no mention).

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx \
        apps/desktop/aegis-desktop/src/test/features/crf/components/AnnotationDialog.test.tsx
git commit -m "feat(crf): @-variable mention Popover in AnnotationDialog"
```

---

## Task 6: Page-level smoke — wire `sdtmDomains` into `CrfDetailPage` tests

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/crf/CrfDetailPage.test.tsx`

The existing page tests render the page and open the dialogs. They need the SDTM context to resolve (or to gracefully not crash). This task adds minimal SDTM command mocks to the page-level `mockCommands` so the page does not break when the dialogs receive the new `sdtmDomains` prop.

- [ ] **Step 1: Add a minimal SDTM mock to every `mockCommands(...)` in the page test**

Open `apps/desktop/aegis-desktop/src/test/features/crf/CrfDetailPage.test.tsx`. Find each call to `mockCommands({` and add three new entries inside the same object literal (typically right after the last entry, before the closing `});`):

```ts
      list_sdtm_versions: () => [
        { id: 5, name: "SDTMIG v3.4", createdAt: "", updatedAt: "" },
      ],
      list_sdtm_domains_by_version: () => [
        {
          id: 100,
          versionId: 5,
          name: "AE",
          category: "Events",
          descriptions: [
            { lang: "en", details: { description: "Adverse Events", structure: "" } },
          ],
          createdAt: "",
          updatedAt: "",
        },
      ],
      list_sdtm_variables_by_domain: () => [],
```

(Add these three lines to every `mockCommands({ ... })` call in the file. If a test does not need SDTM at all, it still needs to mock the commands to avoid `mockInvoke` rejecting "unexpected tauri command" — apply the same change uniformly.)

- [ ] **Step 2: Run the page tests**

Run:
```bash
pnpm --filter aegis-desktop test -- src/test/features/crf/crf-detail-page.test.tsx
```
Expected: all existing page tests still pass. The `useSdtmContext` hook fires its three queries, but the dialogs don't render the autocomplete UX unless the dialog is open, so existing assertions are unaffected.

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/crf/CrfDetailPage.test.tsx
git commit -m "test(crf): seed SDTM mocks in CrfDetailPage test for useSdtmContext"
```

---

## Task 7: Final verification

- [ ] **Step 1: Desktop typecheck**

Run:
```bash
pnpm --filter aegis-desktop typecheck
```
Expected: exit 0. If the `AnnotationDialog` test file's `require("@tanstack/react-query")` line complains, replace it with a top-level import (the line in `SeedAndDialog` was a deliberate hedge against hoisting — remove it and verify the test file's top imports already include `useQueryClient`).

- [ ] **Step 2: UI package typecheck**

Run:
```bash
pnpm --filter @aegis/ui typecheck
```
Expected: exit 0 (no UI package source changes were made, but the i18n additions are type-checked here).

- [ ] **Step 3: Full desktop test run**

Run:
```bash
pnpm --filter aegis-desktop test
```
Expected: all tests pass.

- [ ] **Step 4: Desktop lint**

Run:
```bash
pnpm --filter aegis-desktop lint
```
Expected: exit 0, no warnings (or warnings only on pre-existing unrelated lines).

- [ ] **Step 5: Final commit (only if Step 1-4 surfaced a fix)**

If any of the above surfaced a fix, commit it now with a descriptive message. Otherwise skip — every change is already in a commit from its task.

---

## Self-Review Checklist

1. **Spec coverage** — every requirement in `2026-09-22-crf-detail-sdtm-autocomplete-design.md` is implemented:
   - "Unset → highest id" → Task 2 (`resolvedVersionId` useMemo + Task 2 test "uses the highest id when sdtmig is absent").
   - "Valid → use it" → Task 2 test "uses the configured versionId when it is valid".
   - "Stale + name-match → self-heal" → Task 2 test "self-heals when versionId is stale but versionName matches" + "preserves language + tags in the recovery mutation body".
   - "Stale + name-missing → fall back" → Task 2 test "falls back to highest id when both versionId and versionName are stale".
   - "Language from project config" → Task 2 tests "returns project.language when set" and "falls back to 'en' when project.language is null".
   - "Domain annotation Autocomplete + description auto-fill" → Task 4 tests + implementation.
   - "@-mention detection + insertion" → Task 5 tests + implementation.
2. **Placeholders** — none. Every step shows the exact code and command.
3. **Type consistency** — `useSdtmContext` returns `SdtmContext` (defined once in Task 2 and referenced in Task 3). `DomainAnnotationDialog` props `sdtmDomains` / `sdtmLanguage` are defined in Task 4 and threaded in Task 3. `AnnotationDialog` prop `sdtmDomains` is defined in Task 5 and threaded in Task 3. `crf-variable-${id}` test id is defined in Task 5 implementation and asserted in Task 5 tests. All names match.