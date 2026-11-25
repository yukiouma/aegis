# Auto-seed Variable Descriptions by Available Language

## Problem

When creating a new SDTM variable via `VariableEditDrawer`, the "descriptions"
field (the array of `{ lang, details: { label } }` rows) starts empty. Users
have to manually click "Add description" and pick a language for every row.
That's tedious when the domain already has a known set of supported languages
surfaced by the page-level language dropdown.

## Goal

When the drawer opens in **create mode**, pre-seed one description row per
language available in the domain, with the `lang` field pre-filled and the
`label` left blank for the user to fill.

## Non-goals

- Auto-seed in **edit mode**. Edit mode continues to load `row.descriptions`
  verbatim.
- Make `lang` a `Select` constrained to the available languages. It stays a
  free `TextField`, pre-filled but editable.
- Add a new query for languages inside the drawer. The drawer receives the
  list via a prop.
- Change the submit-time filter. Rows with empty `lang` are still filtered out.

## Architecture

The parent page `SdtmDomainDetail` already computes `availableLanguages` from
the union of the domain's `descriptions[*].lang` and every variable's
`descriptions[*].lang`, sorted. Thread that value into `VariableEditDrawer` via
a new prop. The drawer's existing create-mode `useEffect` branch seeds the
descriptions state from it.

```
SdtmDomainDetail ──(availableLanguages)──▶  VariableEditDrawer
```

No new API call, no duplicated aggregation logic.

## Component Changes

### `apps/desktop/aegis-desktop/src/features/domain-model/components/VariableEditDrawer.tsx`

1. Add an optional prop to the props interface:
   ```ts
   availableLanguages?: string[];
   ```
2. Destructure it in the function signature.
3. In the existing `useEffect` create-mode branch, replace the empty seed with
   a per-language seed:
   ```ts
   } else if (mode === "create") {
     setName("");
     setVariableControlled("");
     setVariableType("Character");
     setVariableCore("Req");
     setVariableRole(null);
     const langs = availableLanguages ?? [];
     setDescriptions(
       langs.map((lang) => ({ lang, details: { label: "" } })),
     );
   }
   ```
4. Nothing else changes. The existing per-row `TextField` UI (lang + label),
   the "Add description" button, and the per-row delete button all continue to
   work, so users can adjust, edit, or remove the seeded rows freely.

### `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainDetail.tsx`

Pass the existing `availableLanguages` value to the drawer at the call site
(around line 256):
```tsx
<VariableEditDrawer
  ...
  availableLanguages={availableLanguages}
/>
```

## Edge Cases

| Case | Behavior |
|---|---|
| `availableLanguages` is empty (no domain descriptions, no variable descriptions yet) | `descriptions` seeds as `[]` — current behavior preserved. User clicks "Add description" to add rows manually. |
| `availableLanguages` has 1 entry | Seeds 1 row with that lang. |
| User deletes an auto-added row, then reopens the drawer in create mode | Re-seeds with the full language list (existing reset semantics — create-mode `useEffect` always overwrites on open). |
| User edits an auto-added row's `lang` to something not in the dropdown | Persists until next open (create mode re-seeds; edit mode preserves). |
| `availableLanguages` changes while drawer is open | No effect — the create-mode reset only fires on `[open, mode, row]` transitions. The existing `useEffect` deps stay unchanged. |
| Edit mode | Unchanged. `row.descriptions` loads as-is; `availableLanguages` is ignored. |
| Submit (create) | Unchanged. `descriptions.filter((d) => d.lang.trim() !== "")` still filters out empty-lang rows. An auto-added row whose `lang` is left intact (but whose label is left blank) submits with that lang and empty label — same semantics as today if the user adds a row manually and fills only lang. |

## Testing

### Component tests — `apps/desktop/aegis-desktop/src/test/features/domain-model/variable-edit-drawer.test.tsx`

The file already exists. Extend it with:

1. **Seeds one row per language in create mode.** Render the drawer open in
   create mode with `availableLanguages={["en", "zh-CN"]}`. Expect 2 rows,
   langs `["en", "zh-CN"]`, labels all empty.
2. **Empty languages → no rows.** `availableLanguages={[]}` in create mode
   yields 0 rows (current behavior preserved).
3. **Single language → single row.** `availableLanguages={["en"]}` yields 1 row
   with `lang: "en"`.
4. **Edit mode ignores `availableLanguages`.** `mode="edit"` with a `row`
   carrying 2 descriptions, plus `availableLanguages={["en"]}`, loads the row's
   2 descriptions verbatim; the prop is not applied.
5. **Close + reopen in create mode re-seeds.** Delete a seeded row, close the
   drawer, reopen — the row is back to the full language list (existing reset
   semantics preserved).
6. **Submit includes auto-added rows.** Open in create mode with
   `availableLanguages={["en", "zh-CN"]}`, fill the name, submit. Assert
   `onCreate` was called with
   `descriptions: [{ lang: "en", details: { label: "" } }, { lang: "zh-CN", details: { label: "" } }]`.

### Parent test — `apps/desktop/aegis-desktop/src/test/features/domain-model/sdtm-domain-detail.test.tsx`

No new test required. The existing "opens the variable create drawer with
max+1 sequence" test (line 117) continues to pass because it doesn't assert on
`descriptions` and the existing variable mock data has descriptions that will
naturally populate `availableLanguages`.

### Verification

- `pnpm test -- variable-edit-drawer sdtm-domain-detail` — all green.
- `pnpm typecheck` — clean.

## Files Touched

- `apps/desktop/aegis-desktop/src/features/domain-model/components/VariableEditDrawer.tsx` (modified)
- `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainDetail.tsx` (modified — one prop added at call site)
- `apps/desktop/aegis-desktop/src/test/features/domain-model/variable-edit-drawer.test.tsx` (modified — extend with the 6 cases above)