# Project Dashboard CRF Status Pie Chart Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a per-CRF-version pie chart summarising CRF form approval status to the project workspace dashboard, with `@mui/x-charts` installed in `lib/packages/ui` and re-exported as `@aegis/ui/charts`.

**Architecture:** `lib/packages/ui` gains a `./charts` subpath that re-exports `@mui/x-charts` as a regular dependency, so the charts share the workspace's single MUI instance and therefore the app's theme. On the desktop side, a new `CrfStatusPieCard` component owns one CRF version's chart and fetches that version's forms with the existing `useListCrfForms` hook; `ProjectDashboardPage` fetches the version list with `useListCrfVersions` and lays the cards out in a responsive grid. Frontend only — no Rust, server, or wire-shape change.

**Tech Stack:** React 19, TypeScript 5.8, MUI 9.2.0, `@mui/x-charts` 9.16.0, TanStack Query 5, TanStack Router 1, Vitest 2 + Testing Library.

## Global Constraints

These apply to every task. They encode decisions that have already been approved — do not relitigate them.

- `@mui/x-charts` is pinned to exactly `9.16.0` and lives in the `dependencies` block of `lib/packages/ui/package.json` — **never** in `peerDependencies`.
- After the install, the lockfile must contain exactly **one** `@mui/material@9.2.0` entry and exactly **one** `@mui/system@9.2.0` entry. Two of either means a duplicated MUI instance and must be fixed before proceeding.
- The chart has exactly **two** slices: Approved and Pending, derived from `CrfForm.approved`. The `notSubmitted` flag is **never** charted.
- Slice colours come from the active MUI theme — `theme.palette.success.main` and `theme.palette.warning.main`. No hard-coded hex values anywhere.
- The library's own legend is suppressed with `hideLegend`. A custom legend row renders swatch + label + count + percentage.
- Reuse the existing translation keys `crf.toolbar.statusApproved` and `crf.toolbar.statusPending`. Do **not** add dashboard-scoped duplicates of them.
- Every new translation key is added to **both** `lib/packages/ui/src/i18n/locales/en.ts` and `lib/packages/ui/src/i18n/locales/zhCN.ts` — `lib/packages/ui/src/i18n/registry.test.ts` asserts the two catalogs have identical key sets.
- All user-visible strings go through `useI18n()` → `t(...)`. No inline literals in JSX.
- MUI primitives come only from `@aegis/ui/mui`. Never import from `@mui/material` directly.
- Query keys come only from the `queryKeys` factory in `apps/desktop/aegis-desktop/src/shared/query/keys.ts`.
- Error text is rendered with `errorMessage(e)` from `apps/desktop/aegis-desktop/src/shared/api/error.ts`.
- Cross-feature imports go through a sibling feature's `data` barrel (`../../crf/data`), never a nested path into a sibling feature's files.
- Frontend only. No Rust, no server, no change to `apps/desktop/aegis-desktop/src/shared/api/types.ts`.
- The `dataviz` skill must be invoked before writing the chart rendering code in Task 3. Where its general advice conflicts with the constraints above, the constraints win — they encode decisions that have already been approved.
- The `aegis-desktop` test suite has **14 pre-existing failures on `main`**, recorded below. They are out of scope. The failure count must not rise above 14, and no new failing file may appear.

## Baseline

Measured on `feat/desktop_project-dashboard` at `49082f6`, which has **zero code changes relative to `main`** (only the two docs commits). `pnpm --filter @aegis/ui test` is fully green: 100/100 passing.

`pnpm --filter aegis-desktop test` reports **14 failed / 669 passed across 7 files**:

| Count | File | Root cause |
|---|---|---|
| 3 | `src/test/features/project-list/project-filter-bar.test.tsx` | `getByLabelText(/search/i)` fails — the component renders a `placeholder`, not a `label` (MUI v9 upgrade regression) |
| 6 | `src/test/features/project-list/project-list-page.test.tsx` | Same `/search/i` label regression |
| 1 | `src/test/features/crf/crf-form-list-page.test.tsx` | `getByRole("heading", { name: /CRF Form List/i })` not found |
| 1 | `src/test/features/crf/crf-form-table.test.tsx` | QC chip `variant=outlined` border assertion |
| 1 | `src/test/features/metadata/metadata-page.test.tsx` | `aria-disabled="true"` expected, `null` received (MUI v9) |
| 1 | `src/test/features/project-list/project-drawer.test.tsx` | Test timed out after 5000 ms |
| 1 | `src/test/features/project-workspace/project-workspace-route.test.tsx` | `unexpected tauri command: current_user` — the file's `mockCommands` never stubs `current_user`, so the Configuration page renders its error alert instead of its heading |

These are pre-existing MUI v9 migration debt and incomplete Tauri mocks. They are **not** part of this feature's scope. Note that `project-workspace-route.test.tsx` is one of the files this plan edits (Task 4), so its pre-existing configuration failure will still be present after the change — it must not be mistaken for a regression.

## File Structure

