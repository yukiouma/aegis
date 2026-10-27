# Project Dashboard — CRF Form Status Pie Chart — Design

**Date:** 2026-10-27
**Scope:** Add a per-CRF-version pie chart summarizing CRF form approval status to the project workspace dashboard. `@mui/x-charts` is installed inside `lib/packages/ui` and re-exported for `aegis-desktop` to consume. This is the first chart on the dashboard; further chart types are expected but out of scope here.

## Motivation

`ProjectDashboardPage` is currently a placeholder — a heading and a "Coming soon" line. The data it needs already exists and is already wired to the frontend: `listCrfVersions(projectCode)` and `listCrfFormsByVersion(versionId)` are exposed on the `api` object and backed by `useListCrfVersions` / `useListCrfForms` in `features/crf/data/list.ts`.

The dashboard's first useful tile is a per-version view of how far the CRF forms have progressed. The approval state already has a single source of truth — `CrfForm.approved` — and a single established visual language: `CrfStatusChip` renders "Approved" in MUI `success` and "Pending" in `warning`. A pie chart per CRF version makes that ratio legible at a glance and, because the counts are derived from the same field and coloured with the same palette, reads as part of the same system as the CRF list page.

## Decisions

Three choices were made explicitly during design and are recorded here so they are not re-litigated:

| Decision | Choice | Rationale |
|---|---|---|
| Chart slices | **Binary: Approved / Pending**, derived from `CrfForm.approved` only | `CrfForm` also carries a `notSubmitted` flag, but the spec for this feature designates `approved` as the status field, and `CrfStatusChip` already treats approval as binary. `notSubmitted` is **not** charted. |
| Layout | **Responsive grid of cards**, one card per CRF version | Scales to N versions and leaves room for the other chart types planned for the dashboard. |
| Data layer | **Component-level hooks** — each card calls `useListCrfForms(version.id)` itself | Least new surface; reuses existing CRF hooks unchanged. Matches the existing pattern where components call data hooks (`ConfigurationGeneralSection` → `useListSdtmVersions`, `ProjectDrawer` → `useListUsers`). |

Rejected alternatives: a dedicated `useDashboardCrfStatus` hook fanning out with `useQueries` (adds a layer for no present benefit — the natural upgrade if a project-wide aggregate KPI is added later), and a new server-side aggregate endpoint (out of proportion; would touch `lib/crates/crf`, the server, and the desktop transport for one dashboard tile).

## Dependency & export — `lib/packages/ui`

`@mui/x-charts@9.16.0` is added to `lib/packages/ui` and re-exported under a new `./charts` subpath, mirroring the existing `./dnd` subpath.

```jsonc
// lib/packages/ui/package.json
{
  "exports": {
    ".": "./src/index.ts",
    "./dnd": "./src/dnd/index.ts",
    "./mui": "./src/mui/index.ts",
    "./icons": "./src/icons/index.ts",
    "./theme": "./src/theme/index.ts",
    "./i18n": "./src/i18n/index.ts",
    "./charts": "./src/charts/index.ts"
  },
  "dependencies": {
    "@dnd-kit/react": "0.5.0",
    "@mui/x-charts": "9.16.0"
  }
}
```

```ts
// lib/packages/ui/src/charts/index.ts
// Thin re-export of `@mui/x-charts`. Consumers compose the chart
// primitives directly; the UI package ships no chart wrapper.
export * from '@mui/x-charts';
```

### Why `dependencies` and not `peerDependencies`

This is the one non-obvious packaging decision, so the reasoning is recorded.

`@mui/x-charts` peer-requires `@mui/material` (range `^7.3.0 || ^9.0.0`). The workspace's MUI topology is unusual: **`@aegis/ui` owns the only MUI instance in the repo.** `lib/packages/ui/node_modules/@mui/material@9.2.0` is materialised there by pnpm's `auto-install-peers` (it is declared as a *peer* of `@aegis/ui`), and `apps/desktop/aegis-desktop/node_modules/` contains **no** `@mui` directory at all — the desktop app reaches MUI exclusively through the `@aegis/ui/mui` re-export.

