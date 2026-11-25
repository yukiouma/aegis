# `@aegis/ui` Components Relocation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `lib/packages/ui/components/Sidebar/` into `lib/packages/ui/src/components/Sidebar/`, drop the now-unused `./Sidebar` subpath from `package.json` exports, and update import paths so all package source lives under `src/`.

**Architecture:** Pure file relocation. The Sidebar component and its tests are moved verbatim — no file inside `components/Sidebar/` is modified. The root barrel's internal import path moves from `'../components/Sidebar'` to `'./components/Sidebar'`. The `tsconfig.json` `include` drops the redundant `"components"` entry. The `./Sidebar` subpath in `package.json` exports is removed because no consumer uses it (the only consumer, `aegis-desktop`, imports `Sidebar` from the root `@aegis/ui`).

**Tech Stack:** TypeScript 5.8, pnpm 10.33 workspaces, git for history-preserving moves.

**Spec:** [2026-07-29-aegis-ui-components-relocation-design.md](../specs/2026-07-29-aegis-ui-components-relocation-design.md)

---

## Global Constraints

These apply to every task. Do not deviate.

- Use `git mv` to relocate the directory so git history is preserved.
- Do not modify any file inside `components/Sidebar/` — only move them.
- Run `pnpm -F @aegis/ui typecheck`, `pnpm -F @aegis/ui test`, and `pnpm --filter aegis-desktop build` after every task. Fix any errors before committing.
- Commit messages: imperative mood, ≤72 chars subject, body explains "why".
- No new runtime dependencies. No changes to the Sidebar's tests, types, or internals.

---

## File Structure

| Path | Change |
| --- | --- |
| `lib/packages/ui/components/Sidebar/` | Delete (after `git mv`) |
| `lib/packages/ui/src/components/Sidebar/` | Create (5 files moved verbatim) |
| `lib/packages/ui/src/index.ts` | Modify (2 lines: import path) |
| `lib/packages/ui/tsconfig.json` | Modify (`include` array) |
| `lib/packages/ui/package.json` | Modify (drop one entry from `exports`) |

No consumer files change. `aegis-desktop/src/App.tsx` imports `Sidebar` from the root `@aegis/ui`, which still re-exports it.

---

## Task 1: Move `components/Sidebar/` → `src/components/Sidebar/`

**Files:**
- Move: `lib/packages/ui/components/Sidebar/` (5 files) → `lib/packages/ui/src/components/Sidebar/`

**Interfaces:**
- Produces: a directory `lib/packages/ui/src/components/Sidebar/` with the same 5 files that used to be at `lib/packages/ui/components/Sidebar/`. Git history of each file is preserved (rename detection).

- [ ] **Step 1: Use `git mv` to relocate the directory**

```bash
cd d:/projects/rusty/aegis
git mv lib/packages/ui/components/Sidebar lib/packages/ui/src/components/Sidebar
```

Expected: git records 5 renames (one per file). No file contents change.

- [ ] **Step 2: Verify the move**

```bash
cd d:/projects/rusty/aegis
ls lib/packages/ui/components 2>&1 | head -3
ls lib/packages/ui/src/components/Sidebar
git status --short
```

Expected:
- `ls lib/packages/ui/components` reports "No such file or directory" (the old path is gone).
- `ls lib/packages/ui/src/components/Sidebar` lists the 5 files: `index.ts`, `Sidebar.tsx`, `Sidebar.test.tsx`, `test-utils.tsx`, `types.ts`.
- `git status --short` shows 5 renamed entries (e.g. `R  lib/...`), no modifications.

- [ ] **Step 3: Verify the build is still green**

```bash
cd d:/projects/rusty/aegis
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
```

Expected:
- `typecheck` FAILS — the root barrel still imports from `'../components/Sidebar'`, which no longer resolves. The next task fixes this.
- `test` runs and reports failures on the barrel import. (Same root cause.)

This is the expected fail state. Proceed to Task 2.

---

## Task 2: Update `src/index.ts` import paths

**Files:**
- Modify: `lib/packages/ui/src/index.ts`

**Interfaces:**
- Produces: the root barrel re-exports `Sidebar`, `MenuItem`, `SubMenuItem`, `SidebarProps` from the new `./components/Sidebar` location.

- [ ] **Step 1: Replace the two `../components/Sidebar` paths**

In `lib/packages/ui/src/index.ts`, replace:

```ts
export { Sidebar } from '../components/Sidebar';
export type { MenuItem, SubMenuItem, SidebarProps } from '../components/Sidebar';
```

with:

```ts
export { Sidebar } from './components/Sidebar';
export type { MenuItem, SubMenuItem, SidebarProps } from './components/Sidebar';
```

- [ ] **Step 2: Verify typecheck and tests pass**

```bash
cd d:/projects/rusty/aegis
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
```

Expected:
- `typecheck` PASS.
- `test` PASS — all 28 tests green. The Sidebar's relative imports (e.g. `./test-utils`, `./types`) are unaffected by the parent move and the barrel's re-export is now resolvable.

- [ ] **Step 3: Verify the consumer still builds**

```bash
cd d:/projects/rusty/aegis
pnpm --filter aegis-desktop build
```

Expected: PASS. `aegis-desktop/src/App.tsx` imports `Sidebar` from `@aegis/ui` (the root), which now resolves through `./components/Sidebar`. No change to the consumer's source.

- [ ] **Step 4: Commit**

