# SDTMIG Version Dropdown — Project Configuration General Section

## Goal

Add a third control to the existing `ConfigurationGeneralSection`: an
**SDTMIG Version** `<Select>` whose items come from
`api.listSdtmVersions()`. The selection is saved alongside `language` and
`tags` as a third field on the project's `configurations` payload, so a
leader can pin a project to one of the SDTM Implementation Guide
versions that the desktop already knows about.

This is a frontend task: the server side already carries `sdtmig`
through `apis::project::ProjectConfigurationData`,
`dto::ProjectConfigurationDataRequest`, and the read projection
(`ProjectConfigurationViewResponse`) — the recent
`feat(project_config-sdtmig-version)` merge on `main` landed the full
backend change. The desktop's TS mirror just hasn't caught up.

## Architecture

Section composition stays the same — `ConfigurationGeneralSection`
remains the one section file for the General section. We extend its
`touched` flag set with `sdtmTouchedRef`, extend its `Select` set with a
second `<FormControl>`, and extend the save payload to include the
new field. Nothing else in the page (`ProjectConfigurationPage`,
`ConfigurationSidebar`, the Members / Filepath sections) needs to
know.

```
┌─ ConfigurationGeneralSection ────────────────────────────────┐
│                                                             │
│  Preferred language     [ English ▾ ]                       │
│                                                             │
│  SDTMIG Version          [ SDTMIG v3.4 ▾ ]  ← new           │
│                                                             │
│  Tags           (TagEditor)                              │
│                                                             │
│                                       [ Save ]              │
└─────────────────────────────────────────────────────────────┘
```

Order is **Language → SDTMIG Version → Tags** (the user-chosen order).
Two adjacent single-selects sit above the multi-value editor.

### Existing hooks we lean on

| Hook | Source | Why |
| --- | --- | --- |
| `useListSdtmVersions()` | `features/domain-model/data/list.ts` | Already wired with `queryKey: queryKeys.domainModel.sdtmVersions()`. Returns `SdtmVersionView[]`. Reused as-is. |
| `useUpdateProject()` | `features/project-list/data/projects.ts` | Already invalidates `project.all` + `project.byCode(code)` on success. Mutation body extends in-place. |

No new files needed.

## Files

### Modified

- `apps/desktop/aegis-desktop/src/shared/api/types.ts` — extend
  `ProjectConfiguration` with `sdtmig`. Add a small named type for the
  pair so the field shape has a single source of truth.
- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx` —
  add `sdtmig` local state, `sdtmTouchedRef`, the new `<FormControl>`,
  the load of `useListSdtmVersions`, and include `sdtmig` in the save
  body.
- `lib/packages/ui/src/i18n/locales/en.ts` — add
  `project.configuration.general.sdtmig` and
  `project.configuration.general.sdtmig.none`.
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — same two keys in
  Simplified Chinese.
- `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx` —
  add focused cases for the new control.

## Wire shape (delta)

The server already publishes the new wire shape; this is just the
desktop TS mirror catching up.

```ts
// shared/api/types.ts (delta)
export interface ProjectConfigurationSdtmig {
  versionId: number;
  versionName: string;
}

export interface ProjectConfiguration {
  language: ProjectLanguage | null;
  tags: Tag[];
  sdtmig?: ProjectConfigurationSdtmig | null;
}
```

`versionId` / `versionName` are camelCase to match the server's
`#[serde(rename_all = "camelCase")]` on `dto::ModelVersionRequest` /
`ModelVersionResponse` (`apps/server/aegis-server/src/transport/http/dto.rs`,
lines 308–340). `sdtmig` is optional + nullable so omitting it from a
fixture stays valid (the existing `makeProject` test helper at
`apps/desktop/aegis-desktop/src/test/helpers/project-fixture.ts`
needs no change).

## State + save semantics

Mirrors the `languageTouchedRef` / `tagsTouchedRef` pattern documented
in `ConfigurationGeneralSection`:

```tsx
const [sdtmig, setSdtmig] = useState<ProjectConfigurationSdtmig | null>(
  initial.sdtmig ?? null,
);
const sdtmTouchedRef = useRef(false);
```

The reseed-on-`initial`-change `useEffect` extends to also reseed
`sdtmig` and reset `sdtmTouchedRef.current = false`. The `dirty` flag
becomes `languageTouchedRef.current || tagsTouchedRef.current ||
sdtmTouchedRef.current`, and the save body becomes:

```tsx
async function onSave() {
  const configurations: ProjectConfiguration = { language, tags, sdtmig };
  await update.mutateAsync({
    code: projectCode,
    body: { configurations },
  });
}
```

The server's `ProjectConfigurationDataRequest` carries
`#[serde(skip_serializing_if = "Option::is_none")]` on
`sdtmig: Option<ModelVersionRequest>`, so an unset selection
serialises as the field being absent on the request — the apis layer
sees `None` and the existing "no change" branch in `handlers.rs`'s
`configuration_data` carries through. No backend change is needed.

