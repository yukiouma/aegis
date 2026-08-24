# Aegis Desktop — Sidebar Reparent — Design

**Date:** 2026-08-24
**Status:** Approved (pending spec review)
**Scope:** Reorder the authenticated sidebar so `Knowledge Base` sits immediately above `Management` (instead of between `Terminology` and `Settings`). Remove the `Terminology` sidebar entry and its SDTM / ADaM sub-menu. The `/terminology/sdtm` and `/terminology/adam` routes, the terminology feature module, and the per-page terminology i18n keys remain intact — the sidebar entry only is dropped.

---

## 1. Goals

1. New sidebar order: admin users see `Home · Projects · Knowledge Base · Management · Settings`; non-admin users see `Home · Projects · Knowledge Base · Settings`.
2. The `Terminology` sidebar entry and its `SDTM` / `ADaM` sub-menu items are removed.
3. `/terminology/sdtm` and `/terminology/adam` continue to be reachable: by the Terminology rows on the Metadata page, by direct URL, and by programmatic navigation.
4. The Terminology feature module (`features/terminology/**`) is unchanged.
5. The three sidebar-nav i18n keys that become unused (`nav.terminology`, `nav.terminology.sdtm`, `nav.terminology.adam`) are removed from both `en` and `zh-CN` catalogs. Per-page terminology keys stay.
6. The existing AppLayout tests are updated so they no longer assert the (now-removed) Terminology entry; the new `app-layout-knowledge-base.test.tsx` keeps its existing assertions unchanged.

Non-goals:
- Removing any Terminology component, hook, route file, or per-page key.
- Reordering Settings or Home / Projects.
- Changing the `Knowledge Base → Metadata` sub-menu structure.
- Adding back any Terminology surface elsewhere (e.g. command palette, breadcrumbs).

---

## 2. New sidebar order

| User role | Order |
| --- | --- |
| Admin / root | `Home` → `Projects` → `Knowledge Base` (▸ `Metadata`) → `Management` (▸ `Users`) → `Settings` |
| Non-admin    | `Home` → `Projects` → `Knowledge Base` (▸ `Metadata`) → `Settings` |

`Knowledge Base` is still available to every authenticated user; `Management` is still gated to `admin` / `root`.

---

## 3. Files changed

| Path | Change |
| --- | --- |
| `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx` | Drop `terminologyEntry`, drop `TerminologyMenuIcon` / `SdtmMenuIcon` / `AdamMenuIcon`, drop `MenuBook` / `Storage` / `Analytics` icon imports, drop the `terminologyEntry` line in `baseMenu`, update the `canManage` slice so Management lands between Knowledge Base and Settings. |
| `apps/desktop/aegis-desktop/src/test/features/app/app-layout.test.tsx` | Replace 4 `findByText("Terminology")` assertions with `findByText("Knowledge Base")` and rename the four corresponding `it` titles. |
| `lib/packages/ui/src/i18n/locales/en.ts` | Remove the `nav.terminology`, `nav.terminology.sdtm`, `nav.terminology.adam` keys. |
| `lib/packages/ui/src/i18n/locales/zhCN.ts` | Same three keys removed. |

No files added. No files removed.

---

## 4. AppLayout change in detail

Before:

```tsx
const baseMenu: MenuItem[] = [
  { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
  { link: "/projects", title: t("nav.projects"), icon: ProjectsMenuIcon },
  terminologyEntry, // Terminology (submenu: SDTM, ADaM)
  metadataEntry, // Knowledge Base (submenu: Metadata)
  { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
];

const menu: MenuItem[] = canManage
  ? [
      ...baseMenu.slice(0, 3), // Home, Projects, Terminology
      managementEntry,
      ...baseMenu.slice(3), // Knowledge Base, Settings
    ]
  : baseMenu;
```

After:

```tsx
const baseMenu: MenuItem[] = [
  { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
  { link: "/projects", title: t("nav.projects"), icon: ProjectsMenuIcon },
  metadataEntry, // Knowledge Base (submenu: Metadata)
  { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
];

const menu: MenuItem[] = canManage
  ? [
      ...baseMenu.slice(0, 3), // Home, Projects, Knowledge Base
      managementEntry,
      ...baseMenu.slice(3), // Settings
    ]
  : baseMenu;
```

The numeric slice boundaries (`slice(0, 3)` and `slice(3)`) are unchanged — their semantic content shifts: `slice(3)` now yields `[settings]` instead of `[metadataEntry, settings]`. `Management` therefore lands between `Knowledge Base` and `Settings` for admin users.

`terminologyEntry`, `TerminologyMenuIcon`, `SdtmMenuIcon`, `AdamMenuIcon`, and the `MenuBook` / `Storage` / `Analytics` icon imports are removed in the same edit.

---

## 5. Test updates

`apps/desktop/aegis-desktop/src/test/features/app/app-layout.test.tsx`:

| Existing test (verbatim) | Updated test |
| --- | --- |
| `it("shows the Terminology entry for a general (non-manager) user", …)` | `it("shows the Knowledge Base entry for a general (non-manager) user", …)` |
| `expect(await screen.findByText("Terminology")).toBeInTheDocument();` | `expect(await screen.findByText("Knowledge Base")).toBeInTheDocument();` |
| `it("shows the Terminology entry for an admin user", …)` | `it("shows the Knowledge Base entry for an admin user", …)` |
| `it("shows the Terminology entry for a root user", …)` | `it("shows the Knowledge Base entry for a root user", …)` |
| `it("still surfaces Terminology when current_user has not yet resolved", …)` | `it("still surfaces Knowledge Base when current_user has not yet resolved", …)` |
| `// includes Terminology for everyone.` | `// includes Knowledge Base for everyone.` |

Four `findByText("Terminology")` assertions across four tests become `findByText("Knowledge Base")`. The two assertions on `Management` (present for admin/root, absent for general) and the comment about `current_user` not yet resolving are unchanged.

`apps/desktop/aegis-desktop/src/test/features/app/app-layout-knowledge-base.test.tsx` is unchanged.

---

## 6. i18n key removal

Three keys are removed from both `en.ts` and `zhCN.ts` (and the `TranslationKey` union, derived from `typeof en`, narrows automatically):

| Key | en (removed) | zh-CN (removed) |
| --- | --- | --- |
| `nav.terminology` | `Terminology` | `术语` |
| `nav.terminology.sdtm` | `SDTM` | `SDTM` |
| `nav.terminology.adam` | `ADaM` | `ADaM` |

All other `terminology.*` keys stay (they are used by `features/terminology/**` and the `MetadataPage`'s `terminologyTarget` strings via the per-page keys, not these nav keys).

---

## 7. Error handling

None — pure presentational reorder + key removal. No data fetching, no user input.

---

## 8. Out of scope

- Removing `/terminology/*` route files.
- Removing any feature in `features/terminology/**`.
- Removing per-page terminology i18n keys (`terminology.codelist.*`, `terminology.search.*`, etc.).
- Reordering the sub-menu items inside `Knowledge Base` (still only `Metadata`).
- Changing the `Metadata` page's Terminology-row navigation targets.