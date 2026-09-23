# CRF AnnotationDialog — SUPP quick-draft button

Date: 2026-09-23
Status: Approved (brainstorming)

## Goal

Add a one-click "SUPP" button to the existing `AnnotationDialog` so
authors can pre-fill the annotation content with the common SDTM
supp-domain pattern `XXXX in SUPPYY` instead of typing it by hand.

The button is purely a content helper — it edits the dialog's local
`body.content`. No server change. No new mutation. No new endpoint.

## Where the button lives

In `apps/desktop/aegis-desktop/src/features/crf/components/AnnotationDialog.tsx`,
immediately to the right of the existing content `TextField`. The
content field and the button sit inside a horizontal `Stack
direction="row"` so the field keeps flex-grow and the button stays
right-aligned at its natural width.

The button label is the literal string `SUPP` — no i18n key, no
tooltip. (Per author preference on 2026-09-23: SUPP is a domain
acronym, kept as-is in both locales.)

## Pattern (replace semantics)

When the user clicks SUPP, the dialog replaces the entire content
with the drafted string. There is no "preserve + append" mode — the
author chose replace.

| Owner kind | Drafted content |
|---|---|
| `item` (and item code resolves) | `<ITEMCODE> in SUPP<DOMAINCODE>` |
| `form` / `option` / `unit` | ` in SUPP<DOMAINCODE>` |
| item (item code fails to resolve) | button disabled — nothing useful to draft |
| no domain annotation selected | button disabled |

Where:
- `<ITEMCODE>` is `CrfItem.code` for the annotation's `owner.id`.
- `<DOMAINCODE>` is the upper-cased `name` of the `DomainAnnotation`
  currently selected in the dialog's domain-annotation `<Select>`.
  This is the existing `selectedDomainName` memo in
  `AnnotationDialog.tsx` (line 148–153) — reused as-is.

## Data flow

Add one new prop to `AnnotationDialog`:

```ts
/**
 * Resolved `CrfItem.code` for the annotation's owner. `null` for
 * non-item owners (form/option/unit) and for item owners whose item
 * is not in the cached form detail. The SUPP button uses this to
 * draft `<itemCode> in SUPP<domainCode>` content; `null` disables
 * the button when nothing useful can be drafted.
 */
ownerItemCode: string | null;
```

The page resolves it via a new helper next to the existing
`readOwnerNotSubmitted` (line 81–103 of `CrfDetailPage.tsx`):

```ts
function readOwnerItemCode(
  detail: CrfFormDetail | undefined,
  owner: AnnotationOwner,
): string | null {
  if (!detail || owner.kind !== "item") return null;
  const found = detail.items.find((i) => i.item.id === owner.id);
  return found ? found.item.code : null;
}
```

This mirrors the existing owner-lookup pattern — `readOwnerNotSubmitted`
returns `null` when the owner isn't in the cache, the SUPP button
treats that as "disable". Keeps the dialog decoupled from
`CrfFormDetail`.

## Component changes

### `AnnotationDialog.tsx`

- Destructure the new `ownerItemCode` prop (currently the `owner`
  prop is dropped with the `_owner` underscore prefix — that needs
  to go too, since SUPP needs to know the owner kind).
- Compute `suppDisabled` once per render:
  ```ts
  const suppDisabled =
    !selectedDomainName ||
    (owner.kind === "item" && !ownerItemCode);
  ```
- Add a `handleSuppClick`:
  ```ts
  function handleSuppClick() {
    if (suppDisabled) return;
    const domainCode = selectedDomainName!;
    const next =
      owner.kind === "item" && ownerItemCode
        ? `${ownerItemCode} in SUPP${domainCode}`
        : ` in SUPP${domainCode}`;
    setBody((b) => ({ ...b, content: next }));
    setMentionRange(null);
    setAnchorEl(null);
    queueMicrotask(() => {
      inputRef.current?.setSelectionRange(next.length, next.length);
    });
  }
  ```
- Wrap the existing content `TextField` in a horizontal `Stack`
  with the button:
  ```tsx
  <Stack direction="row" spacing={1} alignItems="center">
    <TextField /* existing props */ sx={{ flexGrow: 1 }} />
    <Button
      size="small"
      variant="outlined"
      onClick={handleSuppClick}
      disabled={suppDisabled}
      data-testid="crf-annotation-dialog-supp"
    >
      SUPP
    </Button>
  </Stack>
  ```