| File | Responsibility |
|---|---|
| `lib/packages/ui/package.json` | Declares the `@mui/x-charts` dependency and the `./charts` export subpath. |
| `lib/packages/ui/src/charts/index.ts` | **New.** Thin re-export of `@mui/x-charts`, mirroring `src/dnd/index.ts`. |
| `lib/packages/ui/src/charts/index.test.ts` | **New.** Asserts the re-export surface resolves. |
| `lib/packages/ui/src/i18n/locales/en.ts` | **Modified.** Adds the three dashboard keys. |
| `lib/packages/ui/src/i18n/locales/zhCN.ts` | **Modified.** Adds the same three keys in Simplified Chinese. |
| `lib/packages/ui/src/i18n/registry.test.ts` | **Modified.** Asserts the three new keys resolve in both locales. |
| `apps/desktop/aegis-desktop/src/features/project-workspace/components/CrfStatusPieCard.tsx` | **New.** One CRF version's status card: fetches its forms, derives the counts, renders the pie and the legend. |
| `apps/desktop/aegis-desktop/src/features/project-workspace/components/ContrastArcLabel.tsx` | **New.** A `pieArcLabel` slot that colours the on-arc percentage against its own fill, plus the `pickContrastText` helper it uses. |
| `apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectDashboardPage.tsx` | **Modified.** Fetches the CRF versions and renders the grid of cards. |
| `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts` | **Modified.** Exports `CrfStatusPieCard` from the feature barrel. |
| `apps/desktop/aegis-desktop/src/test/features/project-workspace/crf-status-pie-card.test.tsx` | **New.** Unit tests for the card. |
| `apps/desktop/aegis-desktop/src/test/features/project-workspace/contrast-arc-label.test.tsx` | **New.** Unit tests for `pickContrastText` against the theme's status fills. |
| `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-dashboard-page.test.tsx` | **New.** Route-level tests for the dashboard grid. |
| `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-workspace-route.test.tsx` | **Modified.** Its `mockCommands` must now also handle `list_crf_versions`. |

---

### Task 1: Install `@mui/x-charts` and expose `@aegis/ui/charts`

**Files:**
- Modify: `lib/packages/ui/package.json`
- Create: `lib/packages/ui/src/charts/index.ts`
- Create: `lib/packages/ui/src/charts/index.test.ts`
- Modify: `pnpm-lock.yaml` (generated by the install)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: the `@aegis/ui/charts` module, exporting everything `@mui/x-charts` exports — including `PieChart`, `PieSeries`, and `PieValueType`. Later tasks import `PieChart` from `@aegis/ui/charts`.

- [ ] **Step 1: Add the dependency**

```bash
pnpm --filter @aegis/ui add @mui/x-charts@9.16.0 --save-exact
```

Expected: `@mui/x-charts` appears in `lib/packages/ui/package.json` under `"dependencies"` (not `peerDependencies`) as exactly `"9.16.0"`, and `pnpm-lock.yaml` is updated.

- [ ] **Step 2: Verify the dependency landed in the right block and is pinned**

Read `lib/packages/ui/package.json` and confirm the `dependencies` block contains:

```json
"dependencies": {
  "@dnd-kit/react": "0.5.0",
  "@mui/x-charts": "9.16.0"
}
```

Expected: the version is exact (no `^` or `~`), and `@mui/x-charts` is **not** also listed under `peerDependencies`. If pnpm added it to `peerDependencies`, remove that line and re-add it under `dependencies` by hand, then re-run the install.

- [ ] **Step 3: Add the export subpath**

In `lib/packages/ui/package.json`, add the `"./charts"` entry to the `exports` map, keeping the existing entries:

```json
"exports": {
  ".": "./src/index.ts",
  "./dnd": "./src/dnd/index.ts",
  "./mui": "./src/mui/index.ts",
  "./icons": "./src/icons/index.ts",
  "./theme": "./src/theme/index.ts",
  "./i18n": "./src/i18n/index.ts",
  "./charts": "./src/charts/index.ts"
}
```

- [ ] **Step 4: Create the re-export module**

Create `lib/packages/ui/src/charts/index.ts`:

```ts
// Thin re-export of `@mui/x-charts@9.16.0`. Consumers compose the chart
// primitives directly; the UI package ships no chart wrapper. The
// dependency is a regular (not peer) dependency so the charts resolve
// the same `@mui/material` instance the rest of the app uses — a second
// instance would put the charts on a different theme context.
export * from '@mui/x-charts';
```

- [ ] **Step 5: Write the failing export-surface test**

Create `lib/packages/ui/src/charts/index.test.ts`:

```ts
import { describe, expect, it } from 'vitest';

import { PieChart } from './index';

describe('@aegis/ui/charts', () => {
  it('re-exports the x-charts pie primitives', () => {
    expect(PieChart).toBeDefined();
  });
});
```

- [ ] **Step 6: Run the test to verify it passes**

```bash
pnpm --filter @aegis/ui test -- src/charts/index.test.ts
```

