# Aegis Desktop — SDTM variable Terminology column — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adopt the server's structured `variable_controlled` wire shape on the desktop, and add a Terminology column to the SDTM variable table that opens a drawer listing a codelist's code items.

**Architecture:** The desktop Rust transport mirrors the server's `SdtmVariableControlled` DTO (and its `double_option` three-state update), and the TS side mirrors it in `shared/api/types.ts`. The variable table renders `variable_controlled.code` as clickable chips and raises `onTerminologyClick`; the SDTM domain page owns the drawer's open/close state; the drawer — which lives in the terminology feature — resolves the code to a codelist in the latest SDTM terminology version and lists that codelist's items read-only.

**Tech Stack:** Rust (serde, wiremock), React + TypeScript, TanStack Query, MUI via `@aegis/ui`, Vitest + Testing Library.

**Spec:** `docs/superpowers/specs/2026-10-30-desktop-variable-terminology-column-design.md` — read it, including the **Amendments** section (A1–A5), before starting.

## Global Constraints

- The desktop hand-duplicates the server's wire DTOs; `src-tauri/src/http/domain_model/variable.rs` and `src/shared/api/types.ts` change together, in the same commit.
- `SdtmVariableControlled`'s serde attributes must match the server's `dto::SdtmVariableControlled` exactly: `#[serde(rename_all = "camelCase")]` on the struct, and `#[serde(default, skip_serializing_if = "Option::is_none")]` on both `code` and `format`.
- `UpdateSdtmVariableRequest.variable_controlled` must use a `double_option` deserializer (amendment A1). The server's helper is at `apps/server/aegis-server/src/transport/http/dto.rs` (`fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>` → `Option::<T>::deserialize(deserializer).map(Some)`).
- Every new i18n key goes into **both** `lib/packages/ui/src/i18n/locales/en.ts` and `zhCN.ts` — `lib/packages/ui/src/i18n/registry.test.ts` asserts the two catalogs have the same key set, and `TranslationKey = keyof typeof en`.
- All user-visible strings go through `useI18n()` → `t("path.to.key")`. All MUI primitives and icons come from `@aegis/ui/mui` and `@aegis/ui/icons`.
- Query keys are constructed only through the factory in `src/shared/query/keys.ts` — never inline arrays.
- Cross-feature imports go through a feature's `index.ts` barrel, never through a nested path.
- The server clamps `limit` to 500 (`lib/crates/terminology/src/usecase/terminology_usecase.rs`); the codelist lookup requests `limit: 500`.
- The server rejects a search fragment containing `&`, `|`, `!`, `(`, `)` or `:` with a validation error (amendment A4).
- The codelist full-text index does **not** cover `code_lists.code`; terminology codes are codelist **names** (amendment A3).
- Verification gate before every commit:
  ```bash
  cargo fmt --all -- --check
  cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
  cargo test -p aegis-desktop
  pnpm --filter aegis-desktop typecheck
  pnpm --filter aegis-desktop test
  pnpm --filter @aegis/ui test
  ```

## Review Focus

Five input classes the spec implies but no single task's happy path exercises — most likely to bite first.

1. **Clearing the controlled terminology.** The user empties both lists and saves; the server's column must become NULL, not "unchanged". A plain `Option<Option<T>>` derive silently turns "clear" into "no change" (amendment A1). → pinned in Task 1 (Rust) and Task 2 (TS).
2. **A stored code that no codelist matches** — an NCI c-code like `C66731`, or a lower-cased `vstestcd`. The drawer must show "no code list found", never an empty table that reads as "this codelist has no values", and never a crash. → pinned in Task 4.
3. **A variable with no `variableControlled` at all.** The table shows `—`; the edit drawer seeds empty fields and saving sends no `variableControlled` key. → pinned in Task 2 and Task 3.
4. **A terminology code containing a Postgres-reserved character** (`&`, `|`, `!`, `(`, `)`, `:`). The server rejects the fragment, so the drawer must show the error state with **Retry**, not "not found". → pinned in Task 4.
5. **A version list mixing ADaM and SDTM.** The drawer must resolve against the latest **SDTM** version, not the highest id overall. → pinned in Task 4.

---

### Task 1: Rust — the structured wire shape and the three-state update

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/domain_model/variable.rs`
- Test: `apps/desktop/aegis-desktop/src-tauri/src/http/domain_model/variable.rs` (the existing `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: nothing (this is the first task).
- Produces:
  - `pub struct SdtmVariableControlled { pub code: Option<Vec<String>>, pub format: Option<Vec<String>> }` in `crate::http::domain_model::variable` — `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`, `#[serde(rename_all = "camelCase")]`, both fields `#[serde(default, skip_serializing_if = "Option::is_none")]`.
  - `SdtmVariableViewResponse.variable_controlled: Option<SdtmVariableControlled>`
  - `CreateSdtmVariableRequest.variable_controlled: Option<SdtmVariableControlled>`
  - `UpdateSdtmVariableRequest.variable_controlled: Option<Option<SdtmVariableControlled>>`

- [ ] **Step 1: Write the failing tests**

Add these to the existing `mod tests` in `variable.rs`:

```rust
#[test]
fn view_decodes_controlled_code_and_format_lists() {
    let json = serde_json::json!({
        "id": 1, "domainId": 5, "name": "VSTESTCD",
        "variableControlled": { "code": ["VSTESTCD"], "format": ["Codelist"] },
        "variableType": "Character", "variableCore": "Req",
        "variableSequence": 1, "descriptions": [],
        "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
    });
    let v: SdtmVariableViewResponse = serde_json::from_value(json).unwrap();
    assert_eq!(
        v.variable_controlled,
        Some(SdtmVariableControlled {
            code: Some(vec!["VSTESTCD".into()]),
            format: Some(vec!["Codelist".into()]),
        })
    );
}

#[test]
fn view_decodes_null_controlled_as_none() {
    let json = serde_json::json!({
        "id": 1, "domainId": 5, "name": "VSTESTCD",
        "variableControlled": null,
        "variableType": "Character", "variableCore": "Req",
        "variableSequence": 1, "descriptions": [],
        "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
    });
    let v: SdtmVariableViewResponse = serde_json::from_value(json).unwrap();
    assert_eq!(v.variable_controlled, None);
}

#[test]
fn view_decodes_absent_format_list_as_none() {
    let json = serde_json::json!({
        "id": 1, "domainId": 5, "name": "VSTESTCD",
        "variableControlled": { "code": ["VSTESTCD"] },
        "variableType": "Character", "variableCore": "Req",
        "variableSequence": 1, "descriptions": [],
        "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z"
    });
    let v: SdtmVariableViewResponse = serde_json::from_value(json).unwrap();
    assert_eq!(
        v.variable_controlled,
        Some(SdtmVariableControlled {
            code: Some(vec!["VSTESTCD".into()]),
            format: None,
        })
    );
}

#[test]
fn update_request_decodes_null_controlled_as_clear() {
    // The Tauri payload is where the webview's `null` arrives. A plain
    // `Option<Option<T>>` derive maps it to the outer `None`, the key
    // is then dropped by `skip_serializing_if`, and the server reads
    // "unchanged" instead of "clear".
    let body: UpdateSdtmVariableRequest =
        serde_json::from_str(r#"{"variableControlled":null}"#).unwrap();
    assert_eq!(body.variable_controlled, Some(None));
}

#[test]
fn update_request_decodes_absent_controlled_as_unchanged() {
    let body: UpdateSdtmVariableRequest =
        serde_json::from_str(r#"{"name":"renamed"}"#).unwrap();
    assert_eq!(body.variable_controlled, None);
}

#[test]
fn update_request_decodes_controlled_object_as_replace() {
    let body: UpdateSdtmVariableRequest =
        serde_json::from_str(r#"{"variableControlled":{"code":["VSTESTCD"]}}"#).unwrap();
    assert_eq!(
        body.variable_controlled,
        Some(Some(SdtmVariableControlled {
            code: Some(vec!["VSTESTCD".into()]),
            format: None,
        }))
    );
}

#[test]
fn update_request_serializes_controlled_object() {
    let body = UpdateSdtmVariableRequest {
        variable_controlled: Some(Some(SdtmVariableControlled {
            code: Some(vec!["VSTESTCD".into()]),
            format: None,
        })),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_string(&body).unwrap(),
        r#"{"variableControlled":{"code":["VSTESTCD"]}}"#
    );
}

#[test]
fn update_request_serializes_controlled_null_as_clear() {
    let body = UpdateSdtmVariableRequest {
        variable_controlled: Some(None),
        ..Default::default()
    };
    assert_eq!(
        serde_json::to_string(&body).unwrap(),
        r#"{"variableControlled":null}"#
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p aegis-desktop --lib http::domain_model::variable`
Expected: compile error — `cannot find type SdtmVariableControlled in module`.

- [ ] **Step 3: Implement the DTO**

In `variable.rs`, above `SdtmVariableViewResponse`, add:

```rust
/// Controlled-terminology metadata carried by an SDTM variable.
///
/// Mirrors the server's `dto::SdtmVariableControlled`: the same
/// camelCase rename, and both lists default to `None` when the key is
/// absent or explicitly `null`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SdtmVariableControlled {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<Vec<String>>,
}
```

Change the three fields to the types listed in this task's Interfaces block. `CreateSdtmVariableRequest.variable_controlled` and `SdtmVariableViewResponse.variable_controlled` keep their existing `#[serde(skip_serializing_if = "Option::is_none")]`.

Add a private helper next to the structs (module scope, not `pub`):

```rust
/// Deserialise a nullable field into `Option<Option<T>>` so all three
/// states stay distinguishable across the Tauri hop:
/// a missing key is "leave the column alone", an explicit `null` is
/// "clear the column", and a value is "replace the column".
///
/// A plain `Option<Option<T>>` derive collapses the first two — serde
/// maps `null` to the outer `None`, which `skip_serializing_if` then
/// drops from the outgoing request.
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}
```

and annotate the update field:

```rust
    /// Three-state: absent = unchanged, `null` = clear, value = replace.
    #[serde(default, deserialize_with = "double_option")]
    pub variable_controlled: Option<Option<SdtmVariableControlled>>,
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p aegis-desktop --lib http::domain_model::variable`
Expected: PASS — all pre-existing tests in the module plus the eight new ones.

- [ ] **Step 5: Run the Rust gate**

Run: `cargo fmt --all -- --check && cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings && cargo test -p aegis-desktop`
Expected: no formatting diff, no clippy warnings, all tests green.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/domain_model/variable.rs
git commit -m "feat(aegis-desktop): adopt the structured variable_controlled wire shape

The server now sends variable_controlled as { code?, format? } instead of
a free-text string. Mirror the DTO in the desktop transport, including
the server's double_option deserializer so the three-state update
(absent = unchanged, null = clear, value = replace) survives the Tauri
hop."
```

---

### Task 2: TypeScript wire types and the variable edit drawer

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts` (around line 387, the SDTM variable block)
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts` (the type re-export list, around line 580)
- Modify: `apps/desktop/aegis-desktop/src/features/domain-model/components/VariableEditDrawer.tsx`
- Modify: `lib/packages/ui/src/i18n/locales/en.ts` (the `domainModel.sdtm.variable.field.*` block, around line 295)
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts` (the same block, around line 286)
- Test: `apps/desktop/aegis-desktop/src/test/features/domain-model/variable-edit-drawer.test.tsx`

**Interfaces:**
- Consumes: `SdtmVariableControlled` from Task 1's Rust DTO (same wire shape).
- Produces:
  - `export interface SdtmVariableControlled { code?: string[]; format?: string[] }` in `src/shared/api/types.ts`, re-exported from `src/shared/api/index.ts` alongside the other SDTM variable types.
  - `export function parseControlledList(text: string): string[]` in `VariableEditDrawer.tsx` — splits on `,`, trims each part, drops empty parts.
  - `export function controlledListsEqual(a: readonly string[], b: readonly string[]): boolean` in `VariableEditDrawer.tsx` — element-wise equality, order-sensitive.

