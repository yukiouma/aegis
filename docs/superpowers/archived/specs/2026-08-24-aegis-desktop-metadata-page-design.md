# Aegis Desktop — Metadata Page — Design

**Date:** 2026-08-24
**Status:** Approved (pending spec review)
**Scope:** Add a `Metadata` page to `aegis-desktop` that acts as a navigation hub over the existing SDTM / ADaM data surfaces (`Domain Model` — disabled for now — and `Terminology`, which links out to the existing `/terminology/sdtm` and `/terminology/adam` routes). Includes a new `Knowledge Base` sidebar entry with `Metadata` as its sole sub-menu item, the route file, the page component, and matching i18n keys in both `en` and `zh-CN`.

---

## 1. Goals

1. Add a new top-level sidebar entry `Knowledge Base` immediately before `Settings`, with a single sub-menu item `Metadata` that navigates to the new `Metadata` page.
2. Add the `Metadata` page at path `/metadata` (flat, not nested under `Knowledge Base`) so it matches the existing top-level page convention (`/terminology/*`, `/settings`, `/projects`).
3. The `Metadata` page shows two side-by-side MUI `Card`s — `SDTM` on the left, `ADaM` on the right — each containing two `ListItem`s: `Domain Model` (disabled, with a "Coming soon" tooltip) and `Terminology` (navigates to the matching existing terminology page).
4. Reuse the existing `/terminology/sdtm` and `/terminology/adam` routes by navigation only — no prop drilling, no shared state, no new React Query hooks.
5. Follow the existing feature-folder convention: `features/metadata/{pages,index.ts}`, mirroring the layout used by `features/terminology` and `features/settings`.
6. Match the existing MUI-based UI language and the bilingual i18n catalog.

Non-goals:
- Implementing the `Domain Model` feature (placeholder only).
- Any change to the server, the `terminology` crate, or shared API types.
- Pagination, search, or any data fetching on the Metadata page itself.

---

## 2. URL map

| Path        | Route file                                                              | Component       |
| ----------- | ----------------------------------------------------------------------- | --------------- |
| `/metadata` | `apps/desktop/aegis-desktop/src/routes/_authed/_layout/metadata.tsx`    | `MetadataPage`  |

The page is mounted under the existing `/_authed/_layout/` authenticated layout, so it inherits `AppLayout` (sidebar + footer) automatically — same as `/_authed/_layout/settings`.

---

## 3. Files added / changed / removed

### 3.1 Added

| Path                                                                                  | Responsibility |
| ------------------------------------------------------------------------------------- | -------------- |
| `apps/desktop/aegis-desktop/src/routes/_authed/_layout/metadata.tsx`                  | Route file, mounts `MetadataPage`. |
| `apps/desktop/aegis-desktop/src/features/metadata/pages/MetadataPage.tsx`             | Page component: heading + two side-by-side MUI `Card`s. |
| `apps/desktop/aegis-desktop/src/features/metadata/pages/MetadataPage.test.tsx`        | Vitest + RTL coverage for the page. |
| `apps/desktop/aegis-desktop/src/features/metadata/index.ts`                           | Feature barrel re-exporting `MetadataPage`. |

### 3.2 Modified

| Path                                                                                  | Change |
| ------------------------------------------------------------------------------------- | ------ |
| `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx`                | Add `metadataEntry` (`#metadata` link, `LibraryBooks` icon, sub-menu `Metadata` with `Description` icon). Insert it into `baseMenu` between `terminologyEntry` and the `settings` entry. Update the `canManage` spread so `Management` stays between `Terminology` and `Knowledge Base`. |
| `lib/packages/ui/src/i18n/locales/en.ts`                                              | + 8 keys: `nav.knowledgeBase`, `nav.metadata`, `metadata.heading`, `metadata.block.sdtm`, `metadata.block.adam`, `metadata.item.domainModel`, `metadata.item.terminology`, `metadata.disabled.tooltip`. |
| `lib/packages/ui/src/i18n/locales/zhCN.ts`                                            | Matching zh-CN translations. |
| `apps/desktop/aegis-desktop/src/routes/routeTree.gen.ts`                              | Regenerated automatically by `@tanstack/router-plugin`. Not edited by hand. |