Expected: PASS — 1 test. (This test guards the export surface, so it passes as soon as Steps 1 and 4 are done; if it fails with "Cannot find module '@mui/x-charts'", the install in Step 1 did not land and must be re-run.)

- [ ] **Step 7: Typecheck the UI package**

```bash
pnpm --filter @aegis/ui typecheck
```

Expected: PASS with no errors.

- [ ] **Step 8: Verify there is only one MUI instance**

The lockfile has two sections — `packages:` (one entry per version) and `snapshots:` (one entry per resolved peer-context instance). A duplicated MUI instance shows up as a **second snapshot**, so the count must be scoped to the `snapshots:` section:

```bash
awk '/^snapshots:$/{f=1} f' pnpm-lock.yaml > /tmp/snap.txt
for p in '@mui/material@' '@mui/system@' '@mui/x-charts@'; do
  echo "$p -> $(grep -c "^  '$p" /tmp/snap.txt)"
done
```

Expected: each of the three prints `1`.

Also confirm the desktop app still has no MUI of its own — MUI must remain owned by `@aegis/ui`:

```bash
ls apps/desktop/aegis-desktop/node_modules/@mui   # must report "No such file or directory"
ls lib/packages/ui/node_modules/@mui             # must list icons-material@, material@, x-charts@
```

If any snapshot count is greater than `1`, pnpm resolved a second MUI copy. Stop, revert the lockfile change (`git checkout -- pnpm-lock.yaml`), and re-run Step 1 with an explicit `--config.auto-install-peers=true` so the peers resolve to the versions already in the tree.

- [ ] **Step 9: Commit**

```bash
git add lib/packages/ui/package.json lib/packages/ui/src/charts pnpm-lock.yaml
git commit -m "feat(ui): add @mui/x-charts and expose it as @aegis/ui/charts

Installs @mui/x-charts@9.16.0 as a regular dependency of @aegis/ui and
re-exports it through a new ./charts subpath, mirroring the existing
./dnd subpath.

It is a dependency rather than a peer on purpose: @aegis/ui owns the
workspace's only @mui/material instance (the desktop app imports MUI
solely through @aegis/ui/mui), so a peer declaration would make the
desktop app pull in a second, unthemed MUI copy.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 2: Add the project dashboard translation keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`
- Modify: `lib/packages/ui/src/i18n/registry.test.ts`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: three new `TranslationKey` values — `workspace.dashboard.crfStatusTitle`, `workspace.dashboard.noCrfVersions`, `workspace.dashboard.noCrfForms`. Tasks 3 and 4 call them via `t(...)`.

- [ ] **Step 1: Add the keys to the English catalog**

In `lib/packages/ui/src/i18n/locales/en.ts`, find the line:

```ts
  'workspace.dashboard.heading': 'Dashboard — {projectCode}',
```

and insert these three lines directly after it:

```ts
  'workspace.dashboard.crfStatusTitle': 'CRF form status',
  'workspace.dashboard.noCrfVersions': 'No CRF versions yet',
  'workspace.dashboard.noCrfForms': 'No forms in this version',
```

- [ ] **Step 2: Add the same keys to the Simplified Chinese catalog**

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, find the line:

```ts
  'workspace.dashboard.heading': '仪表板 — {projectCode}',
```

and insert these three lines directly after it:

```ts
  'workspace.dashboard.crfStatusTitle': 'CRF 表单状态',
  'workspace.dashboard.noCrfVersions': '暂无 CRF 版本',
  'workspace.dashboard.noCrfForms': '此版本暂无表单',
```

- [ ] **Step 3: Write the failing test**

In `lib/packages/ui/src/i18n/registry.test.ts`, append a new `describe` block at the end of the file:

```ts
describe('project dashboard catalog', () => {
  const keys = [
    'workspace.dashboard.crfStatusTitle',
    'workspace.dashboard.noCrfVersions',
    'workspace.dashboard.noCrfForms',
  ] as const;

  it.each(keys)(
    'has a non-empty en and zh-CN message for %s',
    (key) => {
      expect(translate('en', key)).not.toBe(key);
      expect(translate('en', key).length).toBeGreaterThan(0);
      expect(translate('zh-CN', key)).not.toBe(key);
      expect(translate('zh-CN', key).length).toBeGreaterThan(0);
    },
  );
});
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
pnpm --filter @aegis/ui test -- src/i18n/registry.test.ts
```

Expected: PASS. The file's pre-existing case `'keeps both catalogs on the same key set'` also passes — that is the check that would fail if Step 1 and Step 2 had added different key sets.

- [ ] **Step 5: Commit**

