# Multi-theme support + Settings dropdown + cross-window broadcast

Date: 2026-08-18
Status: Draft (awaiting user review)
Scope: `lib/packages/ui` (theme + i18n) and `apps/desktop/aegis-desktop` (Settings page + persistence)

## Background

Six new MUI themes were added under `lib/packages/ui/src/theme/themes/` —
`anyaTheme`, `chihiroTheme`, `ntdTheme`, `siblyTheme`, `totoroTheme`, and
`xiTheme`. Each is a fully styled palette (component overrides, typography,
shape) inspired by a character. The existing `lightTheme` and `darkTheme`
are intentional placeholders for the user to flesh out later.

Today the theme system only knows about two IDs (`'light' | 'dark'`):
the `ThemeMode` union is narrow, the registry only maps two entries,
the Settings page exposes a `<Switch>` boolean toggle, and there are no
labels for any of the new themes. The cross-window broadcast plumbing
(`aegis:settings-changed` event + `settings.bin` store) already exists and
will be reused — only the payload type and the persistence format need
to widen.

## Goals

1. Make all 8 themes selectable from the Settings page.
2. Keep the existing cross-window broadcast working so a theme picked
   in the main window is visible immediately in any open `project:*`
   workspace window, and vice versa.
3. Persist the chosen theme across launches (already handled by the
   `settings.bin` store).
4. Stay within the existing patterns: i18n keys for user-visible labels,
   no new event names, no new persistence keys.

## Non-goals

- Replacing the placeholder palettes of `lightTheme` / `darkTheme`.
- Adding a per-component theme override API.
- Theming the Tauri shell (Rust side) — only the React webviews.

## Design

### Theme registry

Widen `ThemeMode` ([`lib/packages/ui/src/theme/types.ts`](lib/packages/ui/src/theme/types.ts))
to:

```ts
export type ThemeMode =
  | 'light'
  | 'dark'
  | 'anya'
  | 'chihiro'
  | 'ntd'
  | 'sibly'
  | 'totoro'
  | 'xi';
```

The IDs match the file names (`anyaTheme.ts` → `'anya'`), keeping the
mapping obvious.

Update the registry ([`lib/packages/ui/src/theme/registry.ts`](lib/packages/ui/src/theme/registry.ts))
to import the six new theme constants and map every ID to a `Theme`:

```ts
import { anyaTheme } from './themes/anyaTheme';
import { chihiroTheme } from './themes/chihiroTheme';
// ...etc
const themes: Record<ThemeMode, Theme> = {
  light: lightTheme,
  dark: darkTheme,
  anya: anyaTheme,
  chihiro: chihiroTheme,
  ntd: ntdTheme,
  sibly: siblyTheme,
  totoro: totoroTheme,
  xi: xiTheme,
};
```

Add the six new exports to [`themes/index.ts`](lib/packages/ui/src/theme/themes/index.ts).

### Provider changes

[`AegisThemeProvider.tsx`](lib/packages/ui/src/theme/AegisThemeProvider.tsx)
only needs two changes:

1. The `isThemeMode` type guard widens to the new union (each new ID
   spelled out).
2. The `localStorage` key (`aegis:theme:mode`) and the rest of the
   provider stay as-is — backward compatible with existing stored values.

`useThemeMode` ([`useThemeMode.ts`](lib/packages/ui/src/theme/useThemeMode.ts))
is unaffected — it returns `{ mode, setMode }` and `mode` widens via
the shared `ThemeMode` type.

### Settings page dropdown

Replace the `<Switch>` toggle in
[`SettingsPage.tsx`](apps/desktop/aegis-desktop/src/features/settings/pages/SettingsPage.tsx)
with a `<Select<ThemeMode>>` matching the style of the existing language
picker (same `<FormControl size="small" sx={{ minWidth: 160 }}>`).

The dropdown renders 8 `<MenuItem>` entries; each label resolves through
`useI18n().t('settings.theme.<id>')`:

```tsx
const THEME_OPTIONS: ThemeMode[] = [
  'light', 'dark', 'anya', 'chihiro', 'ntd', 'sibly', 'totoro', 'xi',
];

<Select<ThemeMode>
  labelId="theme-label"
  value={mode}
  label={t('settings.theme.label')}
  onChange={(e) => setMode(e.target.value as ThemeMode)}
>
  {THEME_OPTIONS.map((id) => (
    <MenuItem key={id} value={id}>
      {t(`settings.theme.${id}`)}
    </MenuItem>
  ))}
</Select>
```

`handleThemeChange` from the current Switch implementation is removed.
The `themeLabel` derivation (used by `Switch`) is removed.

### i18n keys

Existing keys kept:

```ts
'settings.theme.label': 'Theme: {mode}',  // unchanged
'settings.theme.dark':  'Dark',           // unchanged
'settings.theme.light': 'Light',          // unchanged
```

New keys added to both [`en.ts`](lib/packages/ui/src/i18n/locales/en.ts)
and [`zhCN.ts`](lib/packages/ui/src/i18n/locales/zhCN.ts):

```ts
'settings.theme.anya':    'Anya',
'settings.theme.chihiro': 'Chihiro',
'settings.theme.ntd':     'NTD',
'settings.theme.sibly':   'Sibly',
'settings.theme.totoro':  'Totoro',
'settings.theme.xi':      'XI',
```

Chinese translations for the non-trivial names:
- `'千寻'` for Chihiro
- `'龙猫'` for Totoro
- The acronyms `NTD`, `Sibly`, `XI` are kept as-is in Chinese
  (consistent with how `Root`, `Admin` are handled in existing keys).