Declaring `@mui/x-charts` as a *peer* of `@aegis/ui` would force the desktop app to install it, and since the desktop app has no `@mui/material`, pnpm would auto-install a **second** MUI 9 into the desktop app's `node_modules`. The charts would then read a different theme context from the app's `ThemeProvider` and render unstyled. Declaring it as a regular **dependency** of `@aegis/ui` puts it in the same resolution scope as the existing `@mui/material@9.2.0`, so a single MUI instance — and a single theme — is shared.

`@mui/x-charts@9.16.0` is pinned exactly (matching how `@dnd-kit/react` is pinned) because it must resolve against MUI 9.2.0.

The root-barrel re-export (rather than `@mui/x-charts/PieChart`) gives the future chart types for free; Vite tree-shakes the unused ones from the production bundle.

**Post-install verification:** the lockfile must contain exactly one `@mui/material@9.2.0` snapshot and exactly one `@mui/system@9.2.0` snapshot. Two of either means a duplicated MUI instance and must be fixed before proceeding.

## Component layout — `aegis-desktop`

```
apps/desktop/aegis-desktop/src/features/project-workspace/
  components/
    CrfStatusPieCard.tsx     ← NEW
  pages/
    ProjectDashboardPage.tsx  ← MODIFIED
```

No new `data/` file. The dashboard reuses the CRF feature's existing hooks:

```ts
import { useListCrfForms, useListCrfVersions } from "../../crf/data";
```

The import targets the CRF feature's **data barrel** rather than the feature root barrel. `features/crf/index.ts` re-exports `./pages`, and pulling the CRF pages into the dashboard route's module graph serves no purpose; `ConfigurationGeneralSection` already reaches into a sibling feature's `data` path (`../../domain-model/data/list`), so this matches in-repo practice.

### `CrfStatusPieCard`

```tsx
interface CrfStatusPieCardProps {
  version: CrfVersion;
}
```

Responsibilities, in order:

1. **Fetch.** `useListCrfForms(version.id)` — the hook's `enabled` guard (`versionId > 0`) already covers the invalid-id case, and the query key `queryKeys.crf.formsByVersion(versionId)` means N cards produce N cache entries and zero redundant network calls.
2. **Derive counts.**
   ```ts
   const approved = forms.filter((f) => f.approved).length;
   const pending = forms.length - approved;
   ```