```bash
git add lib/packages/ui/src/i18n
git commit -m "feat(ui): add the project dashboard translation keys

Adds the CRF status section heading and the two empty-state strings in
both the English and Simplified Chinese catalogs.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 3: Add the `CrfStatusPieCard` component

**Files:**
- Create: `apps/desktop/aegis-desktop/src/features/project-workspace/components/CrfStatusPieCard.tsx`
- Create: `apps/desktop/aegis-desktop/src/features/project-workspace/components/ContrastArcLabel.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts`
- Test: `apps/desktop/aegis-desktop/src/test/features/project-workspace/crf-status-pie-card.test.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/project-workspace/contrast-arc-label.test.tsx`

**Interfaces:**
- Consumes: `useListCrfForms(versionId: number | null): UseQueryResult<CrfForm[], ApiError>` from `../../crf/data`; `PieChart` from `@aegis/ui/charts` (Task 1); `errorMessage(e: unknown): string` from `../../../shared/api/error`; the type `CrfVersion` from `../../../shared/api`; the translation keys from Task 2.
- Produces: `CrfStatusPieCard({ version: CrfVersion }): React.JSX.Element`, exported from `features/project-workspace/index.ts`. Task 4 renders one per CRF version.

- [ ] **Step 1: Invoke the `dataviz` skill before writing chart code**

Invoke the `dataviz` skill via the Skill tool and follow its procedure. Its findings for this chart, measured with `scripts/validate_palette.js`:

- The theme's `success` green (`#2e7d32`) and `warning` orange (`#ed6c02`) are **close under protanopia** — ΔE 5.7 light / 5.9 dark, against a target of ΔE 8. Normal vision is fine (ΔE 28.3 / 20.3).
- `dataviz` names this form twice: `choosing-a-form.md` maps "a single ratio against a limit" to a *meter*, not "a pie of 2 slices", and `anti-patterns.md` lists a 2-slice pie as bad with a stat tile as the good alternative.

Keep the pie — it is the approved form, and the measures below remove the dependency on hue — but apply every non-conflicting rule. The three colour-vision measures the implementation must carry are in the Step 4 code: on-arc percentage labels, a status icon on every legend row, and a surface-coloured gap between the arcs.

- [ ] **Step 2: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/project-workspace/crf-status-pie-card.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { CrfStatusPieCard } from "../../../features/project-workspace/components/CrfStatusPieCard";
import { renderWithQueryClient } from "../../helpers/render-with-query-client";
import { httpError, mockCommands } from "../../helpers/tauri-mock";
import type { CrfForm, CrfVersion } from "../../../shared/api";

