# Aegis Desktop — Sidebar Reparent — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorder the authenticated sidebar so `Knowledge Base` sits immediately above `Management`, and remove the `Terminology` sidebar entry (route files and feature module untouched).

**Architecture:** A four-task plan: remove the three now-unused `nav.terminology*` i18n keys, redirect the four AppLayout tests that assert the (about-to-be-removed) Terminology entry so they assert the now-correct `Knowledge Base` entry, then drop the `terminologyEntry` declaration and re-shuffle `baseMenu` in `AppLayout.tsx`. Final step runs typecheck + the AppLayout/metadata test suites + a smoke build.

**Tech Stack:** React 19, TanStack Router (file-based routes), Material UI, `@aegis/ui/i18n` bilingual catalog (`en`, `zh-CN`), Vitest + Testing Library.

---

## Global Constraints

These rules apply to every task:

- **Bilingual i18n:** `TranslationKey` is derived from `typeof en` in `lib/packages/ui/src/i18n/types.ts`. Removing a key from `en.ts` without removing it from `zhCN.ts` (or vice-versa) does not break the union, but keeping stale unused keys is prohibited by the spec. Remove from both in the same commit.
- **Sidebar pattern:** A parent menu entry with `subMenu` uses a placeholder `link` (e.g. `#management`). Only sub-menu items navigate. The `terminologyEntry` we are removing followed this pattern.
- **Slice boundaries in `AppLayout`:** The numeric slice values `slice(0, 3)` and `slice(3)` are unchanged by this refactor — they survive a length-5 `baseMenu` becoming length-4 — but their semantic content shifts. `slice(3)` now yields `[settings]` instead of `[metadataEntry, settings]`.
- **Test pattern:** Existing AppLayout tests use `findByText` / `getByText` against rendered sidebar labels. New tests follow `apps/desktop/aegis-desktop/src/test/features/app/app-layout-knowledge-base.test.tsx`.

---

## File Map

**Modified:**

| Path | Change |
| --- | --- |
| `lib/packages/ui/src/i18n/locales/en.ts` | Remove `nav.terminology`, `nav.terminology.sdtm`, `nav.terminology.adam`. |
| `lib/packages/ui/src/i18n/locales/zhCN.ts` | Same three keys removed. |
| `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx` | Drop `terminologyEntry`, drop the three icon aliases and their imports, drop `terminologyEntry` from `baseMenu`. |
| `apps/desktop/aegis-desktop/src/test/features/app/app-layout.test.tsx` | Replace four `findByText("Terminology")` assertions with `findByText("Knowledge Base")`, rename the four `it` titles, update one comment. |

**Added:** none.
**Removed:** none.

---

## Task 1: Remove the three unused `nav.terminology*` i18n keys

**Files:**
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

These three keys become unused once `AppLayout.tsx` drops the `terminologyEntry` declaration in Task 3. Per-page terminology keys (`terminology.codelist.*`, `terminology.search.*`, …) stay — they are still rendered by the terminology pages themselves.

- [ ] **Step 1: Delete the keys from `en.ts`**

Open `lib/packages/ui/src/i18n/locales/en.ts` and delete the following three lines (currently on lines 148-150):

```ts
  'nav.terminology': 'Terminology',
  'nav.terminology.sdtm': 'SDTM',
  'nav.terminology.adam': 'ADaM',
```

Do not modify any other key.

- [ ] **Step 2: Delete the same keys from `zhCN.ts`**

Open `lib/packages/ui/src/i18n/locales/zhCN.ts` and delete the same three lines (currently on lines 146-148):

```ts
  'nav.terminology': '术语',
  'nav.terminology.sdtm': 'SDTM',
  'nav.terminology.adam': 'ADaM',
```

Do not modify any other key.

- [ ] **Step 3: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS. Removing keys from `en.ts` narrows the `TranslationKey` union in `lib/packages/ui/src/i18n/types.ts`; nothing should still reference these keys. If typecheck fails with "Property 'nav.terminology' does not exist on type …", grep for the missing reference:

```bash
grep -rn "nav.terminology" apps/desktop/aegis-desktop/src lib/packages/ui/src
```

Any hit other than the locales themselves means a stale reference — fix it before continuing.

- [ ] **Step 4: Commit**

```bash
git add lib/packages/ui/src/i18n/locales/en.ts lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(i18n): drop unused nav.terminology keys"
```

---

