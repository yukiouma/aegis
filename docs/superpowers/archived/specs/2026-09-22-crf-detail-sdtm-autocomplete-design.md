# CRF Detail — SDTM Autocomplete (Domain Annotation & @-variable Mention)

Date: 2026-09-22
Status: Approved (brainstorming complete)
Feature: `crf` (aegis-desktop)

## Goal

Add two autocomplete affordances to the CRF detail page so users can annotate
forms against a real SDTM version pinned by project configuration:

1. **Domain annotation dialog** — auto-complete the **domain name** with
   SDTM domains from the project's resolved version, and auto-fill the
   **domain description** from the matching SDTM record in the project's
   configured language.
2. **Annotation dialog** — when the user types `@` inside the **content**
   field, open a floating list of variables for the currently-selected
   domain annotation's matching SDTM domain; selecting a variable inserts
   the variable's name at the caret.

The SDTM lookup is driven by `project.configurations.sdtmig` (the existing
SDTMIG-version pointer set by the leader in
`ConfigurationGeneralSection` — see
`2026-09-22-aegis-desktop-project-config-sdtmig-dropdown-design.md`). When
`configurations.sdtmig` is absent, the page falls back to the highest-id
SDTM version. When the configured `versionId` is stale (the version was
deleted server-side) but the configured `versionName` still exists, the
page silently self-heals the project configuration with the correct
`versionId`. When neither matches, the page falls back to the latest
version.

## Non-goals

- Backend changes. No DB migrations, no DTO changes, no new endpoints.
- Persisting an SDTM-domain binding on `DomainAnnotation` (no
  `sdtmDomainId` column). The match between a domain annotation and an
  SDTM domain is a **name match** (case-insensitive) at runtime.
- Changing the existing `freeSolo` / free-form path: users can still type
  a free-form domain code that does not match any SDTM domain, and the
  annotation is saved as-is.
- Adding a separate "SDTM Domain" picker to the annotation dialog. The
  dialog's `@` dropdown is gated on the existing domain-annotation
  picker; users who pick an unbound domain annotation get no `@` options
  (no error, no flash).
- Parsing or highlighting pre-existing `@xxx` substrings when editing an
  existing annotation. The user can still trigger a new mention by typing
  `@` while focused.
- zh-CN translations of any of the new i18n keys. English tree is added;
  zh-CN gets the same keys with English placeholders (mirrors the existing
  pattern in `2026-09-22-aegis-desktop-project-config-sdtmig-dropdown-design.md`).

## Architecture

```
CrfDetailPage
 ├─ useProject(projectCode)         // already exists; reads configurations.{language, sdtmig}
 ├─ useListSdtmVersions()           // already exists
 ├─ useSdtmContext(projectCode)     // NEW — resolves versionId + language + domains
 │     ├─ resolves versionId (3-state: explicit / name-recover / latest)
 │     ├─ resolves language (project.language ?? 'en')
 │     └─ runs useListSdtmDomains(resolvedVersionId)
 ├─ <DomainAnnotationDialog ... />  // gets { sdtmDomains, sdtmLanguage }
 └─ <AnnotationDialog     ... />    // gets { sdtmDomains }
```

- **New file**: `apps/desktop/aegis-desktop/src/features/crf/data/sdtm.ts`
  (the `useSdtmContext` hook).
- **Edited**: `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`
  (consume the hook, pass props into both dialogs).
- **Edited**: `apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.tsx`
  (replace the Name TextField with an Autocomplete; add a description
  auto-fill on selection).
- **Edited**: `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx`
  (replace the Content TextField with a controlled input that listens to
  caret position; add a Popover-based variable dropdown driven by
  `@`-mention detection).
- **New tests** (one each for the three major behaviors; details in
  the Testing section).
- **Edited**: existing `crf-detail-page.test.tsx` to mount the page with
  the SDTM context fixture.

## `useSdtmContext` — version + language resolution

The hook owns three derived values and lives at
`apps/desktop/aegis-desktop/src/features/crf/data/sdtm.ts`:

```ts
export interface SdtmContext {
  versionId: number | null;       // null while resolving or no versions exist
  language: string;               // "en" | "zh-CN" | ...
  domains: SdtmDomainView[];      // [] while loading or version unresolved
  loading: boolean;               // versions or domains still fetching
  error: ApiError | null;         // first error from versions or domains query
}

export function useSdtmContext(projectCode: string): SdtmContext
```

