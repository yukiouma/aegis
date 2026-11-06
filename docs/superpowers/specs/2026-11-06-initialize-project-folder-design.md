# Initialize Project Folder — Design

## Goal

Turn a structure template into a real project folder on the desktop machine's
disk.

The `InitializeProjectFolderDialog` already exists and lets the user pick a
template structure. What is missing is the thing that actually creates the
folders: a Tauri command that walks the structure's tree, resolves each
node's `entry_name` against a context, and materialises the result under the
structure's `rootpath`.

1. **A tauri command** — `initialize_project_folder` — that takes a structure
   id plus the context, and returns a report of what it did.
2. **The dialog calls it** — instead of only reporting the chosen structure
   name, the dialog runs the command and shows the report.

The scope is **desktop-only**. The server already exposes
`GET /api/structures/{id}` returning the structure with its effective tree,
which is exactly the input the command needs. Nothing in `apps/server` or
`lib/crates/structures` changes.

## Scope

| In | Out |
| --- | --- |
| A tauri command that generates a project folder on disk | Recording the generated folder server-side |
| Template syntax resolution (`{{ field }}`, `{{ field \| op }}`) | A server-side syntax parser or endpoint |
| Copying `reference` files, `migration` files, and preserving mtimes | Moving or deleting existing files |
| A per-node report (created / skipped + reason) surfaced in the dialog | A project-level structure being created as a side effect |
| New i18n keys in `en` + `zh-CN` | Any change to the templates feature |

## Terminology

- **Structure** — a recorded directory layout: a `rootpath` plus a tree of
  nodes. A *template* structure is global and shared; a *project* structure
  belongs to one project and extends exactly one template.
- **Node** — one folder or one file in the tree. `entry_name` is its own
  name; nesting comes from the parent link.
- **Context** — the values a template can interpolate: `study`, `product`,
  `trial`, `branch`, `date`.
- **Scaffold** — the resolved list of filesystem operations, computed *before*
  anything is written to disk.
- **Skip** — a node that was not created, with the reason why. Skips are
  detected at two different times: **plan time** (the name could not be
  resolved) and **apply time** (the target already exists, or a copy
  failed).

## The path rule

`rootpath` is the **shared base** for every project — e.g. `D:\studies`.
Each project lands in a sub-path of it, derived from the structure's own
tree, not from the materializer.

```
rootpath = D:\studies

tree:
  {{ product | lowercase }}      -> ak001
    {{ trial }}                  -> 001
      01_raw                     -> 01_raw
      02_clean                   -> 02_clean

result:
  D:\studies\ak001\001\01_raw
  D:\studies\ak001\001\02_clean
```

A second project `AK001-002` under the same template produces
`D:\studies\ak001\002\...` and shares the `ak001` level folder. A project
`XY002-001` produces `D:\studies\xy002\001\...`.

**This is the requester's decision.** The alternative — the materializer
always prepending `<product>\<trial>` — was rejected: it would make the
per-project segregation an implicit rule of the generator rather than
something the template declares, and the `| lowercase` operation in the
specification would then have no purpose.

**Consequence to keep in mind:** a template whose tree lacks a
product/trial level puts every project in the same folder. That is the
template author's choice to make, and the design does not second-guess it.

## Context

Resolved from the values the desktop already has:

| Field | Value | Source |
| --- | --- | --- |
| `study` | the project code | `ProjectBranchesSection`'s `projectCode` |
| `product` | `study` split on `-`, first element | derived |
| `trial` | `study` split on `-`, second element | derived |
| `branch` | the branch being initialised | the dialog's `branch` prop |
| `date` | today, `yyyy-mm-dd` | the desktop machine's clock |

`branch` is optional: the dialog can be opened without one, and `{{ branch }}`
then resolves to nothing (see "Unresolved fields" below).

`product` and `trial` are derived from `study` by splitting on `-`. A study
code with no `-` yields a `product` equal to the whole code and **no**
`trial` — `{{ trial }}` is then unresolved. The derivation lives in the
`scaffolding` crate, not the desktop, so it is unit-testable.

