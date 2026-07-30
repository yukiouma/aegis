# `@aegis/ui` Components Relocation — Design

**Date:** 2026-07-29
**Status:** Approved (pending spec review)
**Scope:** Move `lib/packages/ui/components/Sidebar/` into `lib/packages/ui/src/components/Sidebar/`, drop the now-redundant `./Sidebar` subpath from `package.json` exports, and update the import paths and `tsconfig.json` `include` accordingly. The Sidebar is the only component today; the relocation establishes the canonical `src/components/` location for all future components.

---

## 1. Goals

1. Relocate `components/Sidebar/` to `src/components/Sidebar/` so that every package source file lives under `src/`.
2. Drop the `./Sidebar` subpath from `package.json` `exports` — the only consumer (`aegis-desktop`) imports `Sidebar` from the root `@aegis/ui`, not from the subpath.
3. Update the root barrel (`src/index.ts`) to import from the new location.
4. Update `tsconfig.json` `include` so the new path is covered.
5. Update the older `@aegis/ui` package spec so its directory tree matches the actual layout.

---

## 2. Background — why the move

The package's other source (`src/mui/`, `src/icons/`, `src/theme/`) already lives under `src/`. The lone `components/` directory at the package root is an outlier left over from the original scaffold. Putting components under `src/components/`:

- Groups all package source in one tree, simplifying mental model and tooling.
- Removes the asymmetry where `./mui`, `./icons`, `./theme` live under `src/` but `./Sidebar` lives at the package root.
- Lets `tsconfig.json` `include` shrink to `["src", "vitest.setup.ts"]` instead of carrying a separate `"components"` entry.

The `./Sidebar` subpath was added when `components/` lived at the root, before the root barrel re-exported `Sidebar`. Now that the root re-export exists and is the consumer's entry point, the subpath is unused and only adds noise to `package.json`.

---

## 3. Directory layout — before / after

### Before

```
lib/packages/ui/
  package.json
  tsconfig.json                 (include: ["src", "components", "vitest.setup.ts"])
  vitest.config.ts
  vitest.setup.ts
  src/
    index.ts                    (re-exports from '../components/Sidebar')
    index.test.ts
    mui/index.ts
    icons/index.ts
    theme/...
  components/
    Sidebar/
      index.ts
      Sidebar.tsx
      Sidebar.test.tsx
      test-utils.tsx
      types.ts
```

### After

```
lib/packages/ui/
  package.json                  (exports no longer has "./Sidebar")
  tsconfig.json                 (include: ["src", "vitest.setup.ts"])
  vitest.config.ts
  vitest.setup.ts
  src/
    index.ts                    (re-exports from './components/Sidebar')
    index.test.ts
    mui/index.ts
    icons/index.ts
    theme/...
    components/
      Sidebar/
        index.ts
        Sidebar.tsx
        Sidebar.test.tsx
        test-utils.tsx
        types.ts
```

No file inside `components/Sidebar/` is modified. The Sidebar's own relative imports (`./Sidebar`, `./test-utils`, `./types`) are unaffected by the parent move.

---

## 4. File changes

| Path | Change | Details |
| --- | --- | --- |
| `lib/packages/ui/components/Sidebar/` | Delete | After contents are moved. |
| `lib/packages/ui/src/components/Sidebar/` | Create | 5 files moved verbatim: `index.ts`, `Sidebar.tsx`, `Sidebar.test.tsx`, `test-utils.tsx`, `types.ts`. |
| `lib/packages/ui/src/index.ts` | Modify | Two lines: `from '../components/Sidebar'` → `from './components/Sidebar'`. |
| `lib/packages/ui/tsconfig.json` | Modify | `include: ["src", "components", "vitest.setup.ts"]` → `include: ["src", "vitest.setup.ts"]`. The old `components` entry is redundant; `src` covers everything under it. |
| `lib/packages/ui/package.json` | Modify | Remove the `"./Sidebar": "./components/Sidebar/index.ts"` entry from `exports`. |

---

## 5. Consumer impact

`aegis-desktop/src/App.tsx` imports `Sidebar, MenuItem, SidebarProps` from `@aegis/ui` (the root barrel). The root barrel's public surface is unchanged — only its internal import path moves. **No change to the consumer.**

`aegis-desktop/src/App.tsx` does not import from `@aegis/ui/Sidebar`. The subpath has no consumers; dropping it is a safe deletion.

---

## 6. `package.json` `exports` — after

```json
"exports": {
  ".": "./src/index.ts",
  "./mui": "./src/mui/index.ts",
  "./icons": "./src/icons/index.ts",
  "./theme": "./src/theme/index.ts"
}
```

The four remaining subpaths are siblings of the root, all mapping to files under `src/`. No exceptions.

---

## 7. `tsconfig.json` — after

```json
"include": ["src", "vitest.setup.ts"]
```

`src` covers `src/components/Sidebar/Sidebar.test.tsx` (and any future component tests) recursively. The standalone `vitest.setup.ts` entry remains so the setup file is included in the package's typecheck even though nothing imports it.

---

## 8. Verification

- `pnpm -F @aegis/ui typecheck` PASS — confirms the import path update in `src/index.ts` and the trimmed `include`.
- `pnpm -F @aegis/ui test` PASS — all 28 tests still green (the Sidebar's own tests exercise the relative paths inside its directory; they are unaffected by the parent move).
- `pnpm --filter aegis-desktop build` PASS — confirms the root barrel still resolves `Sidebar`, `MenuItem`, `SidebarProps` after the path change.
- `git grep "@aegis/ui/Sidebar"` returns no matches — the dropped subpath is fully gone.
- `git grep "../components" lib/packages/ui/src` returns no matches — the only internal consumer of the old path was the root barrel, now updated.
- `ls lib/packages/ui/components` returns no such directory.

---

## 9. Out of scope

- Per-component subpaths like `./components/Sidebar` — YAGNI; the root re-export is enough and matches how `Sidebar` is consumed today.
- Splitting `Sidebar.tsx` into smaller files.
- Renaming any Sidebar internals.
- Touching the `theme`, `mui`, or `icons` subpaths.
- Adding a new component. This spec only relocates; new components land in `src/components/<Name>/` by convention but are not part of this work.
- Cleaning up `App.css` in the desktop app (unrelated).
- Updating the older `@aegis/ui` package spec (`docs/superpowers/specs/2026-07-28-aegis-ui-package-design.md`). That doc is a historical record of the original design; the git log already tells the relocation story. Leaving it untouched.