### Cross-window broadcast

The pipeline is already in place; only the type of the payload widens.

**Outbound** ([`SettingsSyncBridge.tsx`](apps/desktop/aegis-desktop/src/features/app/components/SettingsSyncBridge.tsx)):

`PersistentThemeProvider` already calls `persistSettings({ theme })` then
`emit('aegis:settings-changed', { theme })` from inside `handleChange`.
The `mode` argument is already typed as `ThemeMode`, so widening the
union automatically flows through.

**Inbound** ([`useListenForSettingsChanges`](apps/desktop/aegis-desktop/src/features/settings/data/persist.ts)):

The listener payload type is `{ theme?: ThemeMode; locale?: Locale }`.
When `payload.theme` arrives, it calls `setMode(payload.theme)` —
type-safe after the union widens.

**On-disk hydration** ([`useHydrateSettingsFromStore`](apps/desktop/aegis-desktop/src/features/settings/data/persist.ts)):

Reads `theme` once per mount from `settings.bin` before first paint.
An unknown stored string falls through to the provider's existing
fallback (`'light'`).

**Race conditions** are already handled: the local `setMode` from
the user action runs before `onModeChange` fires (provider's effect
order), so by the time `emit` fires the local window has already
re-rendered. Other windows receive the event and re-render. No new
synchronization is required.

### Error handling

| Failure | Behavior |
| --- | --- |
| Invalid stored theme string in `localStorage` | Falls back to `'light'` via the widened type guard |
| Invalid stored theme string in `settings.bin` | Falls back to provider's default (`'light'`) |
| `localStorage.setItem` throws (private mode, quota) | Swallowed (existing try/catch) |
| `store.save()` throws | Swallowed by `persistSettings`'s caller; broadcast still emitted |
| `emit` rejects | Currently no error handling — leaving as-is to match existing pattern; the local window already updated |

### Testing

#### `lib/packages/ui/src/theme/registry.test.ts`

Add one test per new theme ID asserting `getTheme(<id>)` returns the
expected import (identity check) and `palette.mode` matches the theme's
own intent (read each theme file rather than hardcoding):

- `anya` → `palette.mode === 'light'`
- `chihiro` → `palette.mode === 'light'`
- `totoro` → `palette.mode === 'light'` (totoro's palette omits `mode`, but MUI's `createTheme` defaults missing `mode` to `'light'`)
- `ntd` → `palette.mode === 'dark'`
- `sibly` → `palette.mode === 'dark'`
- `xi` → `palette.mode === 'dark'`

If the identity assertion fails, the test surfaces a clear regression.
The mode assertions document which themes belong to which family.

#### `lib/packages/ui/src/theme/AegisThemeProvider.test.tsx`

Extend coverage:

- Each new ID round-trips through `setMode` → `localStorage` → re-mount.
- Invalid stored value `'purple'` still falls back to `'light'`.
- `setMode` is stable across renders (unchanged).

#### `apps/desktop/aegis-desktop/src/test/features/settings/`

**`settings-persist.test.tsx`**:

- Add a test that an arbitrary new ID (`'totoro'`) round-trips through
  `persistSettings` → `useListenForSettingsChanges` → `useThemeMode`.

**`SettingsPage.test.tsx`** (new, optional but recommended):

- Renders 8 `<MenuItem>` entries with the expected labels (for the
  default `'en'` locale).
- Selecting an option calls `setMode` with the chosen ID.

If adding a `SettingsPage.test.tsx` is out of scope for this change,
omit it — the dropdown behavior is a thin wrapper around `setMode`
which is already heavily tested.

## Files touched

| Path | Change |
| --- | --- |
| `lib/packages/ui/src/theme/types.ts` | Widen `ThemeMode` union |
| `lib/packages/ui/src/theme/registry.ts` | Import + map the 6 new themes |
| `lib/packages/ui/src/theme/themes/index.ts` | Re-export the 6 new theme constants |
| `lib/packages/ui/src/theme/AegisThemeProvider.tsx` | Widen `isThemeMode` type guard |
| `lib/packages/ui/src/i18n/locales/en.ts` | Add 6 new translation keys |
| `lib/packages/ui/src/i18n/locales/zhCN.ts` | Add 6 new translation keys |
| `apps/desktop/aegis-desktop/src/features/settings/pages/SettingsPage.tsx` | Replace `<Switch>` with `<Select<ThemeMode>>` |
| `lib/packages/ui/src/theme/registry.test.ts` | Add 6 tests |
| `lib/packages/ui/src/theme/AegisThemeProvider.test.tsx` | Extend test coverage |
| `apps/desktop/aegis-desktop/src/test/features/settings/settings-persist.test.tsx` | Add round-trip test |

## Risks and mitigations

- **Stored `theme: "dark"` value**: unaffected — `'dark'` is still a
  valid `ThemeMode`. No migration needed.
- **`localStorage` value conflicts with `settings.bin`**: the provider
  reads `localStorage` synchronously on mount; `useHydrateSettingsFromStore`
  then overrides from `settings.bin` after mount. The store always
  wins on subsequent loads. Behavior is unchanged from today.
- **i18n key naming collision**: `settings.theme.<id>` keys are namespaced
  under `settings.theme.` — no existing key uses these suffixes except
  `.light` and `.dark`, which are preserved.
- **Workspace windows lag**: every window independently listens for the
  event, so the visual update is bound by the Tauri event round-trip
  (sub-millisecond). No action needed.