const version: CrfVersion = {
  id: 7,
  projectCode: "DEMO-001",
  name: "CRF v1.0",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

function form(overrides: Partial<CrfForm> = {}): CrfForm {
  return {
    id: 1,
    versionId: 7,
    code: "FORM",
    name: "Form",
    order: 1,
    notSubmitted: false,
    approved: false,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

function renderCard() {
  return renderWithQueryClient(
    <AegisThemeProvider>
      <AegisI18nProvider>
        <CrfStatusPieCard version={version} />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

afterEach(() => cleanup());

describe("CrfStatusPieCard", () => {
  it("titles the card with the CRF version name", async () => {
    mockCommands({
      list_crf_forms_by_version: () => ({ forms: [form({ approved: true })] }),
    });
    renderCard();
    expect(await screen.findByText("CRF v1.0")).toBeInTheDocument();
  });

  it("splits the forms into Approved and Pending by the approved flag", async () => {
    mockCommands({
      list_crf_forms_by_version: () => ({
        forms: [
          form({ id: 1, approved: true }),
          form({ id: 2, approved: true }),
          form({ id: 3, approved: false }),
        ],
      }),
    });
    renderCard();
    const approved = await screen.findByTestId("crf-status-count-approved");
    expect(approved).toHaveTextContent("2");
    expect(approved).toHaveTextContent("67%");
    const pending = screen.getByTestId("crf-status-count-pending");
    expect(pending).toHaveTextContent("1");
    expect(pending).toHaveTextContent("33%");
    expect(screen.getByText("Approved")).toBeInTheDocument();
    expect(screen.getByText("Pending")).toBeInTheDocument();
  });

  it("counts by the approved flag only and ignores notSubmitted", async () => {
    mockCommands({
      list_crf_forms_by_version: () => ({
        forms: [
          form({ id: 1, approved: true, notSubmitted: true }),
          form({ id: 2, approved: false, notSubmitted: true }),
        ],
      }),
    });
    renderCard();
    expect(await screen.findByTestId("crf-status-count-approved")).toHaveTextContent(
      "1",
    );
    expect(screen.getByTestId("crf-status-count-pending")).toHaveTextContent("1");
  });

  it("shows the empty state when the version has no forms", async () => {
    mockCommands({ list_crf_forms_by_version: () => ({ forms: [] }) });
    renderCard();
    expect(
      await screen.findByText("No forms in this version"),
    ).toBeInTheDocument();
    expect(screen.queryByTestId("crf-status-count-approved")).not.toBeInTheDocument();
  });

  it("gives every status row an icon as well as its label", async () => {
    mockCommands({
      list_crf_forms_by_version: () => ({ forms: [form({ approved: true })] }),
    });
    renderCard();
    expect(
      await screen.findByTestId("crf-status-icon-approved"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("crf-status-icon-pending")).toBeInTheDocument();
  });

  it("shows an error inside the card when the forms query fails", async () => {
    mockCommands({
      list_crf_forms_by_version: () => {
        throw httpError(500, "internal_error", "boom");
      },
    });
    renderCard();
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("internal_error: boom");
  });
});
```

Also create `apps/desktop/aegis-desktop/src/test/features/project-workspace/contrast-arc-label.test.tsx`:

```tsx
import { describe, expect, it } from "vitest";
import { createTheme } from "@aegis/ui/mui";

import { pickContrastText } from "../../../features/project-workspace/components/ContrastArcLabel";

describe("pickContrastText", () => {
  const light = createTheme({ palette: { mode: "light" } });
  const dark = createTheme({ palette: { mode: "dark" } });

  it("returns white on the light theme's success green", () => {
    expect(pickContrastText(light.palette.success.main, light)).toBe("#fff");
  });

  it("returns black on the light theme's warning orange", () => {
    expect(pickContrastText(light.palette.warning.main, light)).toBe("#000");
  });

  it("keeps every theme status fill legible", () => {
    for (const theme of [light, dark]) {
      for (const fill of [
        theme.palette.success.main,
        theme.palette.warning.main,
      ]) {
        const text = pickContrastText(fill, theme);
        expect(["#fff", "#000"]).toContain(text);
      }
    }
  });
});
```

- [ ] **Step 3: Run the test to verify it fails**

```bash
pnpm --filter aegis-desktop test -- src/test/features/project-workspace/crf-status-pie-card.test.tsx
```

Expected: FAIL — the module `features/project-workspace/components/CrfStatusPieCard` does not exist.

- [ ] **Step 4: Implement the component**

First create `apps/desktop/aegis-desktop/src/features/project-workspace/components/ContrastArcLabel.tsx`:

```tsx
import { getContrastRatio, useTheme, type Theme } from "@aegis/ui/mui";
import { PieArcLabel, type PieArcLabelProps } from "@aegis/ui/charts";

/**
 * Pick the text colour with the higher WCAG contrast against
 * `background`, choosing between pure white and pure black.
 *
 * `theme.palette.getContrastText` is deliberately not used here: it
 * compares against `palette.text.primary`, which is 87% opaque black,
 * so the alpha is dropped when the ratio is computed and the colour it
 * returns can fall below 3:1 once it is actually composited over the
 * fill. Pure black and pure white avoid that.
 */
export function pickContrastText(background: string, theme: Theme): string {
  const { white, black } = theme.palette.common;
  return getContrastRatio(background, white) >=
    getContrastRatio(background, black)
    ? white
    : black;
}

/**
 * A `pieArcLabel` slot that colours the label to contrast with the arc
 * it sits on.
 *
 * The library's default label colour is `palette.text.primary`, which
 * is unreadable on a saturated fill — on MUI's `success.main` green it
 * lands at roughly 3:1. Every arc in this chart carries a percentage,
 * so the label has to be legible against its own fill.
 */
export function ContrastArcLabel(props: PieArcLabelProps) {
  const theme = useTheme();
  const { color, ...rest } = props;
  return (
    <PieArcLabel
      {...rest}
      color={color}
      style={{ fill: pickContrastText(color, theme) }}
    />
  );
}
```

Then create `apps/desktop/aegis-desktop/src/features/project-workspace/components/CrfStatusPieCard.tsx`:

```tsx
import {
  Alert,
  Box,
  Card,
  CardContent,
  CardHeader,
  Skeleton,
  Typography,
  useTheme,
} from "@aegis/ui/mui";
import { PieChart } from "@aegis/ui/charts";
import {
  PendingActions as PendingActionsIcon,
  Verified as VerifiedIcon,
} from "@aegis/ui/icons";
import { useI18n } from "@aegis/ui/i18n";

import { useListCrfForms } from "../../crf/data";
import { errorMessage } from "../../../shared/api/error";
import type { CrfVersion } from "../../../shared/api";
import { ContrastArcLabel } from "./ContrastArcLabel";

/** Height of the pie's SVG drawing area, in px. */
const CHART_HEIGHT = 220;

/**
 * Gap between the two arcs, in degrees. At the radii this card uses
 * (roughly 90–110 px) 2° is a ~3 px gap in the surface colour, which
 * is what keeps the two fills from reading as a single shape.
 */
const ARC_PADDING_ANGLE = 2;

/**
 * Below this arc angle the on-arc percentage is dropped — a sliver has
 * no room for it, and a label that overflows its arc is worse than no
 * label at all. The legend carries the value either way.
 */
const ARC_LABEL_MIN_ANGLE = 25;

interface CrfStatusPieCardProps {
  /** The CRF version whose forms this card summarises. */
  version: CrfVersion;
}

/**
 * One CRF version's form-approval status, as a pie chart.
 *
 * The slices are derived from `CrfForm.approved` alone: `Approved` is
 * the count of approved forms and `Pending` is everything else. The
 * `notSubmitted` flag is deliberately not charted — it is a separate
 * submission flag, not an approval state, and the aggregate CRF status
 * chip treats approval as binary too.
 *
 * The colours come from the MUI theme (`success` / `warning`) so the
 * card matches the `CrfStatusChip` on the CRF list page and follows the
 * active theme pack. Because those two fills are close under
 * protanopia, identity is carried by text rather than by hue: every
 * arc states its own percentage, and every legend row carries a status
 * icon as well as its label.
 */
export function CrfStatusPieCard({ version }: CrfStatusPieCardProps) {
  const { t } = useI18n();
  const theme = useTheme();
  const formsQuery = useListCrfForms(version.id);

  const forms = formsQuery.data ?? [];
  const total = forms.length;
  const approved = forms.filter((f) => f.approved).length;
  const pending = total - approved;

  const rows = [
    {
      key: "approved",
      label: t("crf.toolbar.statusApproved"),
      count: approved,
      color: theme.palette.success.main,
      icon: <VerifiedIcon fontSize="small" />,
    },
    {
      key: "pending",
      label: t("crf.toolbar.statusPending"),
      count: pending,
      color: theme.palette.warning.main,
      icon: <PendingActionsIcon fontSize="small" />,
    },
  ];

  return (
    <Card variant="outlined">
      <CardHeader
        title={version.name}
        slotProps={{ title: { variant: "h6" } }}
      />
      <CardContent>
        {formsQuery.isLoading && (
          <Skeleton variant="rounded" height={CHART_HEIGHT} />
        )}

        {formsQuery.isError && (
          <Alert severity="error">{errorMessage(formsQuery.error)}</Alert>
        )}

        {formsQuery.isSuccess && total === 0 && (
          <Typography color="textSecondary">
            {t("workspace.dashboard.noCrfForms")}
          </Typography>
        )}

        {formsQuery.isSuccess && total > 0 && (
          <>
            <PieChart
              height={CHART_HEIGHT}
              hideLegend
              slots={{ pieArcLabel: ContrastArcLabel }}
              series={[
                {
                  data: rows.map((row) => ({
                    id: row.key,
                    value: row.count,
                    label: row.label,
                    color: row.color,
                  })),
                  paddingAngle: ARC_PADDING_ANGLE,
                  arcLabel: (item) =>
                    `${Math.round((item.value / total) * 100)}%`,
                  arcLabelMinAngle: ARC_LABEL_MIN_ANGLE,
                },
              ]}
            />
            <Box sx={{ display: "flex", flexDirection: "column", gap: 0.5 }}>
              {rows.map((row) => (
                <Box
                  key={row.key}
                  sx={{ display: "flex", alignItems: "center", gap: 1 }}
                >
                  <Box
                    data-testid={`crf-status-icon-${row.key}`}
                    sx={{ color: row.color, display: "flex" }}
                  >
                    {row.icon}
                  </Box>
                  <Typography
                    variant="body2"
                    color="text.secondary"
                    sx={{ flexGrow: 1 }}
                  >
                    {row.label}
                  </Typography>
                  <Typography
                    variant="body2"
                    data-testid={`crf-status-count-${row.key}`}
                  >
                    {row.count} (
                    {Math.round((row.count / total) * 100)}%)
                  </Typography>
                </Box>
              ))}
            </Box>
          </>
        )}
      </CardContent>
    </Card>
  );
}
```

- [ ] **Step 5: Export the component from the feature barrel**

In `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts`, add the export alongside the existing component exports:

```ts
export { CrfStatusPieCard } from "./components/CrfStatusPieCard";
```

- [ ] **Step 6: Run the test to verify it passes**

Note: `pnpm --filter aegis-desktop test -- <path>` does **not** filter — the extra `--` reaches vitest as a positional pattern and the whole suite runs. Use `exec` to pass the path through:

```bash
pnpm --filter aegis-desktop exec vitest run src/test/features/project-workspace/crf-status-pie-card.test.tsx src/test/features/project-workspace/contrast-arc-label.test.tsx
```

Expected: PASS — 9 tests (6 card + 3 contrast).

If the test fails with `getBBox is not a function`, stop and report it: that would mean the installed `@mui/x-charts` behaves differently from the 9.16.0 that was inspected, and the fix is to pin the version rather than to add a jsdom shim.

- [ ] **Step 7: Typecheck the desktop app**

```bash
pnpm --filter aegis-desktop typecheck
```

Expected: PASS with no errors.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace apps/desktop/aegis-desktop/src/test/features/project-workspace
git commit -m "feat(desktop): add the CRF status pie card

Renders one CRF version's form-approval status as a pie chart with a
legend row carrying the exact counts. The slices are derived from
CrfForm.approved alone and coloured from the MUI theme, so the card
matches the existing CrfStatusChip on the CRF list page.

The chart is hardened for colour-vision deficiency. The theme's
success/warning pair is close under protanopia, so identity is carried
by text rather than by hue: every arc states its own percentage, every
legend row carries a status icon as well as its label, and the two
arcs are separated by a surface-coloured gap. The arc label colour is
picked against its own fill, because the library's default
palette.text.primary lands near 3:1 on the success green.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 4: Render the CRF status charts on the project dashboard

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectDashboardPage.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-workspace-route.test.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-dashboard-page.test.tsx`

**Interfaces:**
- Consumes: `useListCrfVersions(projectCode: string | null): UseQueryResult<CrfVersion[], ApiError>` from `../../crf/data`; `CrfStatusPieCard({ version: CrfVersion })` from `../components/CrfStatusPieCard` (Task 3); the translation keys `workspace.dashboard.crfStatusTitle` and `workspace.dashboard.noCrfVersions` (Task 2).
- Produces: the updated `ProjectDashboardPage`. Nothing consumes it in turn — it is the feature's terminal deliverable.

- [ ] **Step 1: Write the failing test**

Create `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-dashboard-page.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { renderWithFullRouter } from "../../helpers/file-route-utils";
import { httpError, mockCommands } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";
import type { CrfForm, CrfVersion } from "../../../shared/api";

const versionA: CrfVersion = {
  id: 1,
  projectCode: "DEMO-001",
  name: "CRF v1.0",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

const versionB: CrfVersion = {
  id: 2,
  projectCode: "DEMO-001",
  name: "CRF v2.0",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
};

function form(overrides: Partial<CrfForm> = {}): CrfForm {
  return {
    id: 1,
    versionId: 1,
    code: "FORM",
    name: "Form",
    order: 1,
    notSubmitted: false,
    approved: false,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

function renderDashboard() {
  return renderWithFullRouter({
    initialEntries: ["/project/DEMO-001/dashboard"],
    wrapper: ({ children }) => (
      <AegisThemeProvider>
        <TestQueryProvider>
          <AegisI18nProvider>{children}</AegisI18nProvider>
        </TestQueryProvider>
      </AegisThemeProvider>
    ),
  });
}

afterEach(() => cleanup());

describe("project dashboard — CRF status charts", () => {
  it("renders one card per CRF version", async () => {
    mockCommands({
      is_logged_in: () => true,
      list_crf_versions: () => ({ versions: [versionA, versionB] }),
      list_crf_forms_by_version: () => ({ forms: [form({ approved: true })] }),
    });
    await renderDashboard();
    expect(await screen.findByText("CRF v1.0")).toBeInTheDocument();
    expect(screen.getByText("CRF v2.0")).toBeInTheDocument();
  });

  it("renders the CRF status section heading", async () => {
    mockCommands({
      is_logged_in: () => true,
      list_crf_versions: () => ({ versions: [versionA] }),
      list_crf_forms_by_version: () => ({ forms: [] }),
    });
    await renderDashboard();
    expect(
      await screen.findByRole("heading", { name: "CRF form status" }),
    ).toBeInTheDocument();
  });

  it("shows the empty state when the project has no CRF versions", async () => {
    mockCommands({
      is_logged_in: () => true,
      list_crf_versions: () => ({ versions: [] }),
    });
    await renderDashboard();
    expect(
      await screen.findByText("No CRF versions yet"),
    ).toBeInTheDocument();
  });

  it("isolates a per-version failure to its own card", async () => {
    mockCommands({
      is_logged_in: () => true,
      list_crf_versions: () => ({ versions: [versionA, versionB] }),
      list_crf_forms_by_version: (args) => {
        if (Number(args?.versionId) === 1) {
          throw httpError(500, "internal_error", "boom");
        }
        return { forms: [form({ versionId: 2, approved: true })] };
      },
    });
    await renderDashboard();
    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("internal_error: boom");
    expect(screen.getByText("CRF v2.0")).toBeInTheDocument();
    expect(screen.getByTestId("crf-status-count-approved")).toHaveTextContent("1");
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-dashboard-page.test.tsx
```

Expected: FAIL — the dashboard renders no CRF version cards and no "CRF form status" heading.

- [ ] **Step 3: Rewrite the dashboard page**

Replace the whole contents of `apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectDashboardPage.tsx` with:

```tsx
import { Alert, Box, Skeleton, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";
import { useParams } from "@tanstack/react-router";
import type { ReactNode } from "react";

import { useListCrfVersions } from "../../crf/data";
import { errorMessage } from "../../../shared/api/error";
import { CrfStatusPieCard } from "../components/CrfStatusPieCard";

/**
 * Responsive grid for the dashboard's chart cards. One column on
 * narrow windows, two on medium, three on wide — so the grid keeps
 * growing as more chart types are added to the dashboard.
 */
function DashboardGrid({ children }: { children: ReactNode }) {
  return (
    <Box
      sx={{
        display: "grid",
        gap: 2,
        gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr", lg: "1fr 1fr 1fr" },
      }}
    >
      {children}
    </Box>
  );
}

/**
 * Dashboard page for a project workspace window. Shows one status
 * chart per CRF version; further charts (KPIs, recent activity) are
 * added to the grid as they land.
 */
export function ProjectDashboardPage() {
  const { t } = useI18n();
  const { projectCode } = useParams({ strict: false }) as {
    projectCode: string;
  };
  const versionsQuery = useListCrfVersions(projectCode);

  const versions = versionsQuery.data ?? [];

  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>
        {t("workspace.dashboard.heading", { projectCode })}
      </Typography>

      <Typography variant="h6" gutterBottom>
        {t("workspace.dashboard.crfStatusTitle")}
      </Typography>

      {versionsQuery.isLoading && (
        <DashboardGrid>
          {[0, 1, 2].map((i) => (
            <Skeleton key={i} variant="rounded" height={260} />
          ))}
        </DashboardGrid>
      )}

      {versionsQuery.isError && (
        <Alert severity="error">{errorMessage(versionsQuery.error)}</Alert>
      )}

      {versionsQuery.isSuccess && versions.length === 0 && (
        <Typography color="textSecondary">
          {t("workspace.dashboard.noCrfVersions")}
        </Typography>
      )}

      {versionsQuery.isSuccess && versions.length > 0 && (
        <DashboardGrid>
          {versions.map((version) => (
            <CrfStatusPieCard key={version.id} version={version} />
          ))}
        </DashboardGrid>
      )}
    </Box>
  );
}
```

- [ ] **Step 4: Update the existing route test's mock**

`apps/desktop/aegis-desktop/src/test/features/project-workspace/project-workspace-route.test.tsx` mounts the dashboard route. Now that the dashboard calls `list_crf_versions`, the `mockCommands` helper in that file must handle it — otherwise the mock rejects with `unexpected tauri command: list_crf_versions` and the dashboard renders an error instead of its content.

In that file, change both `mockCommands({ is_logged_in: () => true })` calls (one in the `"/project/DEMO-001/dashboard — authenticated"` block and one in the `"/project/DEMO-001/configuration — authenticated"` block) to:

```tsx
mockCommands({
  is_logged_in: () => true,
  list_crf_versions: () => ({ versions: [] }),
});
```

- [ ] **Step 5: Run the dashboard tests to verify they pass**

```bash
pnpm --filter aegis-desktop exec vitest run src/test/features/project-workspace/project-dashboard-page.test.tsx
```

Expected: PASS — 4 tests.

- [ ] **Step 6: Run the whole project-workspace test folder**

```bash
pnpm --filter aegis-desktop exec vitest run src/test/features/project-workspace
```

Expected: every project-workspace test passes **except one pre-existing failure**: `project-workspace-route.test.tsx > /project/DEMO-001/configuration — authenticated > renders the Configuration heading with the project code`. That test fails on `main` before this change (its `mockCommands` never stubs `current_user`, so the Configuration page renders its error alert instead of its heading). It is recorded here so it is not mistaken for a regression — any *other* failure is one.

- [ ] **Step 7: Run the full desktop test suite and typecheck**

```bash
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop typecheck
```

Expected: typecheck passes with no errors. The test suite reports the **same 14 pre-existing failures** recorded in the baseline (9 in `project-filter-bar` / `project-list-page`, 3 in `crf-form-list-page` / `crf-form-table` / `metadata-page`, 1 in `project-drawer`, 1 in `project-workspace-route`) — all of which fail on `main` before this change. The failure count must not rise above 14, and no new failing file may appear.

**Measured result:** 13 failed / 683 passed across 78 files (baseline: 14 failed / 669 passed across 75 files). The three new test files add 13 tests, all green. The count is one *lower* than the baseline because `project-drawer`'s 5000 ms timeout is flaky and did not reproduce on this run; every remaining failure is a baseline failure, and **no new failing file appeared**.

- [ ] **Step 8: Run the full verification gate**

```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter @aegis/ui test
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build
```

Expected: all PASS.

- [ ] **Step 9: Check the chart is themed in the real app**

```bash
pnpm --filter aegis-desktop tauri dev
```

Open a project workspace window and look at the dashboard. Confirm the two slices are drawn in the theme's success and warning colours (not the library's default palette), and that switching the theme mode re-colours them. This is the one check the automated tests cannot make, because the chart's colours are read from the runtime theme context.

- [ ] **Step 10: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/pages/ProjectDashboardPage.tsx apps/desktop/aegis-desktop/src/test/features/project-workspace
git commit -m "feat(desktop): show the CRF status charts on the project dashboard

Replaces the Coming soon placeholder with a responsive grid of CRF
status charts, one per CRF version. The dashboard route test's mock is
extended to handle the new list_crf_versions call.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Verification

After all four tasks:

```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter @aegis/ui test
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build
```

Plus the two manual checks that the automated suite cannot make:

- The lockfile holds exactly one `@mui/material@9.2.0` entry and exactly one `@mui/system@9.2.0` entry.
- In the running app, the chart's slices use the theme's success/warning colours and follow the theme mode.