## Task 2: Update the four AppLayout tests to expect Knowledge Base

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/app/app-layout.test.tsx`

After Task 3 drops the Terminology sidebar entry, the four existing `findByText("Terminology")` assertions will fail. We update them now so that running them after Task 3 turns them green.

The four tests live inside `describe("AppLayout (role-based menu visibility)", …)` (around lines 99-152). The unrelated "AppLayout (authenticated)" and "AppLayout (unauthenticated)" blocks are not touched.

- [ ] **Step 1: Run the four tests to confirm they currently pass**

Run: `cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/app/app-layout.test.tsx`
Expected: 11 passing, 0 failing. (The Terminology tests pass right now because the entry still exists.)

- [ ] **Step 2: Update the four assertions and four titles**

In `apps/desktop/aegis-desktop/src/test/features/app/app-layout.test.tsx`, make the following edits inside the `describe("AppLayout (role-based menu visibility)", …)` block:

1. Rename:
   - `it("shows the Terminology entry for a general (non-manager) user", …)` → `it("shows the Knowledge Base entry for a general (non-manager) user", …)`
   - `it("shows the Terminology entry for an admin user", …)` → `it("shows the Knowledge Base entry for an admin user", …)`
   - `it("shows the Terminology entry for a root user", …)` → `it("shows the Knowledge Base entry for a root user", …)`
   - `it("still surfaces Terminology when current_user has not yet resolved", …)` → `it("still surfaces Knowledge Base when current_user has not yet resolved", …)`

2. Inside each of those four `it()` blocks, replace every `expect(await screen.findByText("Terminology")).toBeInTheDocument();` with `expect(await screen.findByText("Knowledge Base")).toBeInTheDocument();`. There are exactly four such assertions.

3. Update the comment inside the last test (currently `// Without mocking current_user the query errors out, so role is // undefined. The menu falls into the non-manager branch, which now // includes Terminology for everyone.`) by replacing `Terminology` with `Knowledge Base` so the comment matches the new assertion.

Do not change the existing assertions on `Management` (still present for admin/root, still absent for general).

- [ ] **Step 3: Run the four tests — they now fail (RED)**

Run: `cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/app/app-layout.test.tsx`
Expected: 7 passing, 4 failing. The four renamed tests fail because `screen.findByText("Knowledge Base")` still resolves (since `metadataEntry` is in `baseMenu`), but the assertion now reads "Knowledge Base" while the sidebar still renders "Terminology" — and after Task 3 it will not render "Terminology" either. Wait — actually right now `metadataEntry` is already rendered as "Knowledge Base" (from Task 5 of the metadata plan), so `findByText("Knowledge Base")` does resolve even before Task 3. The four tests fail because `getByText("Terminology")` would no longer find anything in the final state; for the in-between state, the renamed tests must verify that `Knowledge Base` is found and the absence of `Terminology` is no longer asserted. In practice the failure mode after Step 2 is: the test asserts "Knowledge Base" (passes) but the original 4 tests we modified removed a `Terminology` lookup, so we don't see the failing shape directly — instead the `describe("AppLayout (authenticated)")` block's tests should still pass, the renamed role-visibility tests should pass (because `Knowledge Base` already renders), and the test count is now 11 passing.

If the count is 11 passing already, that's fine — Step 2 is a preparation step. The actual RED will come in Task 3 only if a test elsewhere depends on the Terminology entry being absent. If we want a strict RED for this task, skip this step and proceed to Task 3.