### `versionId` resolution

A single `useMemo` over `(versions, projectQuery.data)` runs the
three-state fallback:

1. **Unset** — `projectQuery.data?.configurations.sdtmig?.versionId` is
   `undefined` / `null`. Pick `Math.max(...versions.map(v => v.id))`. If
   `versions` is empty, return `null`.
2. **Valid** — the configured `versionId` exists in `versions`. Use it
   directly.
3. **Stale** — the configured `versionId` is **not** in `versions`:
   - Look up `versions.find(v => v.name.toLowerCase() === sdtmig.versionName.toLowerCase())`.
   - If found → **fire a one-shot `useUpdateProject`** so the project
     config self-heals. The mutation body preserves the existing
     `language` and `tags` fields of `configurations` and only swaps the
     `sdtmig` pointer:

     ```ts
     updateProject.mutate({
       code: projectCode,
       body: {
         configurations: {
           language: projectQuery.data.configurations.language,
           tags: projectQuery.data.configurations.tags,
           sdtmig: { versionId: found.id, versionName: found.name },
         },
       },
     });
     ```

     The mutation runs at most once per stale versionId per session
     (gated by a `useRef<Set<number>>` of already-corrected ids). Use
     `found.id` for the current query.
   - If not found → fall back to **situation 1**.

The recovery write only fires on stale resolution and never loops: the
post-correction state has a `versionId` that exists in `versions`, so
future renders go down branch 2 and skip the effect.

### `language` resolution

`projectQuery.data?.configurations.language ?? "en"`. Pure derivation; no
fetch.

### `domains`

`useListSdtmDomains(versionId)`, enabled only when `versionId != null`.
The hook reuses the existing query key
`queryKeys.domainModel.sdtmDomains(versionId)` so cache hits are free.

### `loading` / `error`

`loading = versionsQuery.isLoading || domainsQuery.isLoading`.
`error = versionsQuery.error ?? domainsQuery.error ?? null`.

## `DomainAnnotationDialog` — name autocomplete + description auto-fill

The dialog keeps its current shape but the **Name** field becomes an MUI
`<Autocomplete>`:

```tsx
<Autocomplete<string, false, true, false>
  freeSolo
  options={sdtmDomains.map(d => d.name)}
  filterOptions={(opts, state) =>
    opts.filter(o => o.toUpperCase().startsWith(state.inputValue.toUpperCase()))
  }
  inputValue={body.name}
  onInputChange={(_, v, reason) => {
    if (reason === "input") setBody(b => ({ ...b, name: v.toUpperCase() }));
    else setBody(b => ({ ...b, name: v }));
  }}
  onChange={(_, v) => {
    const next = (typeof v === "string" ? v : v ?? "").toUpperCase();
    setBody(b => ({
      ...b,
      name: next,
      description: next
        ? sdtmDomains.find(d => d.name.toUpperCase() === next)
            ?.descriptions.find(d => d.lang === sdtmLanguage)
            ?.details.description ?? ""
        : b.description,
    }));
  }}
  renderInput={(params) => (
    <TextField {...params} size="small" label={t("crf.domainDialog.field.name")} />
  )}
/>
```

### Behavior notes

- `freeSolo` preserves the existing "free-form name" path — users can
  still type a code that does not match any SDTM domain, and the
  annotation saves as-is.
- Filter is **startsWith** against the uppercased current input —
  matches "auto upcase what they enter, filter the domains with the
  current value of the domain name text field".
- All entered names are uppercased before going into state
  (`onInputChange` for typing, `onChange` for selection).
- On selection, the description auto-fills from the matching SDTM
  domain's `descriptions[lang === sdtmLanguage].details.description`.
  **If no description exists for that language, the description is left
  empty** (per the brainstorming Q3 decision — matches the existing
  `SdtmDomainList` empty-cell behavior; no em-dash, no warning).
- The Description field stays an editable `<TextField>` — auto-fill is a
  default, not a lock. The user can type over it.
- In edit mode the Autocomplete is still enabled; the user can re-pick a
  different SDTM domain and the description auto-refills.

### New props on `DomainAnnotationDialog`