```bash
cd d:/projects/rusty/aegis
git add lib/packages/ui/src/index.ts
# The renames from Task 1's `git mv` are already staged. Add any
# remaining rename entries if git status still shows them as unstaged.
git add -u lib/packages/ui/components 2>/dev/null || true
git commit -m "feat(ui): relocate components/Sidebar under src/components

git mv preserves the per-file history. Root barrel updated to
import from the new location; the consumer (aegis-desktop) is
unaffected because it imports Sidebar from the root @aegis/ui."
```

Note: `git mv` in Task 1 staged the renames, so they're already in the index. `git add -u lib/packages/ui/components` is a defensive fallback in case some subagent re-stages; the `|| true` swallows the expected "directory gone" error.

---

## Task 3: Update `tsconfig.json` and `package.json`

**Files:**
- Modify: `lib/packages/ui/tsconfig.json`
- Modify: `lib/packages/ui/package.json`

**Interfaces:**
- Produces:
  - `tsconfig.json` `include` is `["src", "vitest.setup.ts"]` (the redundant `"components"` entry is gone — `src` already covers the new location).
  - `package.json` `exports` no longer contains the `./Sidebar` subpath.

- [ ] **Step 1: Update `tsconfig.json` `include`**

In `lib/packages/ui/tsconfig.json`, replace the `include` array:

```json
"include": ["src", "components", "vitest.setup.ts"]
```

with:

```json
"include": ["src", "vitest.setup.ts"]
```

- [ ] **Step 2: Drop the `./Sidebar` subpath from `package.json`**

In `lib/packages/ui/package.json`, remove this single line from the `exports` object:

```json
"./Sidebar": "./components/Sidebar/index.ts",
```

(Also remove the trailing comma on the previous line if needed so the JSON stays valid.)

The result:

```json
"exports": {
  ".": "./src/index.ts",
  "./mui": "./src/mui/index.ts",
  "./icons": "./src/icons/index.ts",
  "./theme": "./src/theme/index.ts"
}
```

- [ ] **Step 3: Verify the build is still green**

```bash
cd d:/projects/rusty/aegis
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
pnpm --filter aegis-desktop build
```

Expected: all three PASS. The `tsconfig.json` change is a tightening (fewer entries) so any file that was previously typechecked remains typechecked. The `package.json` change removes an unused subpath that no consumer referenced.

- [ ] **Step 4: Verify the dropped subpath is truly gone**

```bash
cd d:/projects/rusty/aegis
git grep -n "@aegis/ui/Sidebar" -- ':!**/pnpm-lock.yaml'
```

Expected: no matches. The subpath appears nowhere in source.

- [ ] **Step 5: Verify no stale `../components` import remains**

```bash
cd d:/projects/rusty/aegis
git grep -n "'\.\./components" lib/packages/ui/src
```

Expected: no matches. The only consumer of the old import path was the root barrel, which Task 2 already updated.

- [ ] **Step 6: Verify the old directory is gone**

```bash
cd d:/projects/rusty/aegis
test ! -e lib/packages/ui/components && echo "old components/ directory removed"
```

Expected: prints `old components/ directory removed`. If the directory still exists, the `git mv` in Task 1 missed something — investigate.

- [ ] **Step 7: Commit**

```bash
cd d:/projects/rusty/aegis
git add lib/packages/ui/tsconfig.json lib/packages/ui/package.json
git commit -m "chore(ui): drop redundant tsconfig entry and unused ./Sidebar subpath

tsconfig 'include' no longer needs 'components' (src covers the new
location). package.json exports drops the dead ./Sidebar subpath —
the only consumer imports Sidebar from the root @aegis/ui."
```

---

## Task 4: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Run the full check suite**

```bash
cd d:/projects/rusty/aegis
pnpm -F @aegis/ui typecheck
pnpm -F @aegis/ui test
pnpm --filter aegis-desktop build
```

Expected: all three PASS. Same as Task 3 Step 3; this is the final gate.

- [ ] **Step 2: Spot-check the directory layout**

```bash
cd d:/projects/rusty/aegis
ls lib/packages/ui/src
ls lib/packages/ui/src/components
ls lib/packages/ui/src/components/Sidebar
```

Expected:
- `src` contains: `components`, `icons`, `index.test.ts`, `index.ts`, `mui`, `theme`.
- `src/components` contains: `Sidebar`.
- `src/components/Sidebar` contains: `index.ts`, `Sidebar.tsx`, `Sidebar.test.tsx`, `test-utils.tsx`, `types.ts`.

- [ ] **Step 3: Confirm the package's `exports` map is clean**

```bash
cd d:/projects/rusty/aegis
cat lib/packages/ui/package.json | grep -A6 '"exports"'
```

Expected: 4 entries — `.`, `./mui`, `./icons`, `./theme`. No `./Sidebar`.

- [ ] **Step 4: Commit (only if Step 1–3 surfaced a deviation that was fixed)**

If everything matches, no commit.

---

## Done Criteria

- [ ] All 3 commits on the current branch (Tasks 1, 2, 3 — Task 4 is verification only).
- [ ] `lib/packages/ui/components/` no longer exists.
- [ ] `lib/packages/ui/src/components/Sidebar/` contains the 5 expected files.
- [ ] `tsconfig.json` `include` is `["src", "vitest.setup.ts"]`.
- [ ] `package.json` `exports` has 4 entries, no `./Sidebar`.
- [ ] `pnpm -F @aegis/ui typecheck` PASS.
- [ ] `pnpm -F @aegis/ui test` PASS — 28 tests.
- [ ] `pnpm --filter aegis-desktop build` PASS.
- [ ] `git grep "@aegis/ui/Sidebar"` returns no matches.
- [ ] `git grep "'\.\./components" lib/packages/ui/src` returns no matches.