- The `@`-mention `Popover` is anchored to the content `TextField`
  and keeps `disableAutoFocus` + `disableEnforceFocus` — unchanged.

### `CrfDetailPage.tsx`

- Thread a new prop into the `<AnnotationDialog />` element
  (line 691–757):
  ```tsx
  ownerItemCode={readOwnerItemCode(
    detail,
    annotationDialog ? annotationDialog.owner : { kind: "form", id },
  )}
  ```
- Inside the dialog, the existing `owner: _owner` destructure
  becomes `owner` (drop the underscore) so the SUPP handler can
  narrow on `owner.kind`.

### i18n — none

`SUPP` is the literal button label in both `en.ts` and `zhCN.ts`.
No new translation keys are added. (The 2026-09-23 author preference.)

## Testing

Eight new Vitest cases added to
`apps/desktop/aegis-desktop/src/test/features/crf/crf-annotation-dialog.test.tsx`,
following the existing testing style in that file:

1. SUPP button is rendered when the dialog is open and a domain
   annotation is selected.
2. SUPP is disabled when no domain annotation is selected
   (`body.domainAnnotationId === 0`).
3. SUPP is disabled when owner is `item` and `ownerItemCode` is `null`.
4. SUPP is enabled when owner is `item` and `ownerItemCode` is `"LBCLSIG"`
   and the selected domain annotation is `LB`.
5. Form-level owner + AE annotation: clicking SUPP sets content to
   `" in SUPPAE"`.
6. Item-level owner + `LBCLSIG` / `LB`: clicking SUPP sets content
   to `"LBCLSIG in SUPPLB"`.
7. Existing content in the field is replaced (not appended) by SUPP.
8. Switching the domain-annotation `<Select>` re-evaluates what SUPP
   fills (e.g. switch from `LB` to `AE` then click → `"LBCLSIG in SUPPAE"`).

The existing test scaffold already wraps the dialog in
`AegisI18nProvider` + `TestQueryProvider`; the new cases reuse it.
The `mountWithSeed` helper accepts a partial override — extend it
(or add a sibling) so callers can pass `owner: { kind: "item"; id: ... }`
and `ownerItemCode: "..."` explicitly.

## File layout

```
apps/desktop/aegis-desktop/src/
├── features/crf/
│   ├── components/
│   │   └── AnnotationDialog.tsx        MODIFIED — add ownerItemCode prop,
│   │                                   SUPP Button, suppDisabled logic
│   └── pages/
│       └── CrfDetailPage.tsx           MODIFIED — resolve ownerItemCode,
│                                       pass it to <AnnotationDialog />
└── test/features/crf/
    └── crf-annotation-dialog.test.tsx  MODIFIED — add 8 SUPP test cases

lib/packages/ui/src/i18n/locales/
├── en.ts                              UNCHANGED
└── zhCN.ts                            UNCHANGED
```

No changes to: `src-tauri/`, `apps/server/aegis-server`,
`lib/crates/*`, `lib/packages/ui/src/**` (other than confirming
no i18n key was added).

## Out of scope

- No backend / wire change.
- No i18n entry for the button.
- No new mutation, no debouncing, no toast.
- No change to the `@`-mention Popover or to the domain-annotation
  `<Select>` UI.

## Risk notes

- **Replace-mode destructive on edit**: clicking SUPP in edit mode
  overwrites an existing annotation's content. This is the user's
  explicit choice (2026-09-23). The submit button still requires a
  non-empty `body.content.trim()` so a stray click on Cancel after
  SUPP leaves the original content untouched.
- **`ownerItemCode` becomes stale if the form detail refetches**:
  the page already invalidates `crf.formDetail(formId)` after every
  mutation, and the dialog is closed and re-opened on each edit.
  Worst case: a renamed item code briefly resolves to the old value
  inside an open dialog, which then closes as soon as the user
  submits. Acceptable.
- **`selectedDomainName` depends on `body.domainAnnotationId`**:
  already wired, but the SUPP button being recomputed every render
  means switching the domain-annotation `<Select>` instantly changes
  what SUPP would fill. No extra wiring needed.