## Syntax templates

A template is written `{{ … }}` inside an `entry_name`.

```
{{ study }}                 -> AK001-001
{{ product | lowercase }}   -> ak001
{{ trial | uppercase }}     -> 001
{{ product }}-{{ trial }}   -> ak001-001
```

- The field name is one of `study`, `product`, `trial`, `branch`, `date`.
- The optional operation, after `|`, is `lowercase` or `uppercase`.
- Any number of templates may appear in one `entry_name`; each is replaced
  in place.
- Whitespace inside the braces is insignificant.

An unknown field name, or an unknown operation, is an **unresolved field**
(see below). The parser is strict on purpose: a typo must not silently
produce a directory named `{{ prodcut }}`.

An `entry_name` with an opening `{{` and no closing `}}` is **malformed**.
It is treated the same as an unresolved field — the node is skipped — rather
than aborting the whole run, so one bad name in a template cannot stop the
other 99 folders from being created.

## Behaviour

The rules below are the requester's, in order.

1. **Resolve the structure.** Fetch the structure by id and use its
   effective tree. Hidden nodes are already excluded from the effective
   tree the server returns (`structures/src/domain/merge.rs`), so no
   client-side filtering is needed.
2. **Resolve the root.** `rootpath` is the base. It is used verbatim — the
   `{{ … }}` syntax applies to `entry_name`, not to `rootpath`.
3. **Resolve each `entry_name`** against the context. A node whose
   `entry_name` cannot be resolved is **skipped**; if it is a directory, its
   whole subtree is skipped too.
4. **Create directories.** A directory node becomes a directory under its
   parent. A file node becomes a file under its parent.
5. **Skip what already exists.** If the target directory or file already
   exists, skip it — but if it is a directory, still walk its children, so
   missing descendants are still created.
6. **Copy references.** A file node with a `reference` has that file copied
   to the target path and renamed to the resolved `entry_name`. A file node
   with no `reference` is skipped — an empty placeholder file is not
   created.
7. **Apply migrations.** A directory node with a `migration` gets the
   top-level files of `migration.directory` copied into it. Subdirectories of
   the migration directory are not recursed into.
8. **Preserve timestamps.** Every file copied from another source keeps its
   original modification time.

### Unresolved fields

A node is skipped when its `entry_name` references a field the context does
not have — `{{ branch }}` with no branch selected, or `{{ trial }}` for a
study code with no second segment. Skipping a directory skips its subtree:
the children would be orphaned.

The skip is recorded in the report with the missing field name, so the user
sees *why* a folder did not appear rather than just that it did not.

**What path is reported for a skipped node.** The name cannot be resolved,
so there is no real path. The report carries the *parent's* path joined with
the raw, unresolved `entry_name` — e.g. `D:\studies\ak001\001\{{ branch }}`.
It is not a real directory, and it is not meant to be: it shows the user
exactly which template line was skipped and where it would have gone.

### Invalid resolved names

The server validates an `entry_name` as a single path segment — no `/`, no
`\`, not `.` or `..` ([`node.rs` `validate_name`](../lib/crates/structures/src/domain/node.rs)).
That validation covers the *stored* name, not the *resolved* one, and the
resolved one is built from user-controlled strings: a branch name is only
checked for emptiness ([`project_branch.rs`](../lib/crates/project/src/domain/project_branch.rs)),
so `feat/login` is a legal branch.

A resolved name containing a path separator is therefore **skipped and
reported** with reason `InvalidName`, never sanitised and never allowed to
create nested directories. This is what stops a branch named
`../../Windows` from writing outside `rootpath`. A resolved name that is
empty is treated the same way.

### Existing targets

"Skip what exists" is what makes the command re-runnable: running it twice
against the same project adds only what is missing. It also means an
existing file is never overwritten — including the files a migration would
copy, which are skipped with reason `AlreadyExists` if they are already
there.

### Migration filters

`migration.filter` holds filename patterns. A pattern is matched by:

1. Stripping a leading `*.` if present.
2. Matching the remainder as a **case-insensitive suffix** of the filename.

```
filter = ["*.sas7bdat"]  ->  suffix ".sas7bdat"
  a.sas7bdat   -> matched
  d.docx       -> no match

