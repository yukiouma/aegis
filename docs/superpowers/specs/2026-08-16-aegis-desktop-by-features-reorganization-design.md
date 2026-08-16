---
title: aegis-desktop by-features reorganization
date: 2026-08-16
status: approved
---

# aegis-desktop by-features reorganization

## Purpose

Reorganize the frontend source of `apps/desktop/aegis-desktop/src` so that
`data/`, `pages/`, and `components/` are grouped by **feature** instead of by
file kind. Today these top-level directories are flat and force every feature
to coexist in the same space; the goal is for each feature to own the data
hooks, pages, and sub-components it depends on, so feature work stays inside
one folder.

This is a **pure structural refactor**. No behavior changes, no API surface
changes, no test rewrites. Only file locations and import paths change.

## Out of scope

- No new features, no behavior changes, no test rewrites (only path updates).
- No reshuffle of the `queryKeys` namespace.
- No extracting `ProductView` into its own type module.
- No changes to the Tauri backend (`apps/server/aegis-server`).

## Features and boundaries

Eight features. `product` is merged into `project-list` because its only
consumer is the project drawer. `project-list` and `project-workspace` are
separate features because they live in different windows with different
shells.

| Feature               | Owns                                                                                    |
|-----------------------|-----------------------------------------------------------------------------------------|
| `app`                 | Authed sidebar shell + root-level settings providers (theme, i18n, lang sync)            |
| `auth`                | Login (account + domain), register, logout, current-user footer                          |
| `bootstrap`           | `/bootstrap` health + login-status probes + the pre-route redirect                       |
| `user`                | User management page (list, filter, table, update)                                       |
| `project-list`        | Project list page + sub-components + product data hooks                                  |
| `project-workspace`   | Project workspace window shell + dashboard + configuration (placeholder content for now) |
| `settings`            | Settings page + on-disk settings persistence hooks                                       |
| `home`                | Home page                                                                                |

`UserFooter.tsx` lives in `features/auth/components/` because it is the
current-user footer — it exists only because of the auth boundary, even
though `AppLayout` renders it. `SettingsSyncBridge.tsx` and
`DocumentLangSync.tsx` live in `features/app/components/` because they are
root-level providers, not settings UI. The dependency direction is:
`features/app/components/SettingsSyncBridge.tsx` imports from
`features/settings/data/persist.ts`.

## Target directory layout

```
apps/desktop/aegis-desktop/src/
├── features/
│   ├── app/
│   │   └── components/
│   │       ├── AppLayout.tsx
│   │       ├── SettingsSyncBridge.tsx
│   │       └── DocumentLangSync.tsx
│   ├── auth/
│   │   ├── data/
│   │   │   ├── login.ts               # useLogin, useLoginDomain
│   │   │   ├── logout.ts              # useLogout (closes project workspace windows)
│   │   │   ├── current-user.ts        # useCurrentUser
│   │   │   └── register.ts            # useDomainUserInfo, useRegisterUser
│   │   ├── pages/
│   │   │   ├── LoginPage.tsx
│   │   │   └── RegisterPage.tsx
│   │   ├── components/
│   │   │   └── UserFooter.tsx
│   │   └── index.ts
│   ├── user/
│   │   ├── data/
│   │   │   └── list.ts                # useListUsers, useUpdateUser
│   │   ├── pages/
│   │   │   └── UserListPage.tsx
│   │   ├── components/
│   │   │   ├── UserTable.tsx
│   │   │   └── UserFilterBar.tsx
│   │   └── index.ts
│   ├── bootstrap/
│   │   ├── data/
│   │   │   └── probes.ts              # useHealthz, useIsLoggedIn
│   │   ├── pages/
│   │   │   └── BootstrapPage.tsx
│   │   ├── redirect.ts                # was src/bootstrap-redirect.ts
│   │   └── index.ts
│   ├── project-list/
│   │   ├── data/
│   │   │   ├── projects.ts            # useListProjects, useCreateProject,
│   │   │   │                          # useUpdateProject, useProject
│   │   │   └── products.ts            # useListProducts
│   │   ├── pages/
│   │   │   └── ProjectListPage.tsx
│   │   ├── components/
│   │   │   ├── ProjectTable.tsx
│   │   │   ├── ProjectFilterBar.tsx
│   │   │   └── ProjectDrawer.tsx
│   │   └── index.ts
│   ├── project-workspace/
│   │   ├── pages/
│   │   │   ├── ProjectWorkspaceLayout.tsx
│   │   │   ├── ProjectDashboardPage.tsx
│   │   │   └── ProjectConfigurationPage.tsx
│   │   └── index.ts
│   ├── settings/
│   │   ├── data/
│   │   │   └── persist.ts             # useHydrateSettingsFromStore,
│   │   │                              # useListenForSettingsChanges, persistSettings
│   │   ├── pages/
│   │   │   └── SettingsPage.tsx
│   │   └── index.ts
│   └── home/
│       ├── pages/
│       │   └── HomePage.tsx
│       └── index.ts
├── shared/
│   ├── api/                           # api object + types + ApiError (was src/api/)
│   ├── query/                         # QueryProvider + queryKeys (was src/data/client.tsx + src/data/queryKeys.ts)
│   │   ├── client.tsx
│   │   └── keys.ts
│   └── components/
│       └── BootstrapLog/              # used by auth (Login, Register) + bootstrap
├── routes/                            # flat (TanStack Router requirement)
├── test/
│   ├── shared/...
│   ├── features/<feature>/...
│   └── helpers/                       # test utilities (file-route-utils, render-with-query-client,
│                                      # test-query-provider, tauri-mock, setup)
├── main.tsx
└── vite-env.d.ts
```