## UI

The new control sits between the language `<FormControl>` and the
`<TagEditor>`'s wrapping `<Box>`, matching the existing
`size="small"` + `<InputLabel>` + `<Select>` shape.

```tsx
const versions = useListSdtmVersions();

<FormControl size="small" disabled={readonly}>
  <InputLabel id="config-sdtmig-label">
    {t("project.configuration.general.sdtmig")}
  </InputLabel>
  <Select<string>
    labelId="config-sdtmig-label"
    label={t("project.configuration.general.sdtmig")}
    value={sdtmig ? String(sdtmig.versionId) : ""}
    onChange={(e) => {
      const v = e.target.value;
      if (v === "") {
        setSdtmig(null);
      } else {
        const id = Number(v);
        const found = versions.data?.find((x) => x.id === id);
        setSdtmig(
          found
            ? { versionId: found.id, versionName: found.name }
            : { versionId: id, versionName: "" },
        );
      }
      sdtmTouchedRef.current = true;
    }}
    inputProps={{ "data-testid": "config-sdtmig-input" }}
  >
    <MenuItem value="">
      {t("project.configuration.general.sdtmig.none")}
    </MenuItem>
    {(versions.data ?? []).map((v) => (
      <MenuItem key={v.id} value={String(v.id)}>
        {v.name}
      </MenuItem>
    ))}
  </Select>
</FormControl>
```

The "find-or-blank-name" fallback handles the leading render before
the query has resolved: a leader who lands on the page with a saved
sdtmig pointer will see the control value set immediately even though
the version-options query is still pending (the value comes from
`initial`, not from the query). The local state is the source of
truth for the selection, the query is just the source of the option
list.

The MenuItems render only when `versions.data` resolves; the empty
list during loading is fine (the user sees only the "(unspecified)"
item until the list arrives).

## i18n

Added to `lib/packages/ui/src/i18n/locales/en.ts`:

```
'project.configuration.general.sdtmig': 'SDTMIG Version',
'project.configuration.general.sdtmig.none': '(unspecified)',
```

Added to `lib/packages/ui/src/i18n/locales/zhCN.ts`:

```
'project.configuration.general.sdtmig': 'SDTM-IG 版本',
'project.configuration.general.sdtmig.none': '（未指定）',
```

These follow the existing `project.configuration.general.*` key
hierarchy and the `(unspecified)` / `（未指定）` convention used by
`project.configuration.general.language.none`.

## Tests

Add to
`apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx`.

| Case | Expectation |
| --- | --- |
| Renders the SDTMIG Select with the current selection when `initial.sdtmig` is present | The Select's value label shows the saved `versionName`. |
| Renders an "(unspecified)" item as the first option | `screen.findByText("(unspecified)")` is present. |
| Picking a version enables Save | After change, `config-general-save` is no longer disabled. |
| Picking "(unspecified)" → Save → `configurations.sdtmig === null` | `invoke` last call's `body.configurations.sdtmig` is `null`. |
| Save with sdtmig change sends `{ language, tags, sdtmig }` | `expect.objectContaining({ configurations: expect.objectContaining({ language, tags, sdtmig }) })`. |
| Non-leader: SDTMIG Select is disabled | The Select's form control has `Mui-disabled`. |

Existing tests:

- "clicking Save fires update_project with configurations { language, tags }"
  stays valid — `expect.objectContaining` is additive; I'll extend it
  to also assert `sdtmig` for clarity.
- "language Select is disabled for non-leaders" gets the sibling
  SDTMIG assertion listed.

The test's `mockCommands` blocks gain a `list_sdtm_versions` handler
returning at least one row so the MenuItems render. The
`projectFixture` keeps its current `configurations: { language, tags }`
shape (`sdtmig` stays absent → no option is preselected) for the
bulk of the suite; one focused test overrides the fixture to seed a
preselected sdtmig value.

## Verification gate

```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter @aegis/ui typecheck
```

The first one covers the new TS interface + the new component. The
third covers the i18n key typing. No Rust changes — `cargo check` is
not in the gate for this PR.

## Out of scope

- Showing the description / structure of the picked SDTMIG version
  (the `versions[i]?.descriptions` data — not exposed by the dropdown).
- A way to *add* a new SDTMIG version from the configuration surface.
  Versions already have their own page under `features/domain-model`.
- A "search" affordance for long SDTMIG version lists. The list is
  small enough that a flat `<Select>` matches the language control's
  UX; if it ever grows past ~20 items, swap to `<Autocomplete>`.
- Migrating the existing `ProjectDrawer` to also expose
  `sdtmig`. Out of scope for this PR.

## Commits

3 commits in dependency order:

1. **wire(types)** — TS mirror (`types.ts`).
2. **i18n** — add the two keys in `en.ts` + `zhCN.ts`.
3. **feature(general)** — `ConfigurationGeneralSection` + new tests.