filter = ["docx"]        ->  suffix "docx"
  a.docx       -> matched
  a.docx.bak   -> matched
```

Both spellings work: `*.sas7bdat` (what the repo's existing template data
uses) and `docx` or `.docx` (the plain suffix form).

This is a deliberate choice over general glob matching. The `structures`
domain calls these "target suffixes", the requester's specification says
"apply the filter as the filename suffix", and the data already in the repo
is `*.sas7bdat` / `*.xpt`. Strip-and-suffix is the one rule that satisfies
all three. Only a *leading* `*.` is recognised — `sas*` is not a supported
pattern and matches nothing.

### Relative paths

`reference` and `migration.directory` are absolute paths on the desktop
machine (they come from `PathPicker`, which opens the OS file picker). A
relative path is resolved against `rootpath` rather than rejected, so both
forms work.

## The report

Partial success is the normal case — things exist, fields do not resolve.
The command therefore returns a report rather than a bare path.

```rust
pub struct InitReport {
    pub root: String,
    pub created: Vec<String>,
    pub skipped: Vec<SkippedItem>,
}

pub struct SkippedItem {
    pub path: String,
    pub reason: SkipReason,
    pub detail: Option<String>,
}

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
```

`root` is the resolved base the tree was written under. `detail` carries
the missing field name for `UnresolvedField`, the raw template text for
`MalformedTemplate`, and the underlying I/O error message for the failure
reasons.

`SkipReason` and `SkippedItem` are defined in the `scaffolding` crate, not
in the desktop: the plan-time skips and the apply-time skips are the same
vocabulary, and splitting the enum across two crates would mean the
desktop and the planner could drift apart.

### What the report says about each kind of work

- **A created directory** appears in `created` as its path.
- **A created file** (a reference copy, or a migration copy) appears in
  `created` as its path. A migration that copies *N* files therefore
  contributes *N* entries — the count reflects real files on disk, not
  abstract operations.
- **The root** is created with `create_dir_all` if it does not exist. If
  this run created it, its path appears in `created`; if it was already
  there, it appears in neither list — the root is the base, not a node.
- **An existing target** appears in `skipped` with reason `AlreadyExists`.
- **A migration source file that already exists at the target** appears in
  `skipped` with reason `AlreadyExists`, and is not overwritten.
- **A node skipped at plan time** appears in `skipped` with the
  plan-time reason.

## Where the code lives

**Hybrid.** The pure part — context building, template resolution, and
turning the tree into a scaffold of filesystem operations — lives in a new
workspace crate. The part that touches the disk lives in the desktop app.

This was the requester's choice, between three options:

- **A — all in the desktop crate.** A new `src-tauri/src/scaffold/` module.
  Simple, but bends the guideline that the desktop backend is transport-only.
- **B — a new lib crate doing everything**, filesystem included. Follows
  the layering guideline most literally, but a crate only the desktop
  consumes, doing I/O unlike its server-side siblings.
- **C — hybrid (chosen).** The pure planning in a lib crate; the
  filesystem work in the desktop.

### The new crate: `lib/crates/scaffolding`

A small crate with three dependencies — `std`, `chrono` (the date), and
`serde` (the report types cross the Tauri boundary):

```
src/
├── lib.rs        # re-exports
├── context.rs    # Context, its fields, and their derivation
├── template.rs   # the {{ … }} parser and resolver
├── scaffold.rs   # tree -> Scaffold
├── report.rs     # SkipReason, SkippedItem
└── error.rs      # ScaffoldError
```

Its public surface:

```rust
/// One node of the tree to scaffold. The desktop maps the server's
/// wire DTO onto this — the crate does not depend on `structures` or
/// on the desktop.
pub struct StructureNode {
    pub entry_name: String,
    pub directory: bool,
    pub reference: Option<String>,
    pub migration: Option<Migration>,
    pub children: Vec<StructureNode>,
}