After the migration the top-level directories `api/`, `components/`,
`data/`, `pages/` no longer exist; they are replaced by `features/` and
`shared/`.

## Conventions

### File naming

- **Pages**: PascalCase + `Page` suffix: `LoginPage.tsx`, `UserListPage.tsx`,
  `ProjectListPage.tsx`, `SettingsPage.tsx`, `HomePage.tsx`,
  `BootstrapPage.tsx`. Layouts are not pages, so they keep their existing
  names: `AppLayout.tsx`, `ProjectWorkspaceLayout.tsx`.
- **Data hooks**: camelCase file name matching the primary hook. Multiple
  hooks per file when they share an invalidation contract (e.g. `login.ts`
  holds `useLogin` + `useLoginDomain`).
- **Components**: PascalCase matching the component name.
- **Shared**: keep existing names — `BootstrapLog.tsx`, `useBootstrapLog.ts`,
  `types.ts`.

### Page vs component

- **Page**: consumed by a route file → `pages/`.
- **Sub-component**: consumed by another component or page → `components/`.

### Public surface — barrel `index.ts`

Each feature gets an `index.ts` that exports only what other features
should consume. Page and component modules are not re-exported; route
files and `main.tsx` import those directly by path.

- `features/auth/index.ts` → `useLogin`, `useLoginDomain`, `useLogout`,
  `useCurrentUser`, `useDomainUserInfo`, `useRegisterUser`.
- `features/user/index.ts` → `useListUsers`, `useUpdateUser`.
- `features/bootstrap/index.ts` → `useHealthz`, `useIsLoggedIn`,
  `shouldRedirectToBootstrap`.
- `features/project-list/index.ts` → `useListProjects`, `useCreateProject`,
  `useUpdateProject`, `useProject`, `useListProducts`.
- `features/project-workspace/index.ts` → no exports.
- `features/settings/index.ts` → `useHydrateSettingsFromStore`,
  `useListenForSettingsChanges`, `persistSettings`.
- `features/home/index.ts` → no exports.
- `features/app/index.ts` → no exports.
- `shared/api/index.ts` → existing barrel stays.
- `shared/query/index.ts` → exports `QueryProvider`, `queryClient`,
  `queryKeys`.
- `shared/components/BootstrapLog/index.ts` → existing barrel stays.

### Import rules

Barrels carry **data hooks** — the feature's runtime API surface. Pages
and components are imported by path because they are rendering targets,
not API consumers. This means a feature like `app` can import
`useCurrentUser` from `features/auth`'s barrel but must import
`UserFooter` directly from `features/auth/components/UserFooter.tsx`.

- **Inside a feature**: import sibling files directly (e.g.
  `features/user/pages/UserListPage.tsx` imports `./UserTable`,
  `./UserFilterBar`).
- **Cross-feature — hooks**: import from the other feature's `index.ts`
  barrel (e.g. `features/app/components/AppLayout.tsx` imports
  `useCurrentUser` from `../../auth`; `features/app/components/SettingsSyncBridge.tsx`
  imports `useHydrateSettingsFromStore` from `../../settings`). Never reach
  into `features/<x>/data/<file>.ts`.
- **Cross-feature — components / pages**: import directly by path (e.g.
  `features/app/components/AppLayout.tsx` imports `UserFooter` from
  `../../auth/components/UserFooter`).