```ts
sdtmDomains: SdtmDomainView[];     // [] disables the autocomplete's option list
sdtmLanguage: string;              // "en" by default; used for description lookup
```

`CrfDetailPage` reads both from `useSdtmContext(projectCode)`. When
`sdtmDomains.length === 0` the Autocomplete still renders (with no
options) so the user can still type free-form names.

## `AnnotationDialog` — `@`-variable mention

The **Content** field becomes a controlled `<TextField>` that listens to
caret position. A floating `<Popover>` anchored to the field shows
variables when an active `@fragment` is in progress. **Insertion is
always the variable's `name`** — language-independent.

### State

```ts
const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);
const [mentionRange, setMentionRange] =
  useState<{ start: number; end: number } | null>(null);
const inputRef = useRef<HTMLInputElement | null>(null);

// Find the SDTM domain for the currently-selected domain annotation.
const selectedDomainName = useMemo(() => {
  const da = availableDomainAnnotations.find(
    d => d.id === body.domainAnnotationId,
  );
  return da?.name?.toUpperCase() ?? null;
}, [availableDomainAnnotations, body.domainAnnotationId]);
const selectedDomain = useMemo(
  () => sdtmDomains.find(d => d.name.toUpperCase() === selectedDomainName) ?? null,
  [sdtmDomains, selectedDomainName],
);
// Already cached: keyed by domainId in queryKeys.domainModel.sdtmVariables.
const variablesQuery = useListSdtmVariables(
  open && selectedDomain ? selectedDomain.id : null,
);

const fragment = mentionRange
  ? body.content.slice(mentionRange.start, mentionRange.end)
  : "";
const filteredVariables = useMemo(() => {
  const all = variablesQuery.data ?? [];
  const q = fragment.toUpperCase();
  if (!q) return all;
  return all.filter(v => v.name.toUpperCase().startsWith(q));
}, [variablesQuery.data, fragment]);
```

### `onChange` handler — `@`-trigger detection

```ts
function handleContentChange(e: ChangeEvent<HTMLInputElement>) {
  const value = e.target.value;
  const caret = e.target.selectionStart ?? value.length;
  setBody(b => ({ ...b, content: value }));

  // Find the @ that opens the current mention fragment (if any).
  // Walk backwards from caret looking for whitespace; the last @
  // before that (and before the caret) opens the mention.
  const before = value.slice(0, caret);
  const lastWs = Math.max(
    before.lastIndexOf(" "),
    before.lastIndexOf("\n"),
    before.lastIndexOf("\t"),
  );
  const head = before.slice(lastWs + 1);
  const atIdx = head.lastIndexOf("@");
  if (atIdx >= 0) {
    const start = lastWs + 1 + atIdx;
    // Reject if the @ is mid-word (e.g. user typed `foo@bar`).
    if (atIdx === 0 || /\s/.test(head[atIdx - 1] ?? "")) {
      setMentionRange({ start, end: caret });
      setAnchorEl(e.currentTarget);
      return;
    }
  }
  setMentionRange(null);
  setAnchorEl(null);
}
```

### The Popover — name only

```tsx
<Popover
  open={Boolean(anchorEl) && mentionRange !== null}
  anchorEl={anchorEl}
  anchorOrigin={{ vertical: "bottom", horizontal: "left" }}
  slotProps={{ paper: { sx: { minWidth: 240, maxHeight: 240 } } }}
>
  <MenuList>
    {filteredVariables.length === 0 ? (
      <MenuItem disabled>{t("crf.annotationDialog.variable.noMatch")}</MenuItem>
    ) : (
      filteredVariables.map(v => (
        <MenuItem
          key={v.id}
          onClick={() => insertVariable(v.name)}
          data-testid={`crf-variable-${v.id}`}
        >
          {/* Name only — inserted text is always v.name, language-independent. */}
          {v.name}
        </MenuItem>
      ))
    )}
  </MenuList>
</Popover>
```

### `insertVariable`

```ts
function insertVariable(name: string) {
  if (!mentionRange) return;
  const before = body.content.slice(0, mentionRange.start);
  const after = body.content.slice(mentionRange.end);
  const inserted = name;                       // replaces "@fragment" with the variable's
                                              // name (always, language-independent)
  const next = before + inserted + after;
  setBody(b => ({ ...b, content: next }));
  setMentionRange(null);
  setAnchorEl(null);
  const caret = (before + inserted).length;
  queueMicrotask(() => {
    inputRef.current?.setSelectionRange(caret, caret);
  });
}
```