(If a strict RED is preferred: add a temporary `expect(screen.queryByText("Terminology")).not.toBeInTheDocument();` after each of the four `findByText("Knowledge Base")` calls. After Task 3, drop these temporary assertions in Task 3's commit. Skip this variant unless the engineer wants a clear RED signal.)

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/app/app-layout.test.tsx
git commit -m "test(desktop): assert Knowledge Base sidebar entry instead of Terminology"
```

---

## Task 3: Drop the Terminology entry from AppLayout

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx`

Remove the `terminologyEntry` declaration, the three icon aliases it references (`TerminologyMenuIcon`, `SdtmMenuIcon`, `AdamMenuIcon`), the three corresponding icon imports (`MenuBook`, `Storage`, `Analytics`), and the `terminologyEntry` line in `baseMenu`. The numeric slice boundaries (`slice(0, 3)` / `slice(3)`) stay unchanged — they survive the length-5 → length-4 transition.

- [ ] **Step 1: Drop the icon imports**

In `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx`, replace the existing icon import block (lines 5-16) with:

```tsx
import {
  AdminPanelSettings as AdminPanelSettingsIcon,
  Description as DescriptionIcon,
  Home as HomeIcon,
  LibraryBooks as LibraryBooksIcon,
  People as PeopleIcon,
  Settings as SettingsIcon,
  Workspaces as WorkspacesIcon,
} from "@aegis/ui/icons";
```

This drops `MenuBook`, `Storage`, and `Analytics` from the imports.

- [ ] **Step 2: Drop the icon-component aliases**

Replace the existing icon-component aliases (lines 21-30) with:

```tsx
const HomeMenuIcon = () => <HomeIcon />;
const ProjectsMenuIcon = () => <WorkspacesIcon />;
const SettingsMenuIcon = () => <SettingsIcon />;
const ManagementMenuIcon = () => <AdminPanelSettingsIcon />;
const UsersMenuIcon = () => <PeopleIcon />;
const KnowledgeBaseMenuIcon = () => <LibraryBooksIcon />;
const MetadataMenuIcon = () => <DescriptionIcon />;
```

This drops `TerminologyMenuIcon`, `SdtmMenuIcon`, and `AdamMenuIcon`.

- [ ] **Step 3: Drop the `terminologyEntry` declaration**

Delete the entire `terminologyEntry` declaration (lines 60-76 in the current file):

```tsx
const terminologyEntry: MenuItem = {
  link: "#terminology",
  title: t("nav.terminology"),
  icon: TerminologyMenuIcon,
  subMenu: [
    {
      link: "/terminology/sdtm",
      title: t("nav.terminology.sdtm"),
      icon: SdtmMenuIcon,
    },
    {
      link: "/terminology/adam",
      title: t("nav.terminology.adam"),
      icon: AdamMenuIcon,
    },
  ],
};
```

Leave the `metadataEntry` declaration (which sits just below it) untouched.

- [ ] **Step 4: Drop the `terminologyEntry` line from `baseMenu`**

Replace the `baseMenu` array (lines 91-97) with:

```tsx
const baseMenu: MenuItem[] = [
  { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
  { link: "/projects", title: t("nav.projects"), icon: ProjectsMenuIcon },
  metadataEntry, // Knowledge Base (submenu: Metadata)
  { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
];
```

This drops the `terminologyEntry` line. `baseMenu` is now length-4.

- [ ] **Step 5: Refresh the inline slice comment**

In the `canManage` branch, the inline comments read `// Home, Projects, Terminology` and `// Knowledge Base, Settings`. Update them to reflect the new `baseMenu`:

```tsx
const menu: MenuItem[] = canManage
  ? [
      ...baseMenu.slice(0, 3), // Home, Projects, Knowledge Base
      managementEntry, // Management (submenu: Users)
      ...baseMenu.slice(3), // Settings
    ]
  : baseMenu;
```

The slice numeric values (`slice(0, 3)` and `slice(3)`) are unchanged — `slice(3)` now yields `[settings]`, which is what we want.

- [ ] **Step 6: Run the AppLayout and Metadata test suites — they should all pass**

Run:
```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/app/ src/test/features/metadata/
```

Expected: All tests pass.

- [ ] **Step 7: Typecheck**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS. The removed icon imports / aliases / `terminologyEntry` reference are gone, and `tsc` should compile cleanly.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx
git commit -m "feat(desktop): drop Terminology sidebar entry, reparent Knowledge Base"
```

---

## Task 4: Final verification

**Files:** none — read-only checks.

- [ ] **Step 1: Typecheck the whole desktop app**

Run: `cd apps/desktop/aegis-desktop && pnpm typecheck`
Expected: PASS.

- [ ] **Step 2: Run the AppLayout + metadata + terminology test suites**

Run:
```bash
cd apps/desktop/aegis-desktop && pnpm vitest run src/test/features/app/ src/test/features/metadata/ src/test/features/terminology/
```

Expected: All tests pass. The terminology page / component / data-hook tests continue to pass because only the sidebar entry was removed — the feature module itself is untouched.

- [ ] **Step 3: Smoke-build the desktop app**

Run: `cd apps/desktop/aegis-desktop && pnpm build`
Expected: Build succeeds. The bundle still emits `terminology-*` chunks (the pages still exist) and `metadata-*.js` (still registered).

- [ ] **Step 4: Manual smoke check (optional, only if a running Tauri shell is available)**

Launch the desktop app, log in, and confirm:
- The sidebar shows `Home · Projects · Knowledge Base (▸ Metadata) · Settings` for a general user.
- For an admin/root user, `Management (▸ Users)` sits between `Knowledge Base` and `Settings`.
- The `Terminology` label is not in the sidebar.
- Clicking the SDTM card's `Terminology` row on `/metadata` still navigates to `/terminology/sdtm` (proves the route is still reachable).

This step is optional — if no running shell is available, the test suite and build are sufficient verification.

- [ ] **Step 5: No further commit**

If everything in Steps 1–3 passes, the work is complete. No commit is created in this task; the previous tasks' commits form the change set.

---

## Self-Review

Coverage map (spec → task):

| Spec section / requirement | Task |
| --- | --- |
| §1.1 new sidebar order (admin + non-admin) | T3 |
| §1.2 remove Terminology sidebar entry + sub-menu | T3 |
| §1.3 keep `/terminology/sdtm` and `/terminology/adam` reachable | (verified by T4 step 2 + step 4) |
| §1.4 Terminology feature module unchanged | (out-of-scope commitment, no task needed) |
| §1.5 remove three unused `nav.terminology*` i18n keys | T1 |
| §1.6 update existing AppLayout tests | T2 |
| §2 new sidebar order | T3 (visualised in T3 step 5 comments and T4 step 4) |
| §3 file map | T1, T2, T3 |
| §4 AppLayout change in detail | T3 |
| §5 test updates (4 assertions, 4 titles, 1 comment) | T2 |
| §6 i18n key removal (3 keys × 2 locales) | T1 |
| §7 error handling — n/a | — |
| §8 out of scope — no task implements removed-route deletion | — |

No placeholders ("TBD", "TODO", "implement later", "similar to", "fill in details") remain. Type / function names match across tasks (`terminologyEntry`, `metadataEntry`, `baseMenu`, slice indices). The slice boundary is verified: `baseMenu` length-4 → `slice(0, 3)` = `[home, projects, knowledgeBase]`, `slice(3)` = `[settings]`. `Management` therefore lands between `Knowledge Base` and `Settings` for admin users, as the spec requires.