pub struct Migration {
    pub directory: String,
    pub filter: Vec<String>,
}

pub struct Context {
    pub study: String,
    pub product: Option<String>,
    pub trial: Option<String>,
    pub branch: Option<String>,
    pub date: String,
}

impl Context {
    /// Build the context from the raw inputs, deriving `product` and
    /// `trial` from `study` and formatting `date` as `yyyy-mm-dd`.
    pub fn new(study: &str, branch: Option<&str>, date: NaiveDate) -> Context;
}

pub struct Scaffold {
    pub root: PathBuf,
    /// The filesystem work, in application order (parents first).
    pub steps: Vec<Step>,
    /// The nodes that could not be planned, with the reason.
    pub skipped: Vec<SkippedItem>,
}

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

`scaffold::build` is a pure function:
`(rootpath, tree, context) -> Result<Scaffold, ScaffoldError>`.
It resolves every `entry_name`, drops the unresolvable subtrees, and emits
the steps in application order (parents before children). It performs **no**
I/O — it does not know whether anything exists on disk.

**The scaffold carries its own skips.** The `steps` list cannot express
"this node was skipped because `{{ branch }}` was missing", so `Scaffold`
carries a `skipped` list beside it. The alternative — returning a tuple, or
growing a skip variant onto `Step` — makes the caller reassemble what the
planner already knows.

### When `scaffold::build` fails

`ScaffoldError` has exactly one variant:

```rust
pub enum ScaffoldError {
    /// The structure's `rootpath` is empty.
    EmptyRootpath,
}
```

An empty `rootpath` is reachable, not hypothetical: structures created
before the `rootpath` column existed read back with an empty root path
until one is set (see the `structures` README). There is nothing to
scaffold relative to, so this is a hard error rather than a skip.

Everything else that can go wrong with a *name* — an unresolved field, a
malformed template, an invalid resolved name — is a **skip**, not an
error. One bad name in a template must not stop the rest of the tree from
being created.

### The desktop side

`src-tauri/src/scaffold/` — the applier:

- Walks the `Scaffold`, executing each step against the real filesystem.
- Skips a step whose target already exists (but still descends into an
  existing directory's children — that is handled by the scaffold's step
  ordering, since children are separate steps).
- Copies files, preserving the source's modification time.
- Collects the report, merging the plan-time skips with its own.

The tauri command lives in `src-tauri/src/commands/scaffold.rs` and is
registered in `lib.rs`:

```rust
#[tauri::command]
pub async fn initialize_project_folder(
    app_state: tauri::State<'_, SharedAppState>,
    structure_id: i64,
    study: String,
    branch: Option<String>,
) -> Result<InitReportResponse, ApiError>
```

It fetches the structure via the existing `http::structures::get_structure`,
maps the wire DTO onto `scaffolding::StructureNode`, builds the `Context`,
calls `scaffolding::scaffold::build`, and hands the scaffold to the
applier.

**Note on the transport-only guideline.** The desktop backend is documented
as a transport layer with no domain logic, and `commands/` as a one-line
wrapper over `http/`. This command is the exception: it writes to the local
disk, which is inherently the desktop's job — the `structures` crate README
says so explicitly ("materialising it — creating directories, copying
migration files — is the desktop app's job"). The *rules* stay out of the
desktop by living in `scaffolding`; the desktop holds only the I/O.

### Error type

The command returns `ApiError`, the desktop's single error type. Filesystem
failures map onto it as `ApiError::Parse { message }` — the closest existing
variant for "something on disk was not as expected" — since `ApiError` has
no I/O variant and adding one would widen the shared wire contract for a
desktop-only concern.

A failure to fetch the structure is the ordinary `ApiError::Http` /
`ApiError::Network`. An empty `rootpath` is `ApiError::Parse` too, carrying
a message that names the structure.

## Timestamps

`std::fs::copy` preserves the modification time on Windows but not
portably. The applier sets the target's modification time explicitly from
the source's metadata after each copy, using `File::set_modified` (stable
since Rust 1.75). No new dependency is needed.