- [ ] **Step 1: Write the failing tests**

Append to `variable-edit-drawer.test.tsx`:

```tsx
describe("parseControlledList", () => {
  it("splits a comma-separated list and trims each part", () => {
    expect(parseControlledList("VSTESTCD, AESEV")).toEqual(["VSTESTCD", "AESEV"]);
  });

  it("drops empty parts", () => {
    expect(parseControlledList("A, , B,")).toEqual(["A", "B"]);
  });

  it("returns an empty list for blank input", () => {
    expect(parseControlledList("   ")).toEqual([]);
  });
});

describe("controlledListsEqual", () => {
  it("compares element by element", () => {
    expect(controlledListsEqual(["A", "B"], ["A", "B"])).toBe(true);
    expect(controlledListsEqual(["A"], ["A", "B"])).toBe(false);
    expect(controlledListsEqual([], [])).toBe(true);
  });
});
```

and, in the existing `describe("VariableEditDrawer")` block, add a fixture and four behaviour tests:

```tsx
const controlledRow: SdtmVariableView = {
  ...sample,
  variableControlled: { code: ["VSTESTCD", "AESEV"], format: ["Codelist"] },
};

it("seeds the two controlled-terminology fields from the row", () => {
  renderDrawer({ open: true, mode: "edit", row: controlledRow });
  expect(
    (screen.getByRole("textbox", { name: /controlled terminology \(code list\)/i }) as HTMLInputElement)
      .value,
  ).toBe("VSTESTCD, AESEV");
  expect(
    (screen.getByRole("textbox", { name: /controlled terminology \(format\)/i }) as HTMLInputElement)
      .value,
  ).toBe("Codelist");
});

it("sends the code list as an array in create mode", async () => {
  const onCreate = vi.fn();
  renderDrawer({ open: true, mode: "create", onCreate });
  await userEvent.type(screen.getByRole("textbox", { name: /^name$/i }), "VSTESTCD");
  await userEvent.type(
    screen.getByRole("textbox", { name: /controlled terminology \(code list\)/i }),
    "VSTESTCD",
  );
  await userEvent.click(screen.getByRole("button", { name: /^create$/i }));
  const input = onCreate.mock.calls[0][0] as CreateSdtmVariableInput;
  expect(input.variableControlled).toEqual({ code: ["VSTESTCD"] });
});

it("sends null when the user clears a populated field in edit mode", async () => {
  const onUpdate = vi.fn();
  renderDrawer({ open: true, mode: "edit", row: controlledRow, onUpdate });
  await userEvent.clear(
    screen.getByRole("textbox", { name: /controlled terminology \(code list\)/i }),
  );
  await userEvent.clear(
    screen.getByRole("textbox", { name: /controlled terminology \(format\)/i }),
  );
  await userEvent.click(screen.getByRole("button", { name: /save/i }));
  const body = onUpdate.mock.calls[0][1] as UpdateSdtmVariableInput;
  expect(body.variableControlled).toBeNull();
});

it("sends nothing when the controlled terminology is unchanged in edit mode", async () => {
  const onUpdate = vi.fn();
  renderDrawer({ open: true, mode: "edit", row: controlledRow, onUpdate });
  await userEvent.click(screen.getByRole("button", { name: /save/i }));
  const body = onUpdate.mock.calls[0][1] as UpdateSdtmVariableInput;
  expect(body.variableControlled).toBeUndefined();
});

it("sends no variableControlled when a variable without one is saved unchanged", async () => {
  const onUpdate = vi.fn();
  renderDrawer({ open: true, mode: "edit", row: sample, onUpdate });
  await userEvent.click(screen.getByRole("button", { name: /save/i }));
  const body = onUpdate.mock.calls[0][1] as UpdateSdtmVariableInput;
  expect(body.variableControlled).toBeUndefined();
});
```

Import the two helpers at the top of the test file:

```tsx
import {
  controlledListsEqual,
  parseControlledList,
  VariableEditDrawer,
} from "../../../features/domain-model/components/VariableEditDrawer";
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/domain-model/variable-edit-drawer.test.tsx`
Expected: FAIL — `parseControlledList` is not exported, and the two controlled-terminology textboxes do not exist.

- [ ] **Step 3: Add the TypeScript types**

In `src/shared/api/types.ts`, in the SDTM variable block, add:

```ts
export interface SdtmVariableControlled {
  code?: string[];
  format?: string[];
}
```

and change three fields:

- `SdtmVariableView.variableControlled?: SdtmVariableControlled;`
- `CreateSdtmVariableInput.variableControlled?: SdtmVariableControlled;`
- `UpdateSdtmVariableInput.variableControlled?: SdtmVariableControlled | null;` — `null` means "clear" (three-state semantics: absent = no change, null = clear, value = replace).

In `src/shared/api/index.ts`, add `SdtmVariableControlled` to the type re-export list next to `SdtmVariableCore`.

- [ ] **Step 4: Rewrite the drawer's controlled-terminology fields**

In `VariableEditDrawer.tsx`:

1. Add the two exported helpers at module scope:

```tsx
/** Split a comma-separated free-text field into a list of values. */
export function parseControlledList(text: string): string[] {
  return text
    .split(",")
    .map((part) => part.trim())
    .filter((part) => part !== "");
}

/** Order-sensitive element-wise comparison of two value lists. */
export function controlledListsEqual(
  a: readonly string[],
  b: readonly string[],
): boolean {
  return a.length === b.length && a.every((value, i) => value === b[i]);
}
```

2. Replace the `variableControlled` state with two string states:

```tsx
const [controlledCodes, setControlledCodes] = useState("");
const [controlledFormats, setControlledFormats] = useState("");
```

3. In the seeding effect (`useEffect` on `[open, mode, row]`), edit mode sets
   `setControlledCodes((row.variableControlled?.code ?? []).join(", "))` and
   `setControlledFormats((row.variableControlled?.format ?? []).join(", "))`;
   create mode sets both to `""`.

4. Replace the single "Controlled vocabulary (CCDD)" `TextField` with two:

```tsx
<TextField
  size="small"
  label={t("domainModel.sdtm.variable.field.variableControlledCode")}
  helperText={t("domainModel.sdtm.variable.field.variableControlledHint")}
  value={controlledCodes}
  onChange={(e) => setControlledCodes(e.target.value)}
  disabled={!canMutate}
/>
<TextField
  size="small"
  label={t("domainModel.sdtm.variable.field.variableControlledFormat")}
  helperText={t("domainModel.sdtm.variable.field.variableControlledHint")}
  value={controlledFormats}
  onChange={(e) => setControlledFormats(e.target.value)}
  disabled={!canMutate}
/>
```

5. In `handleSubmit`, compute `const codes = parseControlledList(controlledCodes);` and
   `const formats = parseControlledList(controlledFormats);`, then:

   - create mode — replace the current `variableControlled` expression with:

     ```tsx
     variableControlled:
       codes.length === 0 && formats.length === 0
         ? undefined
         : {
             code: codes.length > 0 ? codes : undefined,
             format: formats.length > 0 ? formats : undefined,
           },
     ```

   - edit mode — replace the current "Only send variableControlled when it actually changed" block with:

     ```tsx
     const currentCodes = row.variableControlled?.code ?? [];
     const currentFormats = row.variableControlled?.format ?? [];
     const changed =
       !controlledListsEqual(codes, currentCodes) ||
       !controlledListsEqual(formats, currentFormats);
     if (changed) {
       body.variableControlled =
         codes.length === 0 && formats.length === 0
           ? null
           : {
               code: codes.length > 0 ? codes : undefined,
               format: formats.length > 0 ? formats : undefined,
             };
     }
     ```

- [ ] **Step 5: Update the i18n catalogs**

In `lib/packages/ui/src/i18n/locales/en.ts`, in the `domainModel.sdtm.variable.field.*` block, replace the key
`"domainModel.sdtm.variable.field.variableControlled": "Controlled vocabulary (CCDD)"` with:

```ts
  "domainModel.sdtm.variable.field.variableControlledCode": "Controlled terminology (code list)",
  "domainModel.sdtm.variable.field.variableControlledFormat": "Controlled terminology (format)",
  "domainModel.sdtm.variable.field.variableControlledHint": "Comma-separated values",
```

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, replace the same key with:

```ts
  "domainModel.sdtm.variable.field.variableControlledCode": "受控术语（代码列表）",
  "domainModel.sdtm.variable.field.variableControlledFormat": "受控术语（格式）",
  "domainModel.sdtm.variable.field.variableControlledHint": "以逗号分隔的值",
```

(`zhCN.ts` quotes this block with double quotes; match the surrounding style.)

- [ ] **Step 6: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/domain-model/variable-edit-drawer.test.tsx && pnpm --filter @aegis/ui test`
Expected: PASS — the drawer tests and the i18n registry test (key sets still match).

- [ ] **Step 7: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: no errors. `variableControlled` is now an object everywhere, so any other site that treated it as a string will fail to compile — that is the point; fix it by following the new type.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts \
        apps/desktop/aegis-desktop/src/shared/api/index.ts \
        apps/desktop/aegis-desktop/src/features/domain-model/components/VariableEditDrawer.tsx \
        apps/desktop/aegis-desktop/src/test/features/domain-model/variable-edit-drawer.test.tsx \
        lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(aegis-desktop): edit variable_controlled as two value lists

The controlled terminology is now { code[], format[] } on the wire, so the
drawer's single free-text field becomes two comma-separated fields. The
update path keeps its three states: unchanged sends nothing, emptied
sends null (clear), edited sends the new lists."
```

---

### Task 3: The Terminology column in the variable table

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/domain-model/components/VariableTable.tsx`
- Modify: `lib/packages/ui/src/i18n/locales/en.ts` (the `domainModel.sdtm.detail.col.*` block, around line 281)
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts` (the same block, around line 272)
- Test: `apps/desktop/aegis-desktop/src/test/features/domain-model/variable-table.test.tsx`

**Interfaces:**
- Consumes: `SdtmVariableControlled` from Task 2.
- Produces: `VariableTableProps.onTerminologyClick?: (code: string) => void` — optional; when absent the chips render but do nothing on click.

- [ ] **Step 1: Write the failing tests**

In `variable-table.test.tsx`, extend the `variables` fixture's second entry with
`variableControlled: { code: ["VSTESTCD"] }`, then add to the `describe("VariableTable")` block:

```tsx
it("renders a Terminology column header", () => {
  renderTable({});
  expect(
    screen.getByRole("columnheader", { name: "Terminology" }),
  ).toBeInTheDocument();
});

it("renders the variable's controlled terminology codes in the Terminology column", () => {
  renderTable({});
  const row2 = screen.getByText("AESEV").closest("tr")!;
  const cells = within(row2).getAllByRole("cell");
  // cells[0] = drag handle, cells[1] = name, cells[2] = type/core chips,
  // cells[3] = label, cells[4] = terminology, cells[5] = role, cells[6] = actions
  expect(within(cells[4]).getByText("VSTESTCD")).toBeInTheDocument();
});

it("renders a dash in the Terminology column when the variable has no codes", () => {
  renderTable({});
  const row1 = screen.getByText("AETERM").closest("tr")!;
  const cells = within(row1).getAllByRole("cell");
  expect(cells[4]).toHaveTextContent("—");
});

it("calls onTerminologyClick with the clicked code", async () => {
  const onTerminologyClick = vi.fn();
  renderTable({ onTerminologyClick });
  await userEvent.click(screen.getByRole("button", { name: "VSTESTCD" }));
  expect(onTerminologyClick).toHaveBeenCalledWith("VSTESTCD");
});
```

and extend `renderTable`'s props with `onTerminologyClick?: (code: string) => void`, passing it to `<VariableTable>`.

