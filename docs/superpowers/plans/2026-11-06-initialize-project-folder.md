# Initialize Project Folder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Tauri command that materializes a structure template into a real project folder on the desktop machine's disk, and a dialog that runs it and reports what happened.

**Architecture:** A new workspace crate `lib/crates/scaffolding` holds the pure planning — context derivation, `{{ … }}` template resolution, and the tree → scaffold transformation — with no filesystem I/O. The desktop crate (`src-tauri/src/scaffold/`) holds the applier that executes a scaffold against the real disk and produces the report. The Tauri command `initialize_project_folder` fetches the structure over HTTP, builds the context, plans, and applies.

**Tech Stack:** Rust 2024 (workspace `edition = "2024"`, `resolver = "3"`), `chrono` for the date, `serde` for the report types, Tauri v2 commands, React + TanStack Query + MUI for the frontend, Vitest + Testing Library for the TS tests.

**Spec:** [`docs/superpowers/specs/2026-11-06-initialize-project-folder-design.md`](../specs/2026-11-06-initialize-project-folder-design.md) — the plan argues from the spec, so the spec travels with it; executors read both.

## Global Constraints

These apply to every task below. They are copied verbatim from the spec.

- **`rootpath` is the shared base.** It is used verbatim — the `{{ … }}` syntax applies to `entry_name` only, never to `rootpath`.
- **The per-project path comes from the tree**, not from the materializer. The template's own `entry_name`s carry `{{ product | lowercase }}` → `{{ trial }}`.
- **Context fields** are exactly `study`, `product`, `trial`, `branch`, `date`. `product` and `trial` are derived from `study` by splitting on `-`; `trial` is absent when the code has no second segment. `branch` is optional. `date` is `yyyy-mm-dd`.
- **Template operations** are exactly `lowercase` and `uppercase`, written `{{ field \| op }}`. Whitespace inside the braces is insignificant. Any number of templates may appear in one `entry_name`.
- **Unknown field, unknown operation, malformed template** → the node is **skipped**, never an error. An unresolved field in a directory skips the whole subtree.
- **A resolved name that is empty or contains a path separator** (`/`, `\`), or is `.` / `..` → the node is **skipped** with reason `InvalidName`. Never sanitise it, never create nested directories. This is what stops a branch named `../../Windows` from escaping `rootpath`.
- **Migration filter matching** is: strip a leading `*.` if present, then match the remainder as a **case-insensitive suffix** of the filename. Only a *leading* `*.` is recognised — `sas*` matches nothing.
- **Migration copy is top-level only.** Subdirectories of `migration.directory` are not recursed into.
- **Existing targets are never overwritten.** An existing directory is skipped but its children are still walked, so missing descendants are still created. An existing file is skipped.
- **Copied files keep their source modification time.**
- **Relative `reference` / `migration.directory` paths** resolve against `rootpath` rather than being rejected.
- **The scaffold carries its own skips** — `Scaffold { root, steps, skipped }`. Plan-time skips and apply-time skips share one `SkipReason` enum, which lives in the `scaffolding` crate.
- **`ScaffoldError` has exactly one variant**, `EmptyRootpath`. Every other name-level problem is a skip.
- **The command's invoke keys** are `structureId`, `study`, `branch` — added to **both** invoke-key lists (the Rust one in `commands/structures.rs` and the TS one in `src/test/features/templates/api.test.ts`).
- **Report types** (`InitReport`, `SkippedItem`, `SkipReason`) live in the desktop's `scaffold` module, NOT in `http/structures.rs` — `http/` mirrors the server and this report has no server counterpart.
- **`SkipReason` serializes as camelCase** so the TypeScript union matches.
- **Shared Rust deps** come from the root `Cargo.toml` via `{ workspace = true }`. Anything not there is declared locally with a comment explaining why.
- **Local-only deps need a one-line `why` comment** in `Cargo.toml`.
- **Wire DTOs are duplicated by hand**, not generated: `src/shared/api/types.ts` mirrors the Rust DTOs 1:1. TS identifiers camelCase, wire snake_case, renamed at the serde boundary. Any wire-shape change updates both halves in the same commit.
- **Frontend state lives in TanStack Query**; query keys come from the factory in `src/shared/query/keys.ts`, never inline arrays.
- **The default `QueryClient` is non-retrying** — `staleTime: Infinity`, `retry: false`. A mutation's `retry: false` is the default.
- **Cross-feature imports go through the feature's `index.ts` barrel**, never a deep path.

---

## File Structure

**New crate — `lib/crates/scaffolding/`** (pure planning, no I/O):

| File | Responsibility |
| --- | --- |
| `Cargo.toml` | Package manifest; deps `chrono`, `serde`, `thiserror` (all workspace) |
| `src/lib.rs` | Crate doc + `pub mod` declarations + root re-exports |
| `src/context.rs` | `Context` + `Context::new` (derives `product`/`trial`, formats `date`) |
| `src/template.rs` | The `{{ … }}` parser and resolver; `Field`, `Operation` |
| `src/scaffold.rs` | `StructureNode`, `Migration`, `Scaffold`, `Step`, `scaffold::build` |
| `src/report.rs` | `SkipReason`, `SkippedItem` (serde, camelCase) |
| `src/error.rs` | `ScaffoldError` |
| `README.md` | What the crate is, what it deliberately does not do |

**Desktop — `apps/desktop/aegis-desktop/src-tauri/src/`:**

| File | Responsibility |
| --- | --- |
| `scaffold.rs` (new) | `pub mod apply;` `pub mod report;` barrel |
| `scaffold/apply.rs` (new) | The applier: executes a `Scaffold` against the real disk |
| `scaffold/report.rs` (new) | `InitReportResponse` + the wire DTO mirror of `SkippedItem` |
| `commands/scaffold.rs` (new) | The `initialize_project_folder` Tauri command |
| `commands.rs` (modify) | Add `pub mod scaffold;` |
| `lib.rs` (modify) | Register `commands::scaffold::initialize_project_folder` |
| `http/structures.rs` (modify) | Unchanged in shape; the command reuses `get_structure` |

**Frontend — `apps/desktop/aegis-desktop/src/`:**

| File | Responsibility |
| --- | --- |
| `shared/api/types.ts` (modify) | Add `InitReport`, `SkippedItem`, `SkipReason` TS mirrors |
| `shared/api/index.ts` (modify) | Add `api.initializeProjectFolder` |
| `features/project-workspace/components/InitializeProjectFolderDialog.tsx` (modify) | Call the command, render the report |
| `features/project-workspace/components/ProjectBranchesSection.tsx` (modify) | Pass `projectCode`, handle the new `onInitialized` |
| `features/project-workspace/data/project-branches.ts` (modify) | Add `useInitializeProjectFolder` mutation hook |
| `features/project-workspace/index.ts` (modify) | Export the new hook |

**i18n — `lib/packages/ui/src/i18n/locales/`:**

| File | Responsibility |
| --- | --- |
| `en.ts` (modify) | New `project.branches.initialize.*` keys |
| `zhCN.ts` (modify) | The same keys, translated |

**Tests:**

| File | Responsibility |
| --- | --- |
| `lib/crates/scaffolding/src/*` (inline `#[cfg(test)]`) | Unit tests, no disk |
| `apps/desktop/aegis-desktop/src-tauri/src/scaffold/apply.rs` (inline `#[cfg(test)]`) | Applier tests against a temp dir |
| `apps/desktop/aegis-desktop/src-tauri/src/commands/structures.rs` (modify) | Add the new command to `EXPECTED_INVOKE_KEYS` |
| `apps/desktop/aegis-desktop/src/test/features/templates/api.test.ts` (modify) | Add the new command to `EXPECTED_INVOKE_KEYS` |
| `apps/desktop/aegis-desktop/src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx` (modify) | Dialog calls the command; renders the report |
| `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-branches-section.test.tsx` (modify) | The section threads `projectCode` through |

---

## Review Focus

The spec is a vision document: it says what the software must do, not everything it will meet. These are the five input classes or failure modes it implies but no listed requirement names outright — each gets a test in the task that owns the code.

1. **A branch name containing `/` (e.g. `feat/login`).** The server only rejects empty branch names, so this is reachable. The resolved `entry_name` must be skipped as `InvalidName`, and nothing may be written outside `rootpath`. → Task 3.
2. **A study code with no `-` (e.g. `AK001`).** There is no second segment, so `{{ trial }}` has no value. The node using it is skipped, and — critically — so is its whole subtree, leaving no orphaned children. → Task 3.
3. **Re-running the command against an already-generated folder.** Nothing is overwritten, nothing is duplicated, and only the genuinely missing pieces are created. → Task 4.
4. **A migration whose filter is `*.sas7bdat` (the shape the repo's existing data uses).** It must match `a.sas7bdat` — a literal-suffix reading would silently copy *zero* files, which looks like success. → Task 4.
5. **A structure whose `rootpath` is empty** (structures created before the `rootpath` column existed read back empty). This must be a clear hard error, not a silent no-op or a relative-path write. → Task 3.

---

### Task 1: The `scaffolding` crate — context and template resolution

The crate's foundation: the `Context` type and the `{{ … }}` parser. Pure, no I/O, and the two things the rest of the feature is built on.

**Files:**
- Create: `lib/crates/scaffolding/Cargo.toml`
- Create: `lib/crates/scaffolding/src/lib.rs`
- Create: `lib/crates/scaffolding/src/context.rs`
- Create: `lib/crates/scaffolding/src/template.rs`
- Create: `lib/crates/scaffolding/src/error.rs`
- Create: `lib/crates/scaffolding/README.md`
- Modify: `Cargo.toml` (workspace root) — add the member

**Interfaces:**
- Consumes: nothing (first task).
- Produces:
  - `scaffolding::Context { pub study: String, pub product: Option<String>, pub trial: Option<String>, pub branch: Option<String>, pub date: String }`
  - `scaffolding::Context::new(study: &str, branch: Option<&str>, date: chrono::NaiveDate) -> Context`
  - `scaffolding::template::Field` — enum with variants `Study`, `Product`, `Trial`, `Branch`, `Date`
  - `scaffolding::template::Operation` — enum with variants `Lowercase`, `Uppercase`
  - `scaffolding::template::TemplateError` — enum with variants `UnknownField(String)`, `UnknownOperation(String)`, `Malformed(String)`
  - `scaffolding::template::resolve(entry_name: &str, ctx: &Context) -> Result<String, TemplateError>`
  - `scaffolding::ScaffoldError` — enum with the single variant `EmptyRootpath`

- [ ] **Step 1: Register the crate in the workspace**

Edit the root `Cargo.toml` `[workspace].members` array — add `"lib/crates/scaffolding",` after `"lib/crates/project",` (keep the array's existing loose grouping; the members list is not alphabetised).

- [ ] **Step 2: Create the crate manifest**

Create `lib/crates/scaffolding/Cargo.toml`:

```toml
[package]
name = "scaffolding"
version = "0.1.0"
edition = "2024"

[dependencies]
# `chrono` supplies `NaiveDate` — the context's `date` field is the
# desktop machine's current date, formatted `yyyy-mm-dd`.
chrono = { workspace = true }
# `serde` derives the report types (`SkipReason`, `SkippedItem`),
# which cross the Tauri boundary to the frontend.
serde = { workspace = true }
# `thiserror` derives `ScaffoldError` and `TemplateError`.
thiserror = { workspace = true }
```

- [ ] **Step 3: Write the failing test for `Context::new`**

Create `lib/crates/scaffolding/src/context.rs` with the type and the tests. Start with the tests only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn derives_product_and_trial_from_a_two_part_study_code() {
        let ctx = Context::new("AK001-001", None, date(2026, 11, 6));
        assert_eq!(ctx.study, "AK001-001");
        assert_eq!(ctx.product.as_deref(), Some("AK001"));
        assert_eq!(ctx.trial.as_deref(), Some("001"));
    }

    #[test]
    fn a_study_code_without_a_dash_has_no_trial() {
        let ctx = Context::new("AK001", None, date(2026, 11, 6));
        assert_eq!(ctx.product.as_deref(), Some("AK001"));
        assert_eq!(ctx.trial, None);
    }

    #[test]
    fn keeps_the_branch_when_given_and_omits_it_otherwise() {
        let with = Context::new("AK001-001", Some("2026Q1"), date(2026, 11, 6));
        assert_eq!(with.branch.as_deref(), Some("2026Q1"));
        let without = Context::new("AK001-001", None, date(2026, 11, 6));
        assert_eq!(without.branch, None);
    }

    #[test]
    fn formats_the_date_as_year_month_day() {
        let ctx = Context::new("AK001-001", None, date(2026, 11, 6));
        assert_eq!(ctx.date, "2026-11-06");
    }
}
```

- [ ] **Step 4: Run the test to verify it fails**

Run: `cargo test -p scaffolding`
Expected: FAIL — the crate does not exist yet, or `Context` is not defined.

- [ ] **Step 5: Implement `Context`**

In `lib/crates/scaffolding/src/context.rs`, above the test module:

```rust
use chrono::NaiveDate;

/// The values a template can interpolate.
///
/// `product` and `trial` are derived from `study` and are absent when
/// the study code does not carry them — a code with no `-` has no
/// `trial`, and a template using `{{ trial }}` is then unresolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub study: String,
    pub product: Option<String>,
    pub trial: Option<String>,
    pub branch: Option<String>,
    /// The current date, `yyyy-mm-dd`.
    pub date: String,
}

impl Context {
    /// Build the context, deriving `product` and `trial` from `study`
    /// and formatting `date`.
    pub fn new(study: &str, branch: Option<&str>, date: NaiveDate) -> Context {
        let mut parts = study.split('-');
        let product = parts.next().filter(|p| !p.is_empty()).map(str::to_string);
        let trial = parts.next().filter(|p| !p.is_empty()).map(str::to_string);
        Context {
            study: study.to_string(),
            product,
            trial,
            branch: branch.map(str::to_string),
            date: date.format("%Y-%m-%d").to_string(),
        }
    }
}
```

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test -p scaffolding`
Expected: PASS — 4 tests.

- [ ] **Step 7: Write the failing test for template resolution**

Create `lib/crates/scaffolding/src/template.rs` with tests only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use chrono::NaiveDate;

    fn ctx() -> Context {
        Context::new(
            "AK001-001",
            Some("2026Q1"),
            NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
        )
    }

    #[test]
    fn resolves_a_bare_field() {
        assert_eq!(resolve("{{ study }}", &ctx()).unwrap(), "AK001-001");
    }

    #[test]
    fn applies_the_lowercase_operation() {
        assert_eq!(resolve("{{ product | lowercase }}", &ctx()).unwrap(), "ak001");
    }

    #[test]
    fn applies_the_uppercase_operation() {
        assert_eq!(resolve("{{ branch | uppercase }}", &ctx()).unwrap(), "2026Q1");
    }

    #[test]
    fn resolves_several_templates_in_one_name() {
        assert_eq!(
            resolve("{{ product }}-{{ trial }}", &ctx()).unwrap(),
            "ak001-001"
        );
    }

    #[test]
    fn ignores_whitespace_inside_the_braces() {
        assert_eq!(resolve("{{study}}", &ctx()).unwrap(), "AK001-001");
        assert_eq!(resolve("{{   product   }}", &ctx()).unwrap(), "AK001");
    }

    #[test]
    fn leaves_text_outside_the_templates_alone() {
        assert_eq!(resolve("raw_{{ trial }}_data", &ctx()).unwrap(), "raw_001_data");
    }

    #[test]
    fn an_unknown_field_is_an_error() {
        let err = resolve("{{ prodcut }}", &ctx()).unwrap_err();
        assert!(matches!(err, TemplateError::UnknownField(f) if f == "prodcut"));
    }

    #[test]
    fn an_unknown_operation_is_an_error() {
        let err = resolve("{{ product | shout }}", &ctx()).unwrap_err();
        assert!(matches!(err, TemplateError::UnknownOperation(o) if o == "shout"));
    }

    #[test]
    fn an_unclosed_brace_is_malformed() {
        let err = resolve("{{ study", &ctx()).unwrap_err();
        assert!(matches!(err, TemplateError::Malformed(_)));
    }

    #[test]
    fn a_missing_optional_field_is_an_error() {
        let no_branch = Context::new(
            "AK001-001",
            None,
            NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
        );
        let err = resolve("{{ branch }}", &no_branch).unwrap_err();
        assert!(matches!(err, TemplateError::UnknownField(f) if f == "branch"));
    }

    #[test]
    fn a_name_without_templates_is_returned_unchanged() {
        assert_eq!(resolve("01_raw", &ctx()).unwrap(), "01_raw");
    }
}
```

- [ ] **Step 8: Run the test to verify it fails**

Run: `cargo test -p scaffolding`
Expected: FAIL — `resolve` and `TemplateError` are not defined.

- [ ] **Step 9: Implement the template resolver**

In `lib/crates/scaffolding/src/template.rs`, above the test module:

```rust
use crate::context::Context;

/// A field a template can interpolate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Study,
    Product,
    Trial,
    Branch,
    Date,
}

/// The transformation applied to a field's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Lowercase,
    Uppercase,
}

/// Why a template could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateError {
    #[error("unknown template field: {0}")]
    UnknownField(String),
    #[error("unknown template operation: {0}")]
    UnknownOperation(String),
    #[error("malformed template: {0}")]
    Malformed(String),
}
```

Then `resolve`:

```rust
/// Resolve every `{{ … }}` template in `entry_name` against `ctx`.
///
/// A name with no templates is returned unchanged. The first failure
/// is returned as an error — the caller decides whether that is a skip
/// or a hard failure.
pub fn resolve(entry_name: &str, ctx: &Context) -> Result<String, TemplateError> {
    let mut out = String::with_capacity(entry_name.len());
    let mut rest = entry_name;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("}}").ok_or_else(|| {
            TemplateError::Malformed(format!("unclosed '{{{{' in {entry_name:?}"))
        })?;
        let body = &after[..end];
        out.push_str(&render(body, ctx)?);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}
```

And `render`, which parses one template body and applies it:

```rust
/// Render one template body — the text between `{{` and `}}`.
fn render(body: &str, ctx: &Context) -> Result<String, TemplateError> {
    let mut parts = body.split('|');
    let name = parts.next().unwrap_or("").trim();
    let operation = match parts.next() {
        Some(op) => Some(op.trim()),
        None => None,
    };
    if parts.next().is_some() {
        return Err(TemplateError::Malformed(format!("too many parts in {body:?}")));
    }
    let value = match name {
        "study" => Some(ctx.study.clone()),
        "product" => ctx.product.clone(),
        "trial" => ctx.trial.clone(),
        "branch" => ctx.branch.clone(),
        "date" => Some(ctx.date.clone()),
        other => return Err(TemplateError::UnknownField(other.to_string())),
    };
    let value = value.ok_or_else(|| TemplateError::UnknownField(name.to_string()))?;
    Ok(match operation {
        None => value,
        Some("lowercase") => value.to_lowercase(),
        Some("uppercase") => value.to_uppercase(),
        Some(other) => return Err(TemplateError::UnknownOperation(other.to_string())),
    })
}
```

**Note on `Field` and `Operation`:** they are the public vocabulary of the template syntax. If the implementer prefers to skip them (they are not used by the resolver above, which matches on strings), they may be removed — but then `Field`/`Operation` must not appear in the crate's public re-exports.

- [ ] **Step 10: Run the test to verify it passes**

Run: `cargo test -p scaffolding`
Expected: PASS — 15 tests (4 context + 11 template).

- [ ] **Step 11: Write the failing test for `ScaffoldError`**

Create `lib/crates/scaffolding/src/error.rs`:

```rust
/// Why a scaffold could not be built at all.
///
/// Every *name*-level problem — an unresolved field, a malformed
/// template, an invalid resolved name — is a skip, not an error. The
/// only hard failure is an empty `rootpath`: there is nothing to
/// scaffold relative to.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScaffoldError {
    #[error("the structure has no root path")]
    EmptyRootpath,
}
```

No test is needed for a single-variant enum with no logic; it is covered by Task 3's `EmptyRootpath` test.

- [ ] **Step 12: Create the crate root and README**

Create `lib/crates/scaffolding/src/lib.rs`:

```rust
//! `scaffolding` workspace crate.
//!
//! Turns a structure template into the list of filesystem operations
//! that materialises it. The crate performs **no** I/O: it resolves
//! `{{ … }}` templates in a node's `entry_name` against a [`Context`]
//! and produces a [`Scaffold`] — the ordered steps an applier would
//! execute, plus the nodes that could not be planned.
//!
//! The applier lives in the desktop app, which owns the filesystem.
//!
//! ```ignore
//! use scaffolding::{Context, scaffold};
//! # fn demo(ctx: Context, node: scaffolding::StructureNode) {
//! let plan = scaffold::build("D:\\studies", &node, &ctx)?;
//! # Ok::<(), scaffolding::ScaffoldError>(())
//! # }
//! ```

pub mod context;
pub mod error;
pub mod report;
pub mod scaffold;
pub mod template;

pub use context::Context;
pub use error::ScaffoldError;
pub use report::{SkipReason, SkippedItem};
pub use scaffold::{Migration, Scaffold, Step, StructureNode, build};
pub use template::{Field, Operation, TemplateError, resolve};
```

Create `lib/crates/scaffolding/README.md` — a short file covering: what the crate does (pure planning, no I/O), the crate layout, the template syntax with an example, and the deliberate non-goals (no filesystem access, no server types, no dependency on `structures`).

- [ ] **Step 13: Verify the crate compiles**

Run: `cargo build -p scaffolding`
Expected: it fails — `report.rs` and `scaffold.rs` do not exist yet. Create them as **empty placeholder modules** so the crate compiles:

`lib/crates/scaffolding/src/report.rs`:
```rust
// Filled in by Task 2.
```

`lib/crates/scaffolding/src/scaffold.rs`:
```rust
// Filled in by Task 2.
```

Then re-run: `cargo build -p scaffolding`
Expected: PASS. The `pub use scaffold::{…}` re-export in `lib.rs` will fail to compile against the placeholder — so in this step, comment out the `pub use scaffold::…` and `pub use report::…` lines in `lib.rs`, and restore them in Task 2.

- [ ] **Step 14: Run the full test suite and commit**

Run: `cargo test -p scaffolding && cargo fmt --all && cargo clippy -p scaffolding --all-targets -- -D warnings`
Expected: PASS with no warnings.

```bash
git add lib/crates/scaffolding Cargo.toml
git commit -m "feat(scaffolding): add context and template resolution

Introduces the scaffolding crate: the Context type (deriving product
and trial from the study code) and the {{ ... }} template resolver.

Co-Authored-By: Claude Code <noreply@anthropic.com>
```

---

### Task 2: The `scaffolding` crate — the plan

Turn a tree plus a context into a `Scaffold`: the ordered steps and the plan-time skips.

**Files:**
- Modify: `lib/crates/scaffolding/src/scaffold.rs` (replace the placeholder)
- Modify: `lib/crates/scaffolding/src/report.rs` (replace the placeholder)
- Modify: `lib/crates/scaffolding/src/lib.rs` (restore the re-exports)

**Interfaces:**
- Consumes: `scaffolding::Context` and `scaffolding::template::resolve` from Task 1.
- Produces:
  - `scaffolding::StructureNode { pub entry_name: String, pub directory: bool, pub reference: Option<String>, pub migration: Option<Migration>, pub children: Vec<StructureNode> }`
  - `scaffolding::Migration { pub directory: String, pub filter: Vec<String> }`
  - `scaffolding::Scaffold { pub root: PathBuf, pub steps: Vec<Step>, pub skipped: Vec<SkippedItem> }`
  - `scaffolding::Step` — enum with `MakeDir { path: PathBuf }`, `CopyFile { from: PathBuf, to: PathBuf }`, `CopyDirFiles { from_dir: PathBuf, to_dir: PathBuf, filter: Vec<String> }`
  - `scaffolding::SkipReason` — enum with `AlreadyExists`, `UnresolvedField`, `MalformedTemplate`, `InvalidName`, `NoReference`, `ReferenceFailed`, `MigrationFailed`; serde `rename_all = "camelCase"`
  - `scaffolding::SkippedItem { pub path: String, pub reason: SkipReason, pub detail: Option<String> }`; serde `rename_all = "camelCase"`
  - `scaffolding::scaffold::build(rootpath: &str, nodes: &[StructureNode], ctx: &Context) -> Result<Scaffold, ScaffoldError>`

- [ ] **Step 1: Write the failing test for the report types**

Create `lib/crates/scaffolding/src/report.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Why a node was not created.
///
/// Shared between the planner (which skips a node it cannot resolve)
/// and the applier (which skips a node whose target already exists),
/// so the two never drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkipReason {
    /// The target already exists on disk.
    AlreadyExists,
    /// The `entry_name` referenced a field the context did not have.
    UnresolvedField,
    /// The `entry_name` did not parse as a template.
    MalformedTemplate,
    /// The resolved name was empty or contained a path separator.
    InvalidName,
    /// A file node with no `reference` — nothing to copy.
    NoReference,
    /// The `reference` file could not be read or copied.
    ReferenceFailed,
    /// The `migration.directory` could not be read.
    MigrationFailed,
}

/// One node that was not created, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedItem {
    /// The path the node would have had. For a node whose name could
    /// not be resolved this is the parent's path joined with the raw,
    /// unresolved `entry_name` — it is not a real directory.
    pub path: String,
    pub reason: SkipReason,
    /// The missing field name, the raw template text, or the
    /// underlying I/O error message.
    pub detail: Option<String>,
}
```

- [ ] **Step 2: Write the failing test for `scaffold::build`**

Create `lib/crates/scaffolding/src/scaffold.rs` with the types and the tests. The test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use chrono::NaiveDate;

    fn ctx() -> Context {
        Context::new(
            "AK001-001",
            Some("2026Q1"),
            NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
        )
    }

    fn dir(entry_name: &str, children: Vec<StructureNode>) -> StructureNode {
        StructureNode {
            entry_name: entry_name.to_string(),
            directory: true,
            reference: None,
            migration: None,
            children,
        }
    }

    fn file(entry_name: &str, reference: Option<&str>) -> StructureNode {
        StructureNode {
            entry_name: entry_name.to_string(),
            directory: false,
            reference: reference.map(str::to_string),
            migration: None,
            children: Vec::new(),
        }
    }

    #[test]
    fn an_empty_rootpath_is_the_one_hard_error() {
        let err = build("", &[dir("01_raw", vec![])], &ctx()).unwrap_err();
        assert_eq!(err, ScaffoldError::EmptyRootpath);
    }

    #[test]
    fn resolves_the_tree_into_ordered_steps() {
        let tree = vec![dir(
            "{{ product | lowercase }}",
            vec![dir("{{ trial }}", vec![dir("01_raw", vec![])])],
        )];
        let plan = build("D:\\studies", &tree, &ctx()).unwrap();
        assert_eq!(plan.root, PathBuf::from("D:\\studies"));
        let paths: Vec<_> = plan
            .steps
            .iter()
            .map(|s| match s {
                Step::MakeDir { path } => path.clone(),
                _ => panic!("expected MakeDir"),
            })
            .collect();
        assert_eq!(
            paths,
            vec![
                PathBuf::from("D:\\studies\\ak001"),
                PathBuf::from("D:\\studies\\ak001\\001"),
                PathBuf::from("D:\\studies\\ak001\\001\\01_raw"),
            ]
        );
    }

    #[test]
    fn an_unresolved_field_skips_the_node_and_its_subtree() {
        let tree = vec![dir(
            "{{ branch }}",
            vec![dir("01_raw", vec![dir("02_clean", vec![])])],
        )];
        let plan = build("D:\\studies", &tree, &ctx()).unwrap();
        assert!(plan.steps.is_empty());
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].reason, SkipReason::UnresolvedField);
        assert_eq!(plan.skipped[0].detail.as_deref(), Some("branch"));
    }

    #[test]
    fn a_missing_trial_skips_only_the_node_that_needs_it() {
        let no_trial = Context::new(
            "AK001",
            Some("2026Q1"),
            NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
        );
        let tree = vec![dir(
            "{{ product }}",
            vec![dir("{{ trial }}", vec![]), dir("01_raw", vec![])],
        )];
        let plan = build("D:\\studies", &tree, &no_trial).unwrap();
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].reason, SkipReason::UnresolvedField);
        assert_eq!(plan.skipped[0].detail.as_deref(), Some("trial"));
        assert_eq!(plan.steps.len(), 2);
    }

    #[test]
    fn a_malformed_template_is_skipped() {
        let tree = vec![dir("{{ study", vec![])];
        let plan = build("D:\\studies", &tree, &ctx()).unwrap();
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].reason, SkipReason::MalformedTemplate);
    }

    #[test]
    fn a_resolved_name_with_a_path_separator_is_skipped() {
        let slashed = Context::new(
            "AK001-001",
            Some("feat/login"),
            NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
        );
        let tree = vec![dir("{{ branch }}", vec![])];
        let plan = build("D:\\studies", &tree, &slashed).unwrap();
        assert!(plan.steps.is_empty());
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].reason, SkipReason::InvalidName);
    }

    #[test]
    fn a_dot_dot_resolved_name_is_skipped() {
        let tree = vec![dir("..", vec![])];
        let plan = build("D:\\studies", &tree, &ctx()).unwrap();
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].reason, SkipReason::InvalidName);
    }

    #[test]
    fn a_file_node_with_no_reference_is_skipped() {
        let tree = vec![file("spec.docx", None)];
        let plan = build("D:\\studies", &tree, &ctx()).unwrap();
        assert!(plan.steps.is_empty());
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].reason, SkipReason::NoReference);
    }

    #[test]
    fn a_file_node_with_a_reference_becomes_a_copy_step() {
        let tree = vec![file("{{ trial }}.docx", Some("D:\\sop\\spec.docx"))];
        let plan = build("D:\\studies", &tree, &ctx()).unwrap();
        assert_eq!(
            plan.steps,
            vec![Step::CopyFile {
                from: PathBuf::from("D:\\sop\\spec.docx"),
                to: PathBuf::from("D:\\studies\\001.docx"),
            }]
        );
    }

    #[test]
    fn a_relative_reference_resolves_against_the_rootpath() {
        let tree = vec![file("spec.docx", Some("sop\\spec.docx"))];
        let plan = build("D:\\studies", &tree, &ctx()).unwrap();
        assert_eq!(
            plan.steps,
            vec![Step::CopyFile {
                from: PathBuf::from("D:\\studies\\sop\\spec.docx"),
                to: PathBuf::from("D:\\studies\\spec.docx"),
            }]
        );
    }

    #[test]
    fn a_directory_with_a_migration_becomes_a_copy_dir_files_step() {
        let mut node = dir("01_raw", vec![]);
        node.migration = Some(Migration {
            directory: "D:\\sop\\raw".to_string(),
            filter: vec!["*.sas7bdat".to_string()],
        });
        let tree = vec![node];
        let plan = build("D:\\studies", &tree, &ctx()).unwrap();
        assert_eq!(
            plan.steps,
            vec![
                Step::MakeDir {
                    path: PathBuf::from("D:\\studies\\01_raw")
                },
                Step::CopyDirFiles {
                    from_dir: PathBuf::from("D:\\sop\\raw"),
                    to_dir: PathBuf::from("D:\\studies\\01_raw"),
                    filter: vec!["*.sas7bdat".to_string()],
                },
            ]
        );
    }

    #[test]
    fn a_skipped_node_reports_its_parent_path_with_the_raw_name() {
        let tree = vec![dir("01_raw", vec![file("{{ branch }}.docx", None)])];
        let plan = build("D:\\studies", &tree, &ctx()).unwrap();
        assert_eq!(
            plan.skipped[0].path,
            "D:\\studies\\01_raw\\{{ branch }}.docx"
        );
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p scaffolding`
Expected: FAIL — `build`, `StructureNode`, `Migration`, `Scaffold`, `Step` are not defined.

- [ ] **Step 4: Implement `scaffold::build`**

In `lib/crates/scaffolding/src/scaffold.rs`, above the test module:

```rust
use std::path::{Path, PathBuf};

use crate::context::Context;
use crate::error::ScaffoldError;
use crate::report::{SkipReason, SkippedItem};
use crate::template::{TemplateError, resolve};

/// One node of the tree to scaffold.
///
/// The desktop maps the server's wire DTO onto this — the crate does
/// not depend on `structures` or on the desktop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureNode {
    /// The node's own name, possibly carrying `{{ … }}` templates.
    pub entry_name: String,
    pub directory: bool,
    /// The file to copy for a file node.
    pub reference: Option<String>,
    /// What to copy into a directory node.
    pub migration: Option<Migration>,
    pub children: Vec<StructureNode>,
}

/// What to copy into a directory once it has been created.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub directory: String,
    /// Filename patterns — see `Step::CopyDirFiles`.
    pub filter: Vec<String>,
}

/// The filesystem work, in application order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scaffold {
    /// The base the tree is written under.
    pub root: PathBuf,
    /// The steps, parents before children.
    pub steps: Vec<Step>,
    /// The nodes that could not be planned, with the reason.
    pub skipped: Vec<SkippedItem>,
}

/// One filesystem operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Create this directory.
    MakeDir { path: PathBuf },
    /// Copy `from` to `to`.
    CopyFile { from: PathBuf, to: PathBuf },
    /// Copy the top-level files of `from_dir` into `to_dir`.
    CopyDirFiles {
        from_dir: PathBuf,
        to_dir: PathBuf,
        filter: Vec<String>,
    },
}
```

Then the builder. The recursion walks the tree, resolving each name; a name it cannot resolve (or cannot use) skips the node and its whole subtree:

```rust
/// Plan the filesystem work for a structure's tree.
///
/// Pure: no I/O, and no knowledge of whether anything already exists
/// on disk. Parents are emitted before their children.
pub fn build(
    rootpath: &str,
    nodes: &[StructureNode],
    ctx: &Context,
) -> Result<Scaffold, ScaffoldError> {
    if rootpath.trim().is_empty() {
        return Err(ScaffoldError::EmptyRootpath);
    }
    let root = PathBuf::from(rootpath);
    let mut steps = Vec::new();
    let mut skipped = Vec::new();
    for node in nodes {
        walk(node, &root, &root, ctx, &mut steps, &mut skipped);
    }
    Ok(Scaffold {
        root,
        steps,
        skipped,
    })
}

/// Resolve one node's name and emit its steps, recursing into its
/// children under the resolved path.
///
/// `parent` is the directory the node sits in; `root` is the
/// structure's `rootpath`, which is what a relative `reference` or
/// `migration.directory` resolves against.
fn walk(
    node: &StructureNode,
    parent: &Path,
    root: &Path,
    ctx: &Context,
    steps: &mut Vec<Step>,
    skipped: &mut Vec<SkippedItem>,
) {
    // The path a skipped node would have had. It joins the raw,
    // unresolved name — it is not a real directory.
    let raw_path = parent.join(&node.entry_name);

    let resolved = match resolve(&node.entry_name, ctx) {
        Ok(name) => name,
        Err(TemplateError::UnknownField(field)) => {
            skipped.push(SkippedItem {
                path: raw_path.display().to_string(),
                reason: SkipReason::UnresolvedField,
                detail: Some(field),
            });
            return;
        }
        Err(TemplateError::Malformed(text)) => {
            skipped.push(SkippedItem {
                path: raw_path.display().to_string(),
                reason: SkipReason::MalformedTemplate,
                detail: Some(text),
            });
            return;
        }
        Err(TemplateError::UnknownOperation(op)) => {
            skipped.push(SkippedItem {
                path: raw_path.display().to_string(),
                reason: SkipReason::MalformedTemplate,
                detail: Some(op),
            });
            return;
        }
    };

    if !is_valid_segment(&resolved) {
        skipped.push(SkippedItem {
            path: raw_path.display().to_string(),
            reason: SkipReason::InvalidName,
            detail: Some(resolved),
        });
        return;
    }

    let path = parent.join(&resolved);

    if node.directory {
        steps.push(Step::MakeDir { path: path.clone() });
        if let Some(migration) = &node.migration {
            steps.push(Step::CopyDirFiles {
                from_dir: resolve_against(&migration.directory, root),
                to_dir: path.clone(),
                filter: migration.filter.clone(),
            });
        }
        for child in &node.children {
            walk(child, &path, root, ctx, steps, skipped);
        }
    } else {
        match &node.reference {
            Some(reference) => steps.push(Step::CopyFile {
                from: resolve_against(reference, root),
                to: path,
            }),
            None => skipped.push(SkippedItem {
                path: path.display().to_string(),
                reason: SkipReason::NoReference,
                detail: None,
            }),
        }
    }
}
```

And the two helpers:

```rust
/// A name is usable when it is a single path segment: not blank, no
/// separator, and neither `.` nor `..`.
fn is_valid_segment(name: &str) -> bool {
    !name.trim().is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != ".."
}

/// Resolve a source path: absolute paths are used as they are, relative
/// ones resolve against the structure's `rootpath`.
fn resolve_against(path: &str, root: &Path) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}
```

**Note on `resolve_against`:** the spec says a relative `reference` or `migration.directory` resolves against `rootpath` — **not** against the node's parent directory. The `walk` recursion must therefore thread the scaffold's root through separately from the parent path it is building under, and pass the root to `resolve_against`. The test `a_relative_reference_resolves_against_the_rootpath` pins this: it asserts `D:\studies\sop\spec.docx` (the root), not the parent directory of the node.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p scaffolding`
Expected: PASS — 27 tests.

- [ ] **Step 6: Restore the crate root re-exports**

In `lib/crates/scaffolding/src/lib.rs`, uncomment the two lines commented out in Task 1 Step 13:

```rust
pub use report::{SkipReason, SkippedItem};
pub use scaffold::{Migration, Scaffold, Step, StructureNode, build};
```

Run: `cargo test -p scaffolding`
Expected: PASS.

- [ ] **Step 7: Verify the crate is clean and commit**

Run: `cargo fmt --all && cargo clippy -p scaffolding --all-targets -- -D warnings && cargo test -p scaffolding`
Expected: PASS with no warnings.

```bash
git add lib/crates/scaffolding
git commit -m "feat(scaffolding): plan a structure tree into filesystem steps

Adds scaffold::build, which resolves every node's entry_name against the
context and emits the steps to execute, plus the plan-time skips.

Co-Authored-By: Claude Code <noreply@anthropic.com>
```

---

### Task 3: The desktop applier and the Tauri command

The desktop's half: execute a scaffold against the real disk, and expose it as `initialize_project_folder`.

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/scaffold.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/scaffold/apply.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/scaffold/report.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/scaffold.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands.rs` — add `pub mod scaffold;`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` — register the command
- Modify: `apps/desktop/aegis-desktop/src-tauri/Cargo.toml` — add the `scaffolding` path dep
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/structures.rs` — add the command to `EXPECTED_INVOKE_KEYS`

**Interfaces:**
- Consumes: `scaffolding::{Context, Scaffold, Step, StructureNode, SkipReason, SkippedItem}` from Tasks 1–2; `http::structures::get_structure` from the existing desktop HTTP layer.
- Produces:
  - `aegis_desktop_lib::scaffold::report::InitReportResponse { pub root: String, pub created: Vec<String>, pub skipped: Vec<SkippedItemResponse> }`; serde `rename_all = "camelCase"`
  - `aegis_desktop_lib::scaffold::report::SkippedItemResponse { pub path: String, pub reason: SkipReason, pub detail: Option<String> }`; serde `rename_all = "camelCase"`
  - `aegis_desktop_lib::scaffold::apply::apply(scaffold: &Scaffold) -> InitReportResponse`
  - `aegis_desktop_lib::commands::scaffold::initialize_project_folder(app_state, structure_id: i64, study: String, branch: Option<String>) -> Result<InitReportResponse, ApiError>` — invoke keys `structureId`, `study`, `branch`

- [ ] **Step 1: Add the `scaffolding` dependency to the desktop**

In `apps/desktop/aegis-desktop/src-tauri/Cargo.toml`, after the `logging-utils` dependency:

```toml
# `scaffolding` holds the pure planning for the initialize-project-folder
# command — context derivation, template resolution, and the tree ->
# scaffold transformation. The desktop keeps only the filesystem half.
scaffolding = { path = "../../../../lib/crates/scaffolding" }
```

- [ ] **Step 2: Write the failing test for the applier**

Create `apps/desktop/aegis-desktop/src-tauri/src/scaffold/apply.rs` with the tests. The test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use scaffolding::{Context, Migration, Step, StructureNode};
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn ctx() -> Context {
        Context::new(
            "AK001-001",
            Some("2026Q1"),
            chrono::Local::now().date_naive(),
        )
    }

    fn dir(entry_name: &str, children: Vec<StructureNode>) -> StructureNode {
        StructureNode {
            entry_name: entry_name.to_string(),
            directory: true,
            reference: None,
            migration: None,
            children,
        }
    }

    fn file(entry_name: &str, reference: Option<&str>) -> StructureNode {
        StructureNode {
            entry_name: entry_name.to_string(),
            directory: false,
            reference: reference.map(str::to_string),
            migration: None,
            children: Vec::new(),
        }
    }

    /// Build a scaffold from a tree rooted at `root`.
    fn plan(root: &Path, nodes: &[StructureNode]) -> Scaffold {
        scaffolding::scaffold::build(&root.display().to_string(), nodes, &ctx()).unwrap()
    }

    #[test]
    fn creates_the_directory_tree() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("studies");
        let tree = vec![dir(
            "ak001",
            vec![dir("001", vec![dir("01_raw", vec![])])],
        )];
        let report = apply(&plan(&root, &tree));
        assert!(root.join("ak001").join("001").join("01_raw").is_dir());
        assert_eq!(report.created.len(), 4); // the root plus three dirs
        assert!(report.skipped.is_empty());
    }

    #[test]
    fn skips_an_existing_directory_but_still_creates_its_children() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("studies");
        fs::create_dir_all(root.join("ak001").join("001")).unwrap();
        let tree = vec![dir("ak001", vec![dir("001", vec![dir("01_raw", vec![])])])];
        let report = apply(&plan(&root, &tree));
        assert!(root.join("ak001").join("001").join("01_raw").is_dir());
        assert!(report
            .skipped
            .iter()
            .any(|s| s.reason == SkipReason::AlreadyExists));
    }

    #[test]
    fn skips_an_existing_file_without_overwriting_it() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("studies");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("spec.docx"), b"original").unwrap();
        let tree = vec![file("spec.docx", Some("source.docx"))];
        let report = apply(&plan(&root, &tree));
        assert_eq!(fs::read(root.join("spec.docx")).unwrap(), b"original");
        assert!(report
            .skipped
            .iter()
            .any(|s| s.reason == SkipReason::AlreadyExists));
    }

    #[test]
    fn copies_a_reference_file_renamed_to_the_resolved_name() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("studies");
        let source = tmp.path().join("source.docx");
        fs::write(&source, b"payload").unwrap();
        let tree = vec![file("001.docx", Some(&source.display().to_string()))];
        apply(&plan(&root, &tree));
        assert_eq!(fs::read(root.join("001.docx")).unwrap(), b"payload");
    }

    #[test]
    fn a_copied_file_keeps_its_modification_time() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("studies");
        let source = tmp.path().join("source.docx");
        fs::write(&source, b"payload").unwrap();
        let stamp = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        fs::File::options()
            .write(true)
            .open(&source)
            .unwrap()
            .set_modified(stamp)
            .unwrap();
        let tree = vec![file("001.docx", Some(&source.display().to_string()))];
        apply(&plan(&root, &tree));
        let copied = fs::metadata(root.join("001.docx")).unwrap().modified().unwrap();
        assert_eq!(copied, stamp);
    }

    #[test]
    fn copies_only_the_top_level_migration_files_matching_the_filter() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("studies");
        let migration = tmp.path().join("sop");
        fs::create_dir_all(migration.join("sub")).unwrap();
        fs::write(migration.join("a.sas7bdat"), b"a").unwrap();
        fs::write(migration.join("b.docx"), b"b").unwrap();
        fs::write(migration.join("sub").join("c.sas7bdat"), b"c").unwrap();
        let mut node = dir("01_raw", vec![]);
        node.migration = Some(Migration {
            directory: migration.display().to_string(),
            filter: vec!["*.sas7bdat".to_string()],
        });
        let report = apply(&plan(&root, &[node]));
        assert!(root.join("01_raw").join("a.sas7bdat").is_file());
        assert!(!root.join("01_raw").join("b.docx").exists());
        assert!(!root.join("01_raw").join("sub").exists());
        // The dir plus the one copied file.
        assert_eq!(report.created.len(), 2);
    }

    #[test]
    fn a_migration_file_already_present_is_skipped_not_overwritten() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("studies");
        let migration = tmp.path().join("sop");
        fs::create_dir_all(&migration).unwrap();
        fs::write(migration.join("a.sas7bdat"), b"new").unwrap();
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("01_raw").join("a.sas7bdat"), b"old").unwrap();
        let mut node = dir("01_raw", vec![]);
        node.migration = Some(Migration {
            directory: migration.display().to_string(),
            filter: vec!["*.sas7bdat".to_string()],
        });
        let report = apply(&plan(&root, &[node]));
        assert_eq!(
            fs::read(root.join("01_raw").join("a.sas7bdat")).unwrap(),
            b"old"
        );
        assert!(report
            .skipped
            .iter()
            .any(|s| s.reason == SkipReason::AlreadyExists));
    }

    #[test]
    fn the_report_carries_the_plan_time_skips() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("studies");
        let tree = vec![dir("{{ branch }}", vec![dir("01_raw", vec![])])];
        let scaffold = scaffolding::scaffold::build(
            &root.display().to_string(),
            &tree,
            &Context::new("AK001-001", None, chrono::Local::now().date_naive()),
        )
        .unwrap();
        let report = apply(&scaffold);
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].reason, SkipReason::UnresolvedField);
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p aegis-desktop --lib scaffold::`
Expected: FAIL — `apply` is not defined.

- [ ] **Step 4: Implement the applier**

In `apps/desktop/aegis-desktop/src-tauri/src/scaffold/apply.rs`, above the test module:

```rust
//! Executes a `scaffolding::Scaffold` against the real filesystem.
//!
//! This is the desktop's half of the initialize-project-folder command:
//! the *rules* live in the `scaffolding` crate, and this module only
//! touches the disk. Nothing here overwrites an existing file — an
//! existing target is skipped and reported.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use scaffolding::{Scaffold, SkipReason, Step};

use super::report::InitReportResponse;

/// Apply a scaffold, returning what was created and what was skipped.
pub fn apply(scaffold: &Scaffold) -> InitReportResponse {
    let mut created = Vec::new();
    let mut skipped = scaffold.skipped.clone();

    // The root is the base, not a node. It is created if missing.
    if !scaffold.root.exists() {
        if let Err(e) = fs::create_dir_all(&scaffold.root) {
            skipped.push(crate::scaffold::report::SkippedItemResponse {
                path: scaffold.root.display().to_string(),
                reason: SkipReason::MigrationFailed,
                detail: Some(e.to_string()),
            });
            return InitReportResponse {
                root: scaffold.root.display().to_string(),
                created,
                skipped,
            };
        }
        created.push(scaffold.root.display().to_string());
    }

    for step in &scaffold.steps {
        match step {
            Step::MakeDir { path } => {
                if path.exists() {
                    skipped.push(skipped_item(path, SkipReason::AlreadyExists, None));
                } else if let Err(e) = fs::create_dir_all(path) {
                    skipped.push(skipped_item(path, SkipReason::MigrationFailed, Some(e.to_string())));
                } else {
                    created.push(path.display().to_string());
                }
            }
            Step::CopyFile { from, to } => {
                if to.exists() {
                    skipped.push(skipped_item(to, SkipReason::AlreadyExists, None));
                    continue;
                }
                match copy_preserving_mtime(from, to) {
                    Ok(()) => created.push(to.display().to_string()),
                    Err(e) => skipped.push(skipped_item(to, SkipReason::ReferenceFailed, Some(e))),
                }
            }
            Step::CopyDirFiles {
                from_dir,
                to_dir,
                filter,
            } => {
                match fs::read_dir(from_dir) {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if !path.is_file() {
                                continue;
                            }
                            let name = entry.file_name();
                            let name = name.to_string_lossy();
                            if !matches_filter(&name, filter) {
                                continue;
                            }
                            let target = to_dir.join(&name);
                            if target.exists() {
                                skipped.push(skipped_item(&target, SkipReason::AlreadyExists, None));
                                continue;
                            }
                            match copy_preserving_mtime(&path, &target) {
                                Ok(()) => created.push(target.display().to_string()),
                                Err(e) => skipped.push(skipped_item(
                                    &target,
                                    SkipReason::MigrationFailed,
                                    Some(e),
                                )),
                            }
                        }
                    }
                    Err(e) => skipped.push(skipped_item(
                        from_dir,
                        SkipReason::MigrationFailed,
                        Some(e.to_string()),
                    )),
                }
            }
        }
    }

    InitReportResponse {
        root: scaffold.root.display().to_string(),
        created,
        skipped,
    }
}
```

And the helpers:

```rust
/// A skipped item.
fn skipped_item(
    path: &Path,
    reason: SkipReason,
    detail: Option<String>,
) -> crate::scaffold::report::SkippedItemResponse {
    crate::scaffold::report::SkippedItemResponse {
        path: path.display().to_string(),
        reason,
        detail,
    }
}

/// Copy a file, keeping the source's modification time.
///
/// `fs::copy` preserves the mtime on Windows but not portably, so the
/// timestamp is set explicitly.
fn copy_preserving_mtime(from: &Path, to: &Path) -> Result<(), String> {
    let mtime = fs::metadata(from)
        .and_then(|m| m.modified())
        .map_err(|e| e.to_string())?;
    fs::copy(from, to).map_err(|e| e.to_string())?;
    fs::File::options()
        .write(true)
        .open(to)
        .map_err(|e| e.to_string())?
        .set_modified(mtime)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Whether a filename matches the migration filter.
///
/// A leading `*.` is stripped, then the remainder is matched as a
/// case-insensitive suffix. Only a *leading* `*.` is recognised.
fn matches_filter(name: &str, filter: &[String]) -> bool {
    if filter.is_empty() {
        return true;
    }
    let name = name.to_lowercase();
    filter.iter().any(|pattern| {
        let suffix = pattern
            .strip_prefix("*.")
            .unwrap_or(pattern)
            .to_lowercase();
        name.ends_with(&suffix)
    })
}
```

- [ ] **Step 5: Create the report DTOs**

Create `apps/desktop/aegis-desktop/src-tauri/src/scaffold/report.rs`:

```rust
//! The report the initialize-project-folder command returns.
//!
//! These types have no server counterpart — they are the desktop's
//! own answer about what happened on disk — so they live here rather
//! than in `http/`, which mirrors the server's wire shapes.

use scaffolding::SkipReason;
use serde::{Deserialize, Serialize};

/// What the command did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitReportResponse {
    /// The base the tree was written under.
    pub root: String,
    /// The paths that were created.
    pub created: Vec<String>,
    /// The nodes that were not created, with the reason.
    pub skipped: Vec<SkippedItemResponse>,
}

/// One node that was not created.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedItemResponse {
    /// The path the node would have had. For a node whose name could
    /// not be resolved this joins the raw, unresolved `entry_name` — it
    /// is not a real directory.
    pub path: String,
    pub reason: SkipReason,
    /// The missing field name, the raw template text, or the
    /// underlying I/O error message.
    pub detail: Option<String>,
}
```

Create `apps/desktop/aegis-desktop/src-tauri/src/scaffold.rs`:

```rust
//! Materialises a structure template onto the desktop machine's disk.
//!
//! The *rules* — context derivation, template resolution, and the
//! tree-to-plan transformation — live in the `scaffolding` crate. This
//! module holds only the filesystem half.
pub mod apply;
pub mod report;
```

- [ ] **Step 6: Run the applier tests to verify they pass**

Run: `cargo test -p aegis-desktop --lib scaffold::`
Expected: PASS — 8 tests.

- [ ] **Step 7: Write the failing test for the command's invoke keys**

In `apps/desktop/aegis-desktop/src-tauri/src/commands/structures.rs`, add to the `EXPECTED_INVOKE_KEYS` array:

```rust
        ("initialize_project_folder", &["structureId", "study", "branch"]),
```

Run: `cargo test -p aegis-desktop --lib commands::structures`
Expected: FAIL — `invoke_keys_match_the_documented_shape` panics with "command initialize_project_folder not found in this file", because the command lives in `commands/scaffold.rs`, not `structures.rs`.

**Note:** `assert_invoke_keys` reads the *given file's* source. The new command is in a different module, so the check must move. In `commands/scaffold.rs`, add its own test module with its own `EXPECTED_INVOKE_KEYS` — and remove the entry just added to `commands/structures.rs`. The two files each check their own commands; the Rust side and the TS side each keep one list per command.

- [ ] **Step 8: Implement the command**

Create `apps/desktop/aegis-desktop/src-tauri/src/commands/scaffold.rs`:

```rust
//! The initialize-project-folder command.
//!
//! Unlike every other command in this crate, this one writes to the
//! local disk: it fetches the structure's tree over HTTP, plans the
//! scaffold with the `scaffolding` crate, and applies it. The rules
//! live in that crate; this file only wires them together.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::dto::ApiError;
use crate::http::structures::{self, NodeViewResponse, StructureViewResponse};
use crate::scaffold::apply::apply;
use crate::scaffold::report::InitReportResponse;
use crate::state::{Caller, RequestContext, SharedAppState};
use scaffolding::{Context, StructureNode};

/// Materialise a structure template into a project folder.
///
/// Tauri maps each JSON key in the args object to a parameter by
/// camelCase name, so the frontend sends `{ structureId, study, branch }`.
#[tauri::command]
pub async fn initialize_project_folder(
    app_state: tauri::State<'_, SharedAppState>,
    structure_id: i64,
    study: String,
    branch: Option<String>,
) -> Result<InitReportResponse, ApiError> {
    let req_ctx = RequestContext {
        trace_id: app_state.trace_id_generator().client_side(),
        caller: Caller::User,
    };
    initialize_project_folder_impl(app_state.inner(), &req_ctx, structure_id, study, branch).await
}

pub async fn initialize_project_folder_impl(
    app_state: &SharedAppState,
    req_ctx: &RequestContext,
    structure_id: i64,
    study: String,
    branch: Option<String>,
) -> Result<InitReportResponse, ApiError> {
    let span = tracing::info_span!(
        "command",
        trace_id = %req_ctx.trace_id,
        caller = %req_ctx.caller,
        command = "initialize_project_folder"
    );
    async move {
        tracing::info!("enter");
        let result = TRACE_ID
            .scope(req_ctx.trace_id.clone(), async {
                let structure =
                    structures::get_structure(app_state.http_client(), structure_id).await?;
                let context = Context::new(
                    &study,
                    branch.as_deref(),
                    chrono::Local::now().date_naive(),
                );
                let nodes = map_nodes(&structure.nodes);
                let scaffold = scaffolding::scaffold::build(&structure.rootpath, &nodes, &context)
                    .map_err(|e| ApiError::Parse {
                        message: e.to_string(),
                    })?;
                Ok(apply(&scaffold))
            })
            .await;
        match &result {
            Ok(_) => tracing::info!("success"),
            Err(e) => tracing::error!(error = %e, "failed"),
        }
        result
    }
    .instrument(span)
    .await
}

/// Map the server's wire tree onto the `scaffolding` crate's node type.
fn map_nodes(nodes: &[NodeViewResponse]) -> Vec<StructureNode> {
    nodes
        .iter()
        .map(|n| StructureNode {
            entry_name: n.entry_name.clone(),
            directory: n.directory,
            reference: n.reference.clone(),
            migration: n.migration.as_ref().map(|m| scaffolding::Migration {
                directory: m.directory.clone(),
                filter: m.filter.clone(),
            }),
            children: map_nodes(&n.children),
        })
        .collect()
}
```

- [ ] **Step 9: Register the command**

In `apps/desktop/aegis-desktop/src-tauri/src/commands.rs`, add `pub mod scaffold;` to the barrel (keep the list alphabetical — it goes after `pub mod project;`).

In `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`, in the `tauri::generate_handler![...]` list, add after the structures commands:

```rust
            commands::scaffold::initialize_project_folder,
```

- [ ] **Step 10: Add the command's own invoke-key test**

In `apps/desktop/aegis-desktop/src-tauri/src/commands/scaffold.rs`, append:

```rust
#[cfg(test)]
mod tests {
    //! Wire-shape tests. Tauri routes JSON keys to function parameters
    //! by camelCase name, so the frontend wrapper must send exactly:
    //!   initialize_project_folder: { structureId, study, branch }

    use super::*;
    use crate::commands::test_support::assert_invoke_keys;

    const EXPECTED_INVOKE_KEYS: &[(&str, &[&str])] = &[
        ("initialize_project_folder", &["structureId", "study", "branch"]),
    ];

    #[test]
    fn invoke_keys_match_the_documented_shape() {
        assert_invoke_keys(include_str!("scaffold.rs"), EXPECTED_INVOKE_KEYS);
    }
}
```

- [ ] **Step 11: Verify the desktop backend compiles and its tests pass**

Run: `cargo test -p aegis-desktop --lib`
Expected: PASS — including the new invoke-key test.

Run: `cargo fmt --all && cargo clippy -p aegis-desktop --all-targets -- -D warnings`
Expected: PASS with no warnings.

- [ ] **Step 12: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri
git commit -m "feat(desktop): add the initialize_project_folder command

Fetches a structure's tree over HTTP, plans the scaffold with the
scaffolding crate, and applies it to the local disk — skipping what
already exists and reporting every skip with its reason.

Co-Authored-By: Claude Code <noreply@anthropic.com>
```

---

### Task 4: The frontend — the dialog runs the command and shows the report

Wire the TypeScript side: the API wrapper, the mutation hook, the dialog, and the parent section.

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts` — add the report mirrors
- Modify: `apps/desktop/aegis-desktop/src/shared/api/index.ts` — add `initializeProjectFolder`
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/data/project-branches.ts` — add the mutation hook
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts` — export the hook
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/components/InitializeProjectFolderDialog.tsx`
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/components/ProjectBranchesSection.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/templates/api.test.ts` — add the invoke keys
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-branches-section.test.tsx`
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

**Interfaces:**
- Consumes: `InitReportResponse` from Task 3; `api.initializeProjectFolder` from this task's Step 1.
- Produces:
  - `shared/api/types.ts`: `InitReport { root: string; created: string[]; skipped: SkippedItem[] }`, `SkippedItem { path: string; reason: SkipReason; detail: string | null }`, `SkipReason = "alreadyExists" | "unresolvedField" | "malformedTemplate" | "invalidName" | "noReference" | "referenceFailed" | "migrationFailed"`
  - `shared/api/index.ts`: `api.initializeProjectFolder(input: { structureId: number; study: string; branch?: string | null }): Promise<InitReport>`
  - `features/project-workspace/data/project-branches.ts`: `useInitializeProjectFolder(): UseMutationResult<InitReport, ApiError, { structureId: number; study: string; branch?: string | null }>`
  - `InitializeProjectFolderDialog` props: `open`, `projectCode: string`, `branch?: ProjectBranchView`, `onClose: () => void`, `onInitialized: (report: InitReport) => void`

- [ ] **Step 1: Add the TypeScript wire mirrors**

In `apps/desktop/aegis-desktop/src/shared/api/types.ts`, after the structures block, add:

```ts
// ---- initialize project folder --------------------------------------
//
// Wire types for the initialize-project-folder command. They mirror
// the Rust DTOs in `src-tauri/src/scaffold/report.rs` field for field.
// The report has no server counterpart — the desktop answers it
// itself about what happened on disk.

/** Why a node was not created. */
export type SkipReason =
  | "alreadyExists"
  | "unresolvedField"
  | "malformedTemplate"
  | "invalidName"
  | "noReference"
  | "referenceFailed"
  | "migrationFailed";

export interface SkippedItem {
  /**
   * The path the node would have had. For a node whose name could
   * not be resolved this joins the raw, unresolved `entryName` — it
   * is not a real directory.
   */
  path: string;
  reason: SkipReason;
  /** The missing field name, the raw template text, or the I/O error. */
  detail: string | null;
}

export interface InitReport {
  /** The base the tree was written under. */
  root: string;
  /** The paths that were created. */
  created: string[];
  /** The nodes that were not created, with the reason. */
  skipped: SkippedItem[];
}
```

- [ ] **Step 2: Add the API wrapper**

In `apps/desktop/aegis-desktop/src/shared/api/index.ts`, in the structures section, add:

```ts
  initializeProjectFolder: (input: {
    structureId: number;
    study: string;
    branch?: string | null;
  }): Promise<InitReport> =>
    call<InitReport>("initialize_project_folder", {
      structureId: input.structureId,
      study: input.study,
      branch: input.branch ?? null,
    }),
```

And add `InitReport` to the type imports at the top of that file (alphabetically, after `Identity`).

- [ ] **Step 3: Add the invoke keys to the TS test**

In `apps/desktop/aegis-desktop/src/test/features/templates/api.test.ts`, add to `EXPECTED_INVOKE_KEYS`:

```ts
  initialize_project_folder: ["structureId", "study", "branch"],
```

And add a case to the `cases` array:

```ts
      [
        "initialize_project_folder",
        () =>
          api.initializeProjectFolder({
            structureId: 3,
            study: "AK001-001",
            branch: "2026Q1",
          }),
      ],
```

And a handler in the `mockCommands({...})` call:

```ts
      initialize_project_folder: () => ({
        root: "D:\\studies",
        created: [],
        skipped: [],
      }),
```

Run: `pnpm --filter aegis-desktop test -- src/test/features/templates/api.test.ts`
Expected: PASS.

- [ ] **Step 4: Add the mutation hook**

In `apps/desktop/aegis-desktop/src/features/project-workspace/data/project-branches.ts`, add:

```ts
/**
 * Materialise a structure template into a project folder on disk.
 *
 * The command does real filesystem work on the desktop machine, so it
 * is not retried — a retry would re-run a partially-applied scaffold.
 */
export function useInitializeProjectFolder() {
  return useMutation<
    InitReport,
    ApiError,
    { structureId: number; study: string; branch?: string | null }
  >({
    mutationFn: (input) => api.initializeProjectFolder(input),
  });
}
```

And add `InitReport` to that file's type imports from `../../shared/api`.

- [ ] **Step 5: Export the hook from the feature barrel**

In `apps/desktop/aegis-desktop/src/features/project-workspace/index.ts`, add `useInitializeProjectFolder` to the `export { ... }` block from `./data/project-branches` (alphabetical: after `useCreateProjectBranch`).

- [ ] **Step 6: Write the failing test for the dialog**

In `apps/desktop/aegis-desktop/src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx`, add a test:

```tsx
  it("runs the command and shows the report", async () => {
    const { onInitialized } = renderDialog();
    await userEvent.click(screen.getByRole("combobox"));
    await userEvent.click(
      await screen.findByRole("option", { name: "Minimal" }),
    );
    await userEvent.click(screen.getByRole("button", { name: /confirm/i }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("initialize_project_folder", {
        structureId: 2,
        study: "AK001-001",
        branch: null,
      }),
    );
    expect(
      await screen.findByTestId("initialize-report-root"),
    ).toHaveTextContent("D:\\studies\\ak001\\001");
    expect(screen.getByTestId("initialize-report-created")).toHaveTextContent(
      "2",
    );
    expect(screen.getByTestId("initialize-report-skipped")).toHaveTextContent(
      "1",
    );
    expect(screen.getByText(/already exists/i)).toBeInTheDocument();
  });
```

The test needs `renderDialog` to pass `projectCode="AK001-001"` and the `invoke` mock to answer `initialize_project_folder`. Update the file's `beforeEach` to route by command name:

```tsx
beforeEach(() => {
  vi.mocked(invoke).mockReset();
  vi.mocked(invoke).mockImplementation((cmd: string) => {
    if (cmd === "list_structures") return Promise.resolve(TEMPLATES);
    if (cmd === "initialize_project_folder") {
      return Promise.resolve({
        root: "D:\\studies\\ak001\\001",
        created: ["D:\\studies\\ak001\\001\\01_raw", "D:\\studies\\ak001\\001\\01_raw\\spec.docx"],
        skipped: [
          { path: "D:\\studies\\ak001\\001\\02_clean", reason: "alreadyExists", detail: null },
        ],
      });
    }
    return Promise.reject(new Error(`unexpected tauri command: ${cmd}`));
  });
});
```

And update `renderDialog` to pass `projectCode="AK001-001"`.

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx`
Expected: FAIL — the dialog does not call the command and renders no report.

- [ ] **Step 7: Implement the dialog**

Replace `apps/desktop/aegis-desktop/src/features/project-workspace/components/InitializeProjectFolderDialog.tsx` with:

```tsx
import { useState } from "react";
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  List,
  ListItem,
  MenuItem,
  Stack,
  TextField,
  Typography,
} from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { useListTemplateStructures } from "../../templates";
import { useInitializeProjectFolder } from "../data/project-branches";
import { errorMessage } from "../../../shared/api/error";
import type { InitReport, ProjectBranchView } from "../../../shared/api";

export interface InitializeProjectFolderDialogProps {
  open: boolean;
  /** The project code — the context's `study`. */
  projectCode: string;
  /** The branch the folder is being initialized for, when the dialog
   *  was opened from a branch row. */
  branch?: ProjectBranchView;
  onClose: () => void;
  /** Called with the report when the folder has been generated. */
  onInitialized: (report: InitReport) => void;
}

/**
 * "Initialize project folder" dialog.
 *
 * Picking a template runs the `initialize_project_folder` command,
 * which materialises the structure on the desktop machine's disk. The
 * command's report — what was created, what was skipped and why —
 * stays in the dialog, because a skipped node is a normal outcome and
 * the user needs to see the reason.
 */
export function InitializeProjectFolderDialog({
  open,
  projectCode,
  branch,
  onClose,
  onInitialized,
}: InitializeProjectFolderDialogProps) {
  const { t } = useI18n();
  const structures = useListTemplateStructures();
  const initialize = useInitializeProjectFolder();
  const [structureId, setStructureId] = useState<number | null>(null);
  const [report, setReport] = useState<InitReport | null>(null);

  const selected = structures.data?.find((s) => s.id === structureId);

  async function handleConfirm() {
    if (!selected) return;
    try {
      const result = await initialize.mutateAsync({
        structureId: selected.id,
        study: projectCode,
        branch: branch?.branch ?? null,
      });
      setReport(result);
      onInitialized(result);
    } catch {
      // The error is rendered inside the dialog, which stays open.
    }
  }

  function handleClose() {
    setReport(null);
    setStructureId(null);
    initialize.reset();
    onClose();
  }

  return (
    <Dialog open={open} onClose={handleClose}>
      <DialogTitle>{t("project.branches.initialize.title")}</DialogTitle>
      <DialogContent>
        {branch && (
          <Typography variant="body2" sx={{ mb: 2 }}>
            {branch.branch}
          </Typography>
        )}

        {report ? (
          <Stack spacing={2} data-testid="initialize-report">
            <Typography variant="body2" data-testid="initialize-report-root">
              {report.root}
            </Typography>
            <Box sx={{ display: "flex", gap: 2 }}>
              <Typography
                variant="body2"
                data-testid="initialize-report-created"
              >
                {t("project.branches.initialize.report.created", {
                  count: report.created.length,
                })}
              </Typography>
              <Typography
                variant="body2"
                data-testid="initialize-report-skipped"
              >
                {t("project.branches.initialize.report.skipped", {
                  count: report.skipped.length,
                })}
              </Typography>
            </Box>
            {report.skipped.length > 0 && (
              <List dense data-testid="initialize-report-skipped-list">
                {report.skipped.map((item, index) => (
                  <ListItem key={index}>
                    <Typography variant="body2">{item.path}</Typography>
                    <Typography variant="caption" color="text.secondary">
                      {t(
                        `project.branches.initialize.report.reason.${item.reason}`,
                      )}
                    </Typography>
                  </ListItem>
                ))}
              </List>
            )}
          </Stack>
        ) : (
          <>
            <TextField
              select
              fullWidth
              size="small"
              label={t("project.branches.initialize.structure")}
              value={structureId ?? ""}
              onChange={(e) => {
                const raw = e.target.value;
                setStructureId(raw === "" ? null : Number(raw));
              }}
              helperText={
                structureId === null
                  ? t("project.branches.initialize.required")
                  : undefined
              }
              data-testid="initialize-folder-structure"
            >
              {(structures.data ?? []).map((s) => (
                <MenuItem key={s.id} value={s.id}>
                  {s.name}
                </MenuItem>
              ))}
            </TextField>
            {initialize.isError && (
              <Alert severity="error" sx={{ mt: 2 }}>
                {t("project.branches.initialize.failed", {
                  message: errorMessage(initialize.error),
                })}
              </Alert>
            )}
          </>
        )}
      </DialogContent>
      <DialogActions>
        {report ? (
          <Button onClick={handleClose}>{t("common.close")}</Button>
        ) : (
          <>
            <Button onClick={handleClose} disabled={initialize.isPending}>
              {t("common.cancel")}
            </Button>
            <Button
              variant="contained"
              disabled={!selected || initialize.isPending}
              onClick={() => void handleConfirm()}
              data-testid="initialize-folder-confirm"
            >
              {t("common.confirm")}
            </Button>
          </>
        )}
      </DialogActions>
    </Dialog>
  );
}
```

- [ ] **Step 8: Add the i18n keys**

In `lib/packages/ui/src/i18n/locales/en.ts`, after `'project.branches.initialize.success':`, add:

```ts
  'project.branches.initialize.failed': 'Initialize failed: {message}',
  'project.branches.initialize.report.created': 'Created {count} item(s)',
  'project.branches.initialize.report.skipped': 'Skipped {count} item(s)',
  'project.branches.initialize.report.reason.alreadyExists': 'Already exists',
  'project.branches.initialize.report.reason.unresolvedField':
    'No field "{detail}" in this context',
  'project.branches.initialize.report.reason.malformedTemplate':
    'Malformed template',
  'project.branches.initialize.report.reason.invalidName':
    'Not a valid folder name',
  'project.branches.initialize.report.reason.noReference':
    'No file to copy',
  'project.branches.initialize.report.reason.referenceFailed':
    'The file could not be copied',
  'project.branches.initialize.report.reason.migrationFailed':
    'The migration could not be copied',
```

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, add the same keys with Chinese values:

```ts
  'project.branches.initialize.failed': '初始化失败：{message}',
  'project.branches.initialize.report.created': '已创建 {count} 项',
  'project.branches.initialize.report.skipped': '已跳过 {count} 项',
  'project.branches.initialize.report.reason.alreadyExists': '已存在',
  'project.branches.initialize.report.reason.unresolvedField':
    '上下文中没有字段 “{detail}”',
  'project.branches.initialize.report.reason.malformedTemplate': '模板格式错误',
  'project.branches.initialize.report.reason.invalidName': '不是合法的文件夹名称',
  'project.branches.initialize.report.reason.noReference': '没有可复制的文件',
  'project.branches.initialize.report.reason.referenceFailed': '文件复制失败',
  'project.branches.initialize.report.reason.migrationFailed': '迁移文件复制失败',
```

**Note:** `common.close` already exists in both locales — no new key is needed for it.

Run: `pnpm --filter @aegis/ui test`
Expected: PASS — the registry test asserts `en` and `zhCN` have the same key set.

- [ ] **Step 9: Run the dialog test to verify it passes**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/initialize-project-folder-dialog.test.tsx`
Expected: PASS.

- [ ] **Step 10: Update the parent section**

In `apps/desktop/aegis-desktop/src/features/project-workspace/components/ProjectBranchesSection.tsx`:

- The `InitializeProjectFolderDialog` usage becomes:

```tsx
      {initializeBranch && (
        <InitializeProjectFolderDialog
          open
          projectCode={projectCode}
          branch={initializeBranch}
          onClose={() => setInitializeBranch(null)}
          onInitialized={handleInitialized}
        />
      )}
```

- Add a handler and drop the success snackbar:

```tsx
  function handleInitialized() {
    // The report stays in the dialog — a skipped node is a normal
    // outcome and the user needs to see its reason there.
  }
```

- Remove the `initialized` state and the `Snackbar` element, and the now-unused `Snackbar` import.

Run: `pnpm --filter aegis-desktop typecheck`
Expected: PASS.

- [ ] **Step 11: Update the branches-section test**

In `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-branches-section.test.tsx`, the mocked `invoke` must now answer `initialize_project_folder`. Add to `mockInvokeWith`:

```ts
    if (cmd === "initialize_project_folder") {
      return Promise.resolve({
        root: "D:\\studies\\ak001\\001",
        created: [],
        skipped: [],
      });
    }
```

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-branches-section.test.tsx`
Expected: PASS.

- [ ] **Step 12: Run the whole frontend suite and commit**

Run: `pnpm --filter aegis-desktop typecheck && pnpm --filter aegis-desktop test && pnpm --filter @aegis/ui test`
Expected: PASS.

```bash
git add apps/desktop/aegis-desktop/src lib/packages/ui/src/i18n
git commit -m "feat(desktop): run the initialize command from the dialog

The dialog now materialises the template on disk and shows the
command's report — what was created, and every skip with its reason.

Co-Authored-By: Claude Code <noreply@anthropic.com>
```

---

## Self-Review

**Spec coverage.** Every spec section maps to a task:

| Spec section | Task |
| --- | --- |
| The path rule (rootpath verbatim, tree carries the path) | 2 (`resolves_the_tree_into_ordered_steps`) |
| Context (study/product/trial/branch/date) | 1 |
| Syntax templates (fields, operations, whitespace, multiple) | 1 |
| Rule 1 — resolve the structure (effective tree) | 3 (the command fetches it) |
| Rule 2 — resolve the root | 2 |
| Rule 3 — resolve each `entry_name` | 2 |
| Rule 4 — create directories | 4 (the applier) |
| Rule 5 — skip what exists, descend into existing dirs | 4 |
| Rule 6 — copy references, rename to `entry_name` | 4 |
| Rule 7 — apply migrations, top level only | 4 |
| Rule 8 — preserve timestamps | 4 |
| Unresolved fields | 2, 4 |
| Invalid resolved names | 2 |
| Migration filters | 4 (`matches_filter`) |
| Relative paths | 2 |
| The report (shape, reasons, what appears where) | 2, 3, 4 |
| Where the code lives (crate split) | 1, 2, 3 |
| Timestamps | 4 |
| The frontend | 4 |
| Wiring details (both invoke-key lists, report DTO location) | 3, 4 |
| Testing | 1–4 |

**Step scan.** Each step names one action with a checkable result. Two steps carry a note rather than a transcript: Task 2 Step 4's `resolve_against` note (the test already pins the expected resolution) and Task 3 Step 7's note about the invoke-key list moving to the new module (the test pins the failure).

**Type consistency.** `Context`, `Scaffold`, `Step`, `SkipReason`, `SkippedItem`, `StructureNode`, `Migration` are defined in Tasks 1–2 and consumed verbatim in Task 3. The TS mirrors (`InitReport`, `SkippedItem`, `SkipReason`) are defined in Task 4 Step 1 and consumed in Steps 2 and 4. The `SkipReason` variants match one-for-one between Rust and TypeScript, in the same camelCase spelling.

**Review Focus.** Each of the five lines is pinned by a test in the task that owns the code: the path-separator branch (Task 2, `a_resolved_name_with_a_path_separator_is_skipped`), the study code with no dash (Task 2, `a_missing_trial_skips_only_the_node_that_needs_it`), re-running against an existing folder (Task 4, `skips_an_existing_directory_but_still_creates_its_children` and `skips_an_existing_file_without_overwriting_it`), the `*.sas7bdat` filter (Task 4, `copies_only_the_top_level_migration_files_matching_the_filter`), and the empty `rootpath` (Task 2, `an_empty_rootpath_is_the_one_hard_error`).

**Proportion.** The plan is roughly the same length as the spec — the code blocks are the signatures and the test bodies that fix the behaviour, not the implementation.