- **Cross-cutting**: import from `shared/api`, `shared/query`,
  `shared/components/BootstrapLog`.
- **Route files**: import the page directly from the feature's `pages/`
  (e.g. `routes/_authed/_layout/management/users.tsx` imports
  `UserListPage` from
  `../../../../features/user/pages/UserListPage`).
- **`main.tsx`**: imports `QueryProvider` from `shared/query`,
  `SettingsSyncBridge` + `DocumentLangSync` from
  `features/app/components/...`, `routeTree` from `./routes/routeTree.gen`.

### Test layout

`src/test/` mirrors the new layout:

- `test/shared/{api,components,query}/...` — tests for cross-cutting modules.
- `test/features/<feature>/...` — tests for each feature (e.g.
  `test/features/user/user-list.test.tsx`,
  `test/features/project-list/project-drawer.test.tsx`).
- `test/helpers/...` — consolidated location for
  `file-route-utils.tsx`, `render-with-query-client.tsx`,
  `test-query-provider.tsx`, `tauri-mock.ts`, `setup.ts`.

Tests are moved with their source; import paths are updated in lockstep.
No test rewrites; assertions and behaviors stay byte-identical.

## Migration mechanics

Single commit. Mostly mechanical:

1. Move files to new locations. File contents stay byte-identical except for
   import paths.
2. Update internal imports (e.g. `ProjectListPage.tsx` previously did
   `import { ProjectDrawer } from "./ProjectDrawer"` → now
   `import { ProjectDrawer } from "../components/ProjectDrawer"`).
3. Update cross-feature imports to go through barrels (e.g. `AppLayout.tsx`
   did `import { useCurrentUser } from "../data"` → now
   `import { useCurrentUser } from "../../auth"`).
4. Update `main.tsx` to import `QueryProvider` from `shared/query` and
   `SettingsSyncBridge` + `DocumentLangSync` from
   `features/app/components/...`.
5. Rewrite route files to point at the new feature pages. These stay pure
   pass-throughs; only the import path changes.
6. Move tests to `test/features/<feature>/...` and `test/shared/...` with
   updated import paths. `test/helpers/` consolidates the existing test
   utilities.
7. Add barrel `index.ts` files for each feature and `shared/query/`.
8. Delete the now-empty old directories: `src/api/`, `src/components/`,
   `src/data/`, `src/pages/`.

## What must NOT change

- `routeTree.gen.ts` — generated by `@tanstack/router-plugin`. Stays in
  `routes/`. URL paths are unchanged; the generated tree is structurally
  identical.
- `bootstrap-redirect.ts` logic — moves to `features/bootstrap/redirect.ts`,
  behavior unchanged.
- `SettingsSyncBridge.tsx` and `DocumentLangSync.tsx` behavior — only the
  import paths change (settings persistence hooks now come from
  `features/settings/data/persist.ts`).
- The `queryKeys.ts` shape — moves to `shared/query/keys.ts`. Keys stay
  grouped (`auth.loginStatus`, `bootstrap.health`, `user.*`, `project.*`,
  `product.*`). Reshuffling the namespace is outside scope.
- All component behaviors, prop interfaces, hook return types, tests'
  assertions.

## Verification

All must pass before merge:

1. `pnpm --filter aegis-desktop typecheck` — zero errors.
2. `pnpm --filter aegis-desktop test` — all tests pass with the same count
   and names as before.
3. Manual smoke (optional but recommended): launch dev server, exercise `/`,
   `/login`, `/register`, `/bootstrap`, `/projects`, `/settings`,
   `/management/users`, and the `/project/<code>` workspace window.

## Risks + mitigations

| Risk | Mitigation |
|---|---|
| Missed import → typecheck failure | Run `pnpm --filter aegis-desktop typecheck` after the move; fix until clean. |
| Missed import → runtime failure (Tauri-only paths) | Run `pnpm --filter aegis-desktop test`; existing tests cover bootstrap, login, register, project-list, user-list, workspace-layout flows. |
| Test paths broken | Tests moved alongside source; imports updated in lockstep. `pnpm test` must stay green. |
| Generated `routeTree.gen.ts` out of sync | After route files are touched, the router plugin auto-regens on the next dev run. |
| Accidental functional change | Move files byte-for-byte; only edit import paths. Single commit so review reads it as a pure rename. |
| Old `data/`, `pages/`, `components/`, `api/` directories left behind | Explicit deletion step at the end; grep for stray references. |