### Behavior notes

- The `@` dropdown **only opens** when the currently-selected domain
  annotation's `name` matches an SDTM domain in the project's resolved
  version (case-insensitive). No match → no dropdown, no flash, no error.
- Each option is rendered as the **variable's `name`** (no label, no
  language-conditional row content). The project's `sdtmLanguage` does
  not affect the `@` experience at all — it only affects the
  domain-description auto-fill in the other dialog.
- Empty `@` (just typed, no fragment yet) shows the full variable list,
  in the order the server returns them (no client-side sort).
- Whitespace, another `@`, or any non-letter character typed into the
  fragment closes the mention. (`startsWith` filters on the uppercased
  fragment; a space typed inside the fragment breaks the
  `lastWs + 1` math and the next render sets `mentionRange` to `null`.)
- Variable list is cached by `domainId` via the existing
  `queryKeys.domainModel.sdtmVariables`; no per-keystroke fetching.
- Backspace into the `@` collapses the mention — the dropdown closes;
  no special handling needed.
- If the dialog re-seeds (`useEffect` on `open/mode/row`) mid-mention,
  the mention is wiped. Acceptable since reseeding only happens on
  open/close and mode switch.
- Edit mode: pre-existing `@xxx` text in `row.content` is not parsed.
  The user can still trigger a new mention by typing `@`.

### New prop on `AnnotationDialog`

```ts
sdtmDomains: SdtmDomainView[];
```

The page passes the same array used by `DomainAnnotationDialog`. The
dialog uses it solely for the name-match lookup; the language does not
affect the `@` UX.

## Wire shape

No wire-shape changes. The spec reuses existing endpoints:

| Endpoint | Already exposed by |
| --- | --- |
| `list_projects` | `project-list/data/projects.ts` (`useListProjects`, `useProject`) |
| `update_project` | `project-list/data/projects.ts` (`useUpdateProject`) |
| `list_sdtm_versions` | `domain-model/data/list.ts` (`useListSdtmVersions`) |
| `list_sdtm_domains_by_version` | `domain-model/data/list.ts` (`useListSdtmDomains`) |
| `list_sdtm_variables_by_domain` | `domain-model/data/list.ts` (`useListSdtmVariables`) |

## File-by-file change list

### New files

- `apps/desktop/aegis-desktop/src/features/crf/data/sdtm.ts`
- `apps/desktop/aegis-desktop/src/features/crf/data/sdtm.test.tsx`
- `apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.test.tsx`
  *(extend existing)* — add cases for the autocomplete + description
  auto-fill.
- `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.test.tsx`
  *(extend existing)* — add cases for the `@` mention.

### Edited files

- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.tsx`
  — call `useSdtmContext(projectCode)`, pass `sdtmDomains` /
  `sdtmLanguage` into both dialogs.
- `apps/desktop/aegis-desktop/src/features/crf/components/DomainAnnotationDialog.tsx`
  — add `sdtmDomains`, `sdtmLanguage` props; replace the Name TextField
  with the Autocomplete block; wire description auto-fill.
- `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx`
  — add `sdtmDomains` prop; wire the controlled content handler; add
  the Popover-based variable dropdown; add the `inputRef` plumbing.
- `apps/desktop/aegis-desktop/src/features/crf/pages/CrfDetailPage.test.tsx`
  *(extend existing)* — mount the page with the SDTM context fixture so
  the existing dialog-mount tests still pass with the new prop shape.
- `lib/packages/ui/src/i18n/locales/en.ts` — add the two keys below.
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — same two keys, English
  placeholders.

## i18n additions

`lib/packages/ui/src/i18n/locales/en.ts` — append:

```
"crf.annotationDialog.field.content.placeholder": "Type content — use @ to insert a variable"
"crf.annotationDialog.variable.noMatch":          "No variables match"
```

`lib/packages/ui/src/i18n/locales/zhCN.ts` — append with English
placeholders:

```
"crf.annotationDialog.field.content.placeholder": "Type content — use @ to insert a variable"
"crf.annotationDialog.variable.noMatch":          "No variables match"
```

(The placeholder is wired on the Content `<TextField>`; the `noMatch`
key is wired into the Popover as a single `<MenuItem disabled>` row
when `filteredVariables.length === 0` but a fragment is in progress.)

## Testing

### Unit — `useSdtmContext` (`features/crf/data/sdtm.test.tsx`)

| Case | Expectation |
| --- | --- |
| `versionId` unset → uses highest-id version | With versions `[1, 5, 3]` and project with no `sdtmig`, returned `versionId === 5`. |
| `versionId` set + valid → uses it | With versions `[1, 5, 3]` and project `sdtmig.versionId === 3`, returned `versionId === 3`. |
| `versionId` stale + name matches → uses new id, fires `useUpdateProject` once | `updateProject` called with `{ versionId: <foundId>, versionName: <foundName> }`. Re-rendering does not refire. |
| `versionId` stale + name missing → falls back to highest-id | `updateProject` not called; `versionId` is `Math.max(...)`. |
| `language` from project.language | Project `language: "zh-CN"` → `language === "zh-CN"`. |
| `language` falls back to `"en"` | Project `language: null` → `language === "en"`. |
| `loading` while versions still fetching | `loading === true` while `versionsQuery.isLoading`. |
| `error` surfaces from either query | First non-null error from `versions` or `domains`. |

### Component — `DomainAnnotationDialog` (extend existing)

| Case | Expectation |
| --- | --- |
| Typing in the Name field auto-uppercases | `fireEvent.change(input, { target: { value: "ae" } })` → Autocomplete `inputValue === "AE"`. |
| Options filtered by uppercased startsWith | Three domains `[AE, AESI, AG]` + input `"a"` → only `AE` and `AESI` shown (`AG` doesn't startsWith `"A"`). |
| Selecting a domain auto-fills the description (language match) | Pick `AE` with project language `"en"` and `AE` having an English description → description field equals that description. |
| Selecting a domain with no description for that language → description empty | Pick `AE` with project language `"zh-CN"` and `AE` only having an English description → description field is empty. |
| Free-form typing (no pick) leaves description unchanged | Type `"ZZ"` (no match) → description field is unchanged. |
| In edit mode, picking a different domain re-fills the description | Initial body has `description: "old"`; pick `AE` → description becomes the SDTM one. |

### Component — `AnnotationDialog` (extend existing)

| Case | Expectation |
| --- | --- |
| Typing `@` opens the Popover with full variable list | After `@`, `findByTestId("crf-variable-1")` etc. are present. |
| Typing `@a` filters to variables starting with `A` | Only variables whose `name.toUpperCase().startsWith("A")` are rendered. |
| Clicking a variable inserts the name and replaces the `@fragment` | `@age` then click `AGE` → content is `"AGE"`. |
| Typing `@` mid-word does **not** open | Content `"foo@bar"` → no Popover. |
| Selecting a domain annotation that does not match any SDTM domain → no Popover | With no matching domain in `sdtmDomains`, typing `@` shows nothing. |
| Edit mode: pre-existing `@xxx` text is preserved; new `@` still opens | Re-seed with content `"old @zzz"`; type more → previous text untouched; new `@a` opens filtered list. |

### Page — `CrfDetailPage` (extend existing)

| Case | Expectation |
| --- | --- |
| Mounts with project `language: null` | AnnotationDialog renders with the `en` default (smoke — full detail in `useSdtmContext` tests). |
| Stale `versionId` resolution mounts without throwing | Page renders; the recovery mutation is fired at most once. |

## Verification gate

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter @aegis/ui typecheck
```

No Rust changes — `cargo check` is not in the gate for this PR.

## Open decisions resolved during brainstorming

1. **How does an annotation know which SDTM domain it belongs to?**
   *Match by domain name* — when a domain annotation is picked, find the
   SDTM domain whose `name` equals the annotation's `name` (case-insensitive).
   If no match, the `@` dropdown shows nothing. No backend change.
2. **When `sdtmig` is unset, which version counts as "latest"?**
   *Highest `id`* — monotonic with insert order; no sorting change needed.
3. **When the SDTM domain has no description for the project's language?**
   *Leave the description empty* — matches the existing `SdtmDomainList`
   empty-cell behavior.
4. **Dropdown row content for `@`-variable list?**
   *Variable name only* — the option IS the variable name; the project's
   language does not affect what's inserted.