3. **Render.** MUI `Card` → `CardHeader` (the version's `name`) → `CardContent` holding the chart and the legend row.

### The chart

```tsx
const theme = useTheme();
const approvedLabel = t("crf.toolbar.statusApproved");
const pendingLabel = t("crf.toolbar.statusPending");

<PieChart
  height={220}
  series={[
    {
      data: [
        { id: 0, value: approved, label: approvedLabel, color: theme.palette.success.main },
        { id: 1, value: pending,  label: pendingLabel,  color: theme.palette.warning.main },
      ],
    },
  ]}
/>
```

- **Colours** are taken from the active MUI theme (`success.main` / `warning.main`), not hard-coded hex values, so the tile follows the app's theme mode and the theme pack. This is the same pairing `CrfStatusChip` uses, which keeps the dashboard tile and the CRF list page consistent.
- **Labels** reuse the existing `crf.toolbar.statusApproved` / `crf.toolbar.statusPending` translation keys rather than introducing dashboard-scoped duplicates that could drift.
- **The library's built-in legend is not used.** A custom legend row is rendered below the pie instead: a colour swatch, the label, the count, and the percentage (percentage = `Math.round((count / total) * 100)`; the two slices therefore sum to 100% except for rounding on a non-zero total). Exact counts carry more information than percentages alone on a status tile, the row remains readable without relying on colour, and it gives the tests a DOM target that does not depend on SVG arc geometry.

### `ProjectDashboardPage`

- Keeps the existing heading (`workspace.dashboard.heading`).
- Calls `useListCrfVersions(projectCode)`; `projectCode` comes from the route params as it does today.
- Renders a section heading (`workspace.dashboard.crfStatusTitle`) above a responsive grid of `CrfStatusPieCard`:
  ```tsx
  <Box sx={{ display: "grid", gap: 2, gridTemplateColumns: { xs: "1fr", sm: "1fr 1fr", lg: "1fr 1fr 1fr" } }}>
  ```
- The `"Coming soon"` placeholder line is **removed** — it contradicts the presence of real content — and the file's doc comment is updated to describe the dashboard's actual contents.

## States & error handling

| Condition | Rendering |
|---|---|
| Version list loading | Grid of card skeletons |
| Version list error | Page-level `<Alert severity="error">` using `errorMessage(e)` from `src/shared/api/error.ts` |
| No CRF versions | "No CRF versions yet" empty state |
| Forms loading (one card) | `<Skeleton>` inside that card |
| Forms error (one card) | `<Alert severity="error">` inside that card only — the other cards are unaffected |
| Version has zero forms | "No forms in this version" text in that card |

Error rendering follows the repo convention: `errorMessage(e)` from `src/shared/api/error.ts` is the only place that turns an `ApiError` into a string.

## i18n

`@aegis/ui` owns all translations, so the new keys are added to both `lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts`:

| Key | en | zh-CN |
|---|---|---|
| `workspace.dashboard.crfStatusTitle` | CRF form status | CRF 表单状态 |
| `workspace.dashboard.noCrfVersions` | No CRF versions yet | 暂无 CRF 版本 |
| `workspace.dashboard.noCrfForms` | No forms in this version | 此版本暂无表单 |

`TranslationKey` is derived from the `en` resource (`keyof typeof en`), so adding to `en.ts` extends the type automatically; `zhCN.ts` must be updated in the same commit to keep the two resources in step.

## Testing

New file: `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-dashboard-page.test.tsx`.

`invoke` is mocked per the repo convention (`vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))` in the test file — `vi.mock` is hoisted and cannot live in a helper module), dispatching `list_crf_versions` and `list_crf_forms_by_version` through `mockCommands`.

Cases covered:

1. One card per CRF version, titled with the version name.
2. The legend counts are derived from the `approved` flags — a version with 2 approved and 1 unapproved form reports 2 / 1.
3. A version with zero forms renders the "no forms" empty state rather than an empty pie.
4. A project with no CRF versions renders the "no versions" empty state.
5. A failing `list_crf_forms_by_version` surfaces an error in that card only; the sibling card still renders its chart.

Assertions target the custom legend DOM, not SVG paths.

### Known risk: `getBBox` in jsdom — investigated, no action needed

MUI X charts measure text with `SVGElement.getBBox`, which jsdom does not implement, and the chart is widely reported to throw `getBBox is not a function` during render.

**This was investigated against the installed `@mui/x-charts@9.16.0` and the risk does not materialise.** The library guards the call: `internals/domUtils.js` wraps `element.getBBox()` in a try/catch that falls back to `element.getBoundingClientRect()` (which jsdom implements, returning zeros), and `getStringSize` wraps the whole measurement in an outer try/catch that returns `{ width: 0, height: 0 }` on failure. The library is additionally test-aware (`process.env.NODE_ENV === 'test'` clears the measurement container after each measurement).

No `getBBox` shim is therefore added to `apps/desktop/aegis-desktop/src/test/helpers/Setup.ts` — the existing `ResizeObserver` / `IntersectionObserver` / `PointerEvent` shims are sufficient. The assertions are written against our own legend markup, so they hold regardless of whether the arcs render (in jsdom the chart's width resolves to 0, so no arcs are drawn).

## Out of scope

- Charting the `notSubmitted` flag, or any three-state variant (deliberately excluded; see Decisions).
- Other dashboard charts (KPIs, recent activity, SDTM coverage). The card-per-chart structure and the grid layout are designed to accommodate them.
- A project-wide aggregate pie across all CRF versions.
- Any Rust, server, or wire-shape change. This feature is frontend-only.
- Changing `CrfStatusChip`, the CRF list page, or any existing translation key.

## Verification gate

```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter @aegis/ui test
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter aegis-desktop build
```

Plus, after the dependency install:

- The lockfile holds exactly one `@mui/material@9.2.0` and one `@mui/system@9.2.0` snapshot (no duplicated MUI instance).
- The app renders the chart themed correctly — a visual check in `pnpm --filter aegis-desktop tauri dev`, since the theme wiring is not observable from the type checker or the unit tests.