Also update the two existing tests whose cell indices shift — the comment in
`"renders the type and core chips in their own column, separate from the name"` becomes
`cells[0] = drag handle, cells[1] = name, cells[2] = type/core chips, cells[3] = label, cells[4] = terminology, cells[5] = role, cells[6] = actions`.
The assertions in `"swaps the label cell when selectedLang changes"` still target `cells[3]` (the label) and stay as they are.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `pnpm --filter aegis-desktop test -- src/test/features/domain-model/variable-table.test.tsx`
Expected: FAIL — no "Terminology" column header, and no button named "VSTESTCD".

- [ ] **Step 3: Add the column**

In `VariableTable.tsx`:

1. `VariableTableProps` gains `onTerminologyClick?: (code: string) => void;`
2. `DraggableRowProps` gains the same field; `DraggableRow` destructures it.
3. In `DraggableRow`, compute `const codes = row.variableControlled?.code ?? [];` and add a cell **after** the label cell and **before** the role cell:

```tsx
<TableCell>
  {codes.length === 0 ? (
    "—"
  ) : (
    <Box sx={{ display: "flex", flexWrap: "wrap", gap: 0.5 }}>
      {codes.map((code) => (
        <Chip
          key={code}
          size="small"
          variant="outlined"
          label={code}
          component="button"
          onClick={() => onTerminologyClick?.(code)}
        />
      ))}
    </Box>
  )}
</TableCell>
```

`component="button"` matters: `@dnd-kit/dom`'s default `PointerSensor` has a `preventActivation` that suppresses drag activation when the pointer-down target is an interactive element, so the chip is clickable inside the draggable row.

4. In the table header, add `<TableCell>{t("domainModel.sdtm.detail.col.terminology")}</TableCell>` between the label header cell and the role header cell.
5. Change the empty body's `colSpan={6}` to `colSpan={7}`.
6. `VariableTable` destructures `onTerminologyClick` and passes it to every `DraggableRow`.

- [ ] **Step 4: Update the i18n catalogs**

In `en.ts`, after the `"domainModel.sdtm.detail.col.label"` line, add:

```ts
  "domainModel.sdtm.detail.col.terminology": "Terminology",
```

In `zhCN.ts`, in the same block, add:

```ts
  "domainModel.sdtm.detail.col.terminology": "术语",
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/domain-model/variable-table.test.tsx && pnpm --filter @aegis/ui test`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/domain-model/components/VariableTable.tsx \
        apps/desktop/aegis-desktop/src/test/features/domain-model/variable-table.test.tsx \
        lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(aegis-desktop): show controlled terminology in the variable table

The variable table gains a Terminology column next to Label that lists
each codelist reference of the variable as a clickable chip."
```

---

### Task 4: The terminology lookup hooks and the code-items drawer

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/query/keys.ts` (the `terminology` block, around line 25)
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/data/list.ts`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/index.ts`
- Modify: `apps/desktop/aegis-desktop/src/features/terminology/index.ts`
- Create: `apps/desktop/aegis-desktop/src/features/terminology/components/TerminologyCodeItemsDrawer.tsx`
- Modify: `lib/packages/ui/src/i18n/locales/en.ts` (the `terminology.codeitem.*` block, around line 241)
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts` (the same block, around line 235)
- Test: `apps/desktop/aegis-desktop/src/test/features/terminology/terminology-code-items-drawer.test.tsx`

**Interfaces:**
- Consumes: `api.listCodeLists(versionId, { fragment, offset, limit })` and `api.listCodeItems(codelistId, { fragment, offset, limit })` from `src/shared/api`; `useListTerminologyVersions` and `useListCodeItems` from `features/terminology/data/list.ts`.
- Produces:
  - `queryKeys.terminology.codeListByName(versionId: number, name: string)` → `["terminology", "codeListByName", versionId, name] as const` — a **separate** key from `queryKeys.terminology.codeLists`, because that one belongs to the 20-per-page infinite query and must not be reused for a single 500-row lookup.
  - `export function useLatestSdtmTerminologyVersionId(): number | null` in `features/terminology/data/list.ts` — the highest `id` among the versions with `kind === "sdtm"`, or `null` when there is none.
  - `export function useFindCodeListByName(versionId: number | null, name: string | null): UseQueryResult<PagedCodeListListResponse, ApiError> & { codelist: CodeListView | null }` in the same file.
  - `export interface TerminologyCodeItemsDrawerProps { open: boolean; code: string | null; onClose: () => void }` and `export function TerminologyCodeItemsDrawer(props: TerminologyCodeItemsDrawerProps)` in `features/terminology/components/TerminologyCodeItemsDrawer.tsx`.

- [ ] **Step 1: Write the failing test**

Create `src/test/features/terminology/terminology-code-items-drawer.test.tsx`:

```tsx
import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AegisI18nProvider } from "@aegis/ui/i18n";
import { AegisThemeProvider } from "@aegis/ui/theme";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { mockCommands, mockInvoke } from "../../helpers/tauri-mock";
import { TestQueryProvider } from "../../helpers/test-query-provider";
import { TerminologyCodeItemsDrawer } from
  "../../../features/terminology/components/TerminologyCodeItemsDrawer";

const sdtmVersion = {
  id: 5,
  kind: "sdtm",
  name: "2024-06-28",
  createdAt: "",
  updatedAt: "",
};

const adamVersion = {
  id: 9,
  kind: "adam",
  name: "2024-09-30",
  createdAt: "",
  updatedAt: "",
};

const targetCodelist = {
  id: 11,
  versionId: 5,
  code: "C66741",
  extensible: true,
  name: "VSTESTCD",
  submissionValue: "VSTESTCD",
  synonym: "",
  definition: "",
  nciPreferredTerm: "",
  createdAt: "",
  updatedAt: "",
};

const decoyCodelist = {
  ...targetCodelist,
  id: 12,
  name: "Other",
  definition: "mentions VSTESTCD in its definition",
};

const codeItem = {
  id: 21,
  codelistId: 11,
  versionId: 5,
  code: "C174106",
  submissionValue: "PASS",
  synonym: "",
  definition: "",
  nciPreferredTerm: "",
  createdAt: "",
  updatedAt: "",
};