### 3.3 Removed

None.

---

## 4. Sidebar structure

After this change, the sidebar order is:

**Non-admin user:**
`Home` · `Projects` · `Terminology` (▸ `SDTM`, `ADaM`) · `Knowledge Base` (▸ `Metadata`) · `Settings`

**Admin / root user:**
`Home` · `Projects` · `Terminology` (▸ `SDTM`, `ADaM`) · `Management` (▸ `Users`) · `Knowledge Base` (▸ `Metadata`) · `Settings`

The parent `Knowledge Base` label itself does not navigate (link `#metadata`) — clicking it only toggles the sub-menu open/closed, matching the existing `Terminology` and `Management` patterns.

---

## 5. Component layout

```
┌────────────────────────────────────────────────────────────────┐
│  Metadata                                                       │  ← Typography h4
│                                                                 │
│  ┌──────────────────────────┐   ┌──────────────────────────┐    │
│  │ SDTM                     │   │ ADaM                     │    │
│  ├──────────────────────────┤   ├──────────────────────────┤    │
│  │ [icon] Domain Model  ⓘ   │   │ [icon] Domain Model  ⓘ   │    │  ← disabled, tooltip "Coming soon"
│  │ [icon] Terminology    ›  │   │ [icon] Terminology    ›  │    │  ← navigates
│  └──────────────────────────┘   └──────────────────────────┘    │
└────────────────────────────────────────────────────────────────┘
```

Concrete element shape (`features/metadata/pages/MetadataPage.tsx`):

```tsx
<Box sx={{ p: 4, display: "flex", flexDirection: "column", gap: 3 }}>
  <Typography variant="h4">{t("metadata.heading")}</Typography>

  <Box sx={{ display: "flex", gap: 3, flexWrap: "wrap" }}>
    {(["sdtm", "adam"] as const).map((kind) => (
      <Card key={kind} sx={{ flex: 1, minWidth: 320 }}>
        <CardHeader
          title={t(kind === "sdtm" ? "metadata.block.sdtm" : "metadata.block.adam")}
        />
        <List disablePadding>
          <ListItem disablePadding>
            <Tooltip title={t("metadata.disabled.tooltip")}>
              {/* span wrapper so Tooltip can attach to a disabled button */}
              <span style={{ width: "100%" }}>
                <ListItemButton disabled>
                  <ListItemIcon>
                    <ArchitectureIcon />  {/* generic schema/blueprint icon */}
                  </ListItemIcon>
                  <ListItemText primary={t("metadata.item.domainModel")} />
                </ListItemButton>
              </span>
            </Tooltip>
          </ListItem>
          <ListItem disablePadding>
            <ListItemButton
              onClick={() =>
                navigate({
                  to: kind === "sdtm" ? "/terminology/sdtm" : "/terminology/adam",
                })
              }
            >
              <ListItemIcon>
                {kind === "sdtm" ? <StorageIcon /> : <AnalyticsIcon />}
              </ListItemIcon>
              <ListItemText primary={t("metadata.item.terminology")} />
            </ListItemButton>
          </ListItem>
        </List>
      </Card>
    ))}
  </Box>
</Box>
```

Notes on the implementation:
- `ArchitectureIcon` (from `@aegis/ui/icons`, which re-exports `@mui/icons-material`) for `Domain Model` rows — generic "schema/model" feel without committing to a future domain-model visual.
- `StorageIcon` / `AnalyticsIcon` for the `Terminology` rows, mirroring the icons already used for the SDTM / ADaM sub-menu entries in the sidebar.
- The disabled `ListItemButton` is wrapped in a `<span>` because MUI's `Tooltip` will not show on a natively disabled element. Standard MUI pattern.
- Cards use `flex: 1` and `minWidth: 320` so the layout collapses to a vertical stack on narrow viewports rather than clipping content.

---

## 6. AppLayout menu wiring

In `apps/desktop/aegis-desktop/src/features/app/components/AppLayout.tsx`:

```tsx
const MetadataMenuIcon = () => <LibraryBooksIcon />;
const MetadataSubMenuIcon = () => <DescriptionIcon />;

const metadataEntry: MenuItem = {
  link: "#metadata",
  title: t("nav.knowledgeBase"),
  icon: MetadataMenuIcon,
  subMenu: [
    {
      link: "/metadata",
      title: t("nav.metadata"),
      icon: MetadataSubMenuIcon,
    },
  ],
};

const baseMenu: MenuItem[] = [
  { link: "/", title: t("nav.home"), icon: HomeMenuIcon },
  { link: "/projects", title: t("nav.projects"), icon: ProjectsMenuIcon },
  terminologyEntry,
  metadataEntry,                 // ← new
  { link: "/settings", title: t("nav.settings"), icon: SettingsMenuIcon },
];

const menu: MenuItem[] = canManage
  ? [
      ...baseMenu.slice(0, 3),   // Home, Projects, Terminology
      managementEntry,
      ...baseMenu.slice(3),      // Knowledge Base, Settings
    ]
  : baseMenu;
```

The `slice(0, 3)` / `slice(3)` boundary moves from index `3` to index `3` (unchanged numeric value, but its semantic content shifts): `slice(3)` now yields `[metadataEntry, settings]` instead of `[settings]`. `Management` therefore lands between `Terminology` and `Knowledge Base` for admin users, as required.

`LibraryBooksIcon` and `DescriptionIcon` are added to the existing import from `@aegis/ui/icons`.

---

## 7. i18n keys

Both `en` and `zh-CN` catalogs gain the same set of keys. `TranslationKey` is derived from `typeof en`, so the union type updates automatically.

| Key                            | en                | zh-CN       |
| ------------------------------ | ----------------- | ----------- |
| `nav.knowledgeBase`            | `Knowledge Base`  | `知识库`    |
| `nav.metadata`                 | `Metadata`        | `元数据`    |
| `metadata.heading`             | `Metadata`        | `元数据`    |
| `metadata.block.sdtm`          | `SDTM`            | `SDTM`      |
| `metadata.block.adam`          | `ADaM`            | `ADaM`      |
| `metadata.item.domainModel`    | `Domain Model`    | `域模型`    |
| `metadata.item.terminology`    | `Terminology`     | `术语`      |
| `metadata.disabled.tooltip`    | `Coming soon`     | `敬请期待`  |

---

## 8. Testing

`apps/desktop/aegis-desktop/src/features/metadata/pages/MetadataPage.test.tsx` covers:

1. Renders the page heading and both card titles (`SDTM`, `ADaM`).
2. Renders two `Domain Model` rows and two `Terminology` rows (one of each per card).
3. Both `Domain Model` rows render as disabled (assert via `aria-disabled` or the `Mui-disabled` class).
4. Focusing / hovering a `Domain Model` row exposes the `Coming soon` tooltip text.
5. Clicking the SDTM card's `Terminology` row calls `useNavigate()` with `{ to: "/terminology/sdtm" }`.
6. Clicking the ADaM card's `Terminology` row calls `useNavigate()` with `{ to: "/terminology/adam" }`.

`useNavigate` is mocked via `vi.mock("@tanstack/react-router", ...)` in the test file. `useI18n` is provided by the test renderer (`renderWithTheme` from the UI package), same as other page tests.

No changes to `AppLayout.test.tsx` — adding a single menu entry to the existing visual-order assertion would be more boilerplate than regression-catching value. If the sidebar ordering becomes a regression hotspot later, we can add a snapshot-style test at that point.

---

## 9. Error handling

None. The page does no data fetching and has no user-input forms. The two clickable rows either navigate (success) or no-op (which cannot happen — both targets are real routes).

---

## 10. Out of scope

- The actual `Domain Model` page / feature. Only a disabled placeholder row is rendered.
- Adding the new `Metadata` (or `Knowledge Base`) entry to any other navigation surface (e.g. a future command palette, breadcrumbs, etc.).
- Server-side support for "metadata" as a concept — the server knows nothing about this change.
- Persisting the sidebar's expanded/collapsed state for `Knowledge Base` across reloads (it already resets on reload via the `useState<Set<string>>` in `Sidebar`; that is consistent with `Terminology` and `Management`).