Directory timestamps are not preserved — only files copied from another
source, per rule 8.

## The frontend

`InitializeProjectFolderDialog` gains the real behaviour:

- On confirm, it calls `api.initializeProjectFolder({ structureId, study, branch })`.
- While the command runs, the confirm button shows a pending state.
- On success, the dialog **stays open** and shows the report: the resolved
  root, the created count, the skipped count, and each skipped path with
  its reason.
- On failure, the dialog shows the error and stays open.

The current `onInitialized(structureName)` prop is replaced by
`onInitialized(report)`. The parent (`ProjectBranchesSection`) drops its
success snackbar — the report in the dialog is the feedback — and instead
closes the dialog when the user dismisses the report.

The structure id, not the name, is what the command needs; the dialog
already has the selected `StructureSummaryView` and can pass its id.

`study` comes from the dialog's new `projectCode` prop, threaded down from
`ProjectBranchesSection`, which already has it.

## Wiring details the implementer must not miss

- The new command must be added to **both** copies of the invoke-key list:
  `EXPECTED_INVOKE_KEYS` in `src-tauri/src/commands/structures.rs` (asserted
  against the source text by `invoke_keys_match_the_documented_shape`) and
  the equivalent map in `src/test/features/templates/api.test.ts`. A command
  missing from either list fails that list's own test.
- `SkipReason` needs an explicit serde casing (`camelCase`, per the desktop
  convention) so the TypeScript union matches. The TS mirror of
  `InitReport` / `SkippedItem` / `SkipReason` goes in
  `src/shared/api/types.ts`.
- `src-tauri/src/http/` mirrors the server, and the report has no server
  counterpart — so the report DTO belongs to the desktop's own `scaffold`
  module, not to `http/structures.rs`.
- The new crate must be registered in the root `Cargo.toml`
  `[workspace].members`, and the desktop must depend on it by path.

## Testing

**`scaffolding` (unit tests, no disk):**

- Template parsing: each field, each operation, both operations together,
  multiple templates in one name, whitespace handling.
- Unknown field, unknown operation, and malformed template each produce
  the right skip reason.
- An unresolved field in a directory drops the subtree; in a file drops
  just the file.
- A resolved name with a path separator, or an empty resolved name, is
  skipped as `InvalidName`.
- `product` / `trial` derivation, including a study code with no `-`.
- Step ordering: parents before children.
- `reference` and `migration.directory` resolution against `rootpath`.
- Filter matching: `*.sas7bdat`, `docx`, `.docx`, and a non-matching
  pattern.
- An empty `rootpath` is the one `ScaffoldError`.

**Desktop applier (unit tests against a temp dir):**

- Creates the directory tree.
- Creates the root when it is missing.
- Skips an existing directory but still creates its missing children.
- Skips an existing file without overwriting it.
- Copies a reference file and renames it to the resolved `entry_name`.
- Skips a file node with no reference, creating nothing.
- Copies migration files, honouring the filter, ignoring subdirectories,
  and skipping files that already exist at the target.
- Preserves the modification time of a copied file.
- The report lists what was created and why things were skipped.

**Frontend:**

- The dialog calls the command with the right arguments.
- The report renders the created and skipped counts and the skip reasons.
- An error keeps the dialog open and shows the message.

## Out of scope

- Recording the generated folder on the server (no endpoint exists; none is
  needed for this feature).
- Creating a project-level structure as a side effect of initializing.
- Deleting or moving anything that already exists.
- Overwriting existing files.