function setupMocks(overrides?: {
  versions?: unknown[];
  codeLists?: { items: unknown[] };
  codeItems?: { items: unknown[] };
}) {
  mockInvoke.mockReset();
  mockCommands({
    list_terminology_versions: () =>
      overrides?.versions ?? [sdtmVersion, adamVersion],
    list_code_lists: () => overrides?.codeLists ?? { items: [decoyCodelist, targetCodelist] },
    list_code_items: () => overrides?.codeItems ?? { items: [codeItem] },
  });
}

function renderDrawer(props?: { open?: boolean; code?: string | null }) {
  return render(
    <AegisThemeProvider>
      <TestQueryProvider>
        <AegisI18nProvider>
          <TerminologyCodeItemsDrawer
            open={props?.open ?? true}
            code={props?.code ?? "VSTESTCD"}
            onClose={() => {}}
          />
        </AegisI18nProvider>
      </TestQueryProvider>
    </AegisThemeProvider>,
  );
}

describe("TerminologyCodeItemsDrawer", () => {
  afterEach(() => cleanup());

  it("resolves the code against the latest SDTM version and lists the code list's items", async () => {
    setupMocks();
    renderDrawer();

    expect(await screen.findByText("PASS")).toBeInTheDocument();
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "list_code_lists",
        expect.objectContaining({ versionId: 5, fragment: "VSTESTCD" }),
      );
    });
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "list_code_items",
        expect.objectContaining({ codelistId: 11 }),
      );
    });
  });

  it("picks the code list whose name matches exactly, not a full-text hit", async () => {
    setupMocks();
    renderDrawer();
    expect(await screen.findByText("PASS")).toBeInTheDocument();
    // The decoy code list (id 12) also mentions the code in its definition;
    // only the exact name match (id 11) is used.
    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter((c) => c[0] === "list_code_items");
      expect(calls.every((c) => (c[1] as { codelistId: number }).codelistId === 11)).toBe(true);
    });
  });

  it("shows a not-found state when no code list matches", async () => {
    setupMocks({ codeLists: { items: [decoyCodelist] } });
    renderDrawer();
    expect(
      await screen.findByText(/no code list found for VSTESTCD/i),
    ).toBeInTheDocument();
  });

  it("shows a not-found state when the code list name only matches case-insensitively", async () => {
    // The search is case-insensitive, so `vstestcd` still finds the code
    // list; the resolution must accept it rather than reporting a miss.
    setupMocks();
    renderDrawer({ code: "vstestcd" });
    expect(await screen.findByText("PASS")).toBeInTheDocument();
  });

  it("shows an error state when the code list lookup fails", async () => {
    mockInvoke.mockReset();
    mockCommands({
      list_terminology_versions: () => [sdtmVersion],
      list_code_lists: () => {
        throw { kind: "http", status: 400, code: "invalid_fragment", message: "bad fragment" };
      },
      list_code_items: () => ({ items: [] }),
    });
    renderDrawer();
    expect(await screen.findByText(/failed to load/i)).toBeInTheDocument();
  });

  it("shows a distinct state when no SDTM terminology version exists", async () => {
    setupMocks({ versions: [adamVersion] });
    renderDrawer();
    expect(
      await screen.findByText(/no sdtm terminology version/i),
    ).toBeInTheDocument();
  });

  it("renders the items read-only — no create, edit, or delete affordances", async () => {
    setupMocks();
    renderDrawer();
    await screen.findByText("PASS");
    expect(screen.queryByRole("button", { name: /create code item/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /edit code item/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /delete C174106/i })).not.toBeInTheDocument();
  });

  it("does not query while the drawer is closed", async () => {
    setupMocks();
    renderDrawer({ open: false });
    await waitFor(() => {
      const calls = mockInvoke.mock.calls.filter((c) => c[0] === "list_code_lists");
      expect(calls.length).toBe(0);
    });
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter aegis-desktop test -- src/test/features/terminology/terminology-code-items-drawer.test.tsx`
Expected: FAIL — the module `features/terminology/components/TerminologyCodeItemsDrawer` does not exist.

- [ ] **Step 3: Add the query key**

In `src/shared/query/keys.ts`, inside the `terminology` block, add:

```ts
    codeListByName: (versionId: number, name: string) =>
      ["terminology", "codeListByName", versionId, name] as const,
```

- [ ] **Step 4: Add the two hooks**

In `features/terminology/data/list.ts`, append:

```ts
/**
 * Id of the most recently imported SDTM terminology version, or `null`
 * when none exists. The server orders the version list by `id ASC`, so
 * the highest id is the latest import. ADaM versions are ignored.
 */
export function useLatestSdtmTerminologyVersionId(): number | null {
  const versions = useListTerminologyVersions();
  const sdtmVersions = (versions.data ?? []).filter((v) => v.kind === "sdtm");
  return sdtmVersions.reduce<number | null>(
    (latest, v) => (latest == null || v.id > latest ? v.id : latest),
    null,
  );
}

/**
 * Find the code list whose `name` equals `name` in a terminology
 * version. The server search is a full-text match, so several rows can
 * come back; the exact name match wins, and a case-insensitive match is
 * accepted as a fallback (the search itself is case-insensitive).
 *
 * The lookup asks for the server's maximum page size so a common token
 * does not push the exact match off the first page. It uses its own
 * query key — `codeLists` belongs to the 20-per-page infinite query.
 */
export function useFindCodeListByName(
  versionId: number | null,
  name: string | null,
) {
  const query = useQuery<PagedCodeListListResponse, ApiError>({
    queryKey: queryKeys.terminology.codeListByName(versionId ?? 0, name ?? ""),
    queryFn: () =>
      api.listCodeLists(versionId!, { fragment: name!, offset: 0, limit: 500 }),
    enabled: versionId != null && versionId > 0 && !!name,
  });
  const codelist = useMemo(() => {
    const items = query.data?.items ?? [];
    if (!name) return null;
    return (
      items.find((c) => c.name === name) ??
      items.find((c) => c.name.toLowerCase() === name.toLowerCase()) ??
      null
    );
  }, [query.data, name]);
  return { ...query, codelist };
}
```

Add `useMemo` to the `@tanstack/react-query` import list only if it is not already imported from React — it is not in that file today, so import it from `react`: `import { useMemo } from "react";`.

- [ ] **Step 5: Create the drawer component**

Create `features/terminology/components/TerminologyCodeItemsDrawer.tsx`:

```tsx
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Drawer,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { errorMessage } from "../../../shared/api/error";
import { InfiniteScrollSentinel } from "../../../shared/components/InfiniteScrollSentinel";
import {
  useFindCodeListByName,
  useLatestSdtmTerminologyVersionId,
  useListCodeItems,
} from "../data";
import { CodeItemTable } from "./CodeItemTable";

export interface TerminologyCodeItemsDrawerProps {
  open: boolean;
  code: string | null;
  onClose: () => void;
}

/**
 * Lists the code items of the code list a terminology code refers to.
 *
 * Resolution: the latest SDTM terminology version → the code list whose
 * `name` matches the code → that code list's items. The list is read
 * only: no create, edit, or delete affordances, and no
 * "same code in other codelists" dialog.
 *
 * Owns its own queries — the caller controls `open`, `code`, and
 * `onClose` only.
 */
export function TerminologyCodeItemsDrawer({
  open,
  code,
  onClose,
}: TerminologyCodeItemsDrawerProps) {
  const { t } = useI18n();
  const activeCode = open ? code : null;
  const versionId = useLatestSdtmTerminologyVersionId();
  const codelistsQuery = useFindCodeListByName(versionId, activeCode);
  const codelist = codelistsQuery.codelist;

  const itemsQuery = useListCodeItems(codelist?.id ?? null);
  const rows = itemsQuery.data?.pages.flatMap((p) => p.items) ?? [];
  const hasMore = itemsQuery.hasNextPage ?? false;

  const noVersion = versionId == null;
  const resolving = codelistsQuery.isLoading;
  const lookupError = codelistsQuery.error ?? itemsQuery.error;

  return (
    <Drawer
      anchor="right"
      open={open && !!code}
      onClose={onClose}
      slotProps={{ paper: { sx: { width: 720 } } }}
    >
      <Box sx={{ p: 3, display: "flex", flexDirection: "column", gap: 2 }}>
        <Typography variant="h6">
          {t("terminology.codeitem.drawer.title", { code: code ?? "" })}
        </Typography>

        {noVersion ? (
          <Alert severity="info">
            {t("terminology.codeitem.drawer.noVersion")}
          </Alert>
        ) : resolving ? (
          <Box sx={{ display: "flex", justifyContent: "center", py: 4 }}>
            <CircularProgress />
          </Box>
        ) : lookupError ? (
          <Box sx={{ display: "flex", flexDirection: "column", gap: 1 }}>
            <Alert severity="error">
              {t("terminology.codeitem.loadFailed", {
                message: errorMessage(lookupError),
              })}
            </Alert>
            <Box>
              <Button
                onClick={() => {
                  void codelistsQuery.refetch();
                  void itemsQuery.refetch();
                }}
              >
                {t("common.retry")}
              </Button>
            </Box>
          </Box>
        ) : !codelist ? (
          <Typography color="text.secondary">
            {t("terminology.codeitem.drawer.notFound", { code: code ?? "" })}
          </Typography>
        ) : (
          <>
            <Typography variant="body2" color="text.secondary">
              {`${codelist.name} · ${codelist.code}`}
            </Typography>
            <CodeItemTable
              rows={rows}
              loading={itemsQuery.isLoading}
              mutationLoading={false}
              error={itemsQuery.error}
              canMutate={false}
              onRetry={() => void itemsQuery.refetch()}
              onCreate={() => {}}
              onEdit={() => {}}
              onDelete={() => {}}
              emptyMessage={t("terminology.codeitem.empty")}
              bottomSlot={(scrollEl) => (
                <InfiniteScrollSentinel
                  root={scrollEl}
                  onIntersect={() => void itemsQuery.fetchNextPage()}
                  hasMore={hasMore}
                  loading={itemsQuery.isFetchingNextPage}
                />
              )}
            />
          </>
        )}
      </Box>
    </Drawer>
  );
```

- [ ] **Step 6: Export the component through the feature barrels**

Create `features/terminology/components/index.ts`:

```ts
export * from "./CodeItemTable";
export * from "./TerminologyCodeItemsDrawer";
```

(Including `CodeItemTable` makes the read-only table importable from other features through the barrel — the desktop guideline requires cross-feature imports to go through a barrel.)

In `features/terminology/index.ts`, add `export * from "./components";`.

- [ ] **Step 7: Update the i18n catalogs**

In `en.ts`, after the `'terminology.codeitem.readOnly'` line, add:

```ts
  'terminology.codeitem.drawer.title': 'Code items for {code}',
  'terminology.codeitem.drawer.notFound': 'No code list found for {code}',
  'terminology.codeitem.drawer.noVersion':
    'No SDTM terminology version has been imported.',
```

In `zhCN.ts`, after the `'terminology.codeitem.readOnly'` line, add:

```ts
  'terminology.codeitem.drawer.title': '{code} 的代码项',
  'terminology.codeitem.drawer.notFound': '未找到 {code} 对应的代码列表',
  'terminology.codeitem.drawer.noVersion': '尚未导入 SDTM 术语版本。',
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `pnpm --filter aegis-desktop test -- src/test/features/terminology/terminology-code-items-drawer.test.tsx && pnpm --filter @aegis/ui test`
Expected: PASS.

If the "does not query while the drawer is closed" test fails because the version query still runs, that is fine and expected — assert on `list_code_lists` only (as written), not on `list_terminology_versions`.

- [ ] **Step 9: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: no errors.

- [ ] **Step 10: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/query/keys.ts \
        apps/desktop/aegis-desktop/src/features/terminology/data/list.ts \
        apps/desktop/aegis-desktop/src/features/terminology/components/index.ts \
        apps/desktop/aegis-desktop/src/features/terminology/index.ts \
        apps/desktop/aegis-desktop/src/features/terminology/components/TerminologyCodeItemsDrawer.tsx \
        apps/desktop/aegis-desktop/src/test/features/terminology/terminology-code-items-drawer.test.tsx \
        lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(aegis-desktop): resolve a terminology code to its code items

Adds the lookup hooks (latest SDTM terminology version, code list by
exact name) and a read-only drawer that lists the code items of the
resolved code list."
```

---

### Task 5: Open the drawer from the SDTM domain page

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainDetail.tsx`
- Test: `apps/desktop/aegis-desktop/src/test/features/domain-model/sdtm-domain-detail.test.tsx`

**Interfaces:**
- Consumes: `TerminologyCodeItemsDrawer` from `features/terminology` (imported through the barrel: `import { TerminologyCodeItemsDrawer } from "../../terminology";`).
- Produces: none — this task wires the existing pieces together.

- [ ] **Step 1: Write the failing test**

In `sdtm-domain-detail.test.tsx`, give the second fixture variable a controlled terminology and add the terminology commands to `setupMocks`:

```tsx
const variables = [
  {
    id: 1,
    domainId: 7,
    name: "AETERM",
    variableType: "Character",
    variableCore: "Req",
    variableRole: "Topic",
    variableSequence: 1,
    descriptions: [{ lang: "en", details: { label: "Term" } }],
    createdAt: "",
    updatedAt: "",
  },
  {
    id: 2,
    domainId: 7,
    name: "AESEV",
    variableType: "Character",
    variableCore: "Req",
    variableRole: "Record Qualifier",
    variableSequence: 2,
    variableControlled: { code: ["VSTESTCD"] },
    descriptions: [{ lang: "en", details: { label: "Severity" } }],
    createdAt: "",
    updatedAt: "",
  },
];
```

and add these handlers to `mockCommands` in `setupMocks`:

```tsx
    list_terminology_versions: () => [
      {
        id: 5,
        kind: "sdtm",
        name: "2024-06-28",
        createdAt: "",
        updatedAt: "",
      },
    ],
    list_code_lists: () => ({
      items: [
        {
          id: 11,
          versionId: 5,
          code: "C66741",
          extensible: true,
          name: "VSTESTCD",
          submissionValue: "VSTESTCD",
          synonym: "",
          definition: "",
          nciPreferredTerm: "",
          createdAt: "",
          updatedAt: "",
        },
      ],
    }),
    list_code_items: () => ({
      items: [
        {
          id: 21,
          codelistId: 11,
          versionId: 5,
          code: "C174106",
          submissionValue: "PASS",
          synonym: "",
          definition: "",
          nciPreferredTerm: "",
          createdAt: "",
          updatedAt: "",
        },
      ],
    }),
```

Then add:

```tsx
  it("opens the terminology drawer with the code items of the clicked code", async () => {
    renderPage();
    await screen.findByText("AESEV");
    await userEvent.click(screen.getByRole("button", { name: "VSTESTCD" }));
    expect(await screen.findByText("PASS")).toBeInTheDocument();
  });
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `pnpm --filter aegis-desktop test -- src/test/features/domain-model/sdtm-domain-detail.test.tsx`
Expected: FAIL — there is no button named "VSTESTCD" (no Terminology column wired to the page).

- [ ] **Step 3: Wire the drawer into the page**

In `SdtmDomainDetail.tsx`:

1. Import the drawer through the terminology barrel:
   `import { TerminologyCodeItemsDrawer } from "../../terminology";`
2. Add the state next to the other drawer state:
   `const [terminologyCode, setTerminologyCode] = useState<string | null>(null);`
3. Pass the handler to the table:
   `onTerminologyClick={(code) => setTerminologyCode(code)}`
4. Render the drawer next to `<VariableEditDrawer>`:
   ```tsx
   <TerminologyCodeItemsDrawer
     open={terminologyCode !== null}
     code={terminologyCode}
     onClose={() => setTerminologyCode(null)}
   />
   ```

- [ ] **Step 4: Run the test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- src/test/features/domain-model/sdtm-domain-detail.test.tsx`
Expected: PASS.

- [ ] **Step 5: Run the full gate**

Run:
```bash
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
pnpm --filter @aegis/ui test
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop
```
Expected: everything green.

- [ ] **Step 6: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/domain-model/pages/SdtmDomainDetail.tsx \
        apps/desktop/aegis-desktop/src/test/features/domain-model/sdtm-domain-detail.test.tsx
git commit -m "feat(aegis-desktop): open the terminology drawer from the variable table

Clicking a terminology code in the variable table opens a read-only
drawer with the code items of the matching code list."
```

---

## Implementation notes (deviations from the plan above)

Recorded after execution and the final review, so a reader comparing
this plan to the code is not left guessing.

1. **Task 1, Step 3 — the update field needs both serde attributes.**
   The step showed only `deserialize_with = "double_option"`. The field
   must also keep `skip_serializing_if = "Option::is_none"`: without it
   the outer `None` serializes as `null`, so every update would clear
   the column. The pre-existing test `update_request_skips_none_fields`
   caught it. The two attributes are independent — one governs
   serialization, the other deserialization.

2. **Task 4 — `useLatestSdtmTerminologyVersionId` returns an object,
   not `number | null`.** A bare id collapsed three states into one:
   "still loading", "the request failed" and "none imported". A failed
   versions query was reported as "No SDTM terminology version has been
   imported" — a confidently wrong message with no error and no Retry.
   The hook now returns `{ versionId, isLoading, error, refetch }` and
   the drawer renders the three states separately.

3. **Task 4 — Retry refetches only the query that failed.** The code-item
   query is disabled until a code list resolves, but an explicit
   `refetch()` bypasses `enabled` — so Retry asked the server for an
   arbitrary, unfiltered page of code items.

4. **Task 4 — the not-found wording and the edit drawer's hint changed.**
   The drawer only searches code list *names*; `code_lists.code` is not
   in the full-text index and no endpoint looks a codelist up by its
   code. The message and the helper text now say so, instead of
   claiming the code list does not exist.

5. **Task 3/4 — the spec's `terminology.codeitem.drawer.openTooltip`
   key, which the plan's task text dropped.** The chips now carry it
   as a tooltip and an `aria-label`, so the button is not announced as
   a bare code.
