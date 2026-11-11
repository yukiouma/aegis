# scaffolding

Turns a **structure template** into the list of filesystem operations that
materialises it.

The crate performs **no filesystem I/O**. It resolves the `{{ … }}` templates
in a node's `entry_name` against a `Context` and produces a `Scaffold` — the
ordered steps an applier would execute, plus the nodes that could not be
planned. The applier lives in the desktop app, which owns the disk.

## What it does

- **Derives the context.** `product` and `trial` come from the study code
  (`AK001-001` → `AK001` / `001`); a code with no `-` has no `trial`.
- **Resolves templates.** `{{ field }}` and `{{ field | op }}`, where `op`
  is `lowercase` or `uppercase`. Any number of templates may appear in one
  name.
- **Plans the tree.** A node's resolved name becomes a directory, a file
  copied from its `reference`, or a directory populated from its
  `migration`.

## The template syntax

```
{{ study }}                 -> AK001-001
{{ product | lowercase }}   -> ak001
{{ trial | uppercase }}     -> 001
{{ product }}-{{ trial }}   -> AK001-001
```

Fields are `study`, `product`, `trial`, `branch`, and `date`. An unknown
field or operation is an error — a typo must not silently produce a folder
named `{{ prodcut }}`.

## What it deliberately does not do

- **No filesystem access.** The crate plans; it does not touch the disk.
- **No server types.** It does not depend on `structures` or on the
  desktop — the caller maps its own tree onto [`StructureNode`].
- **No hard failures for bad names.** An unresolved field, a malformed
  template, and an invalid resolved name are all *skips*, recorded with a
  reason. The only hard error is an empty `rootpath`: there is nothing to
  scaffold relative to.

## `src/` tree

```
src/
├── lib.rs        # re-exports
├── context.rs    # Context, its fields, and their derivation
├── template.rs   # the {{ … }} parser and resolver
├── scaffold.rs   # tree -> Scaffold
├── report.rs     # SkipReason, SkippedItem
└── error.rs      # ScaffoldError
```

## Running the tests

```bash
cargo test -p scaffolding
```

The tests need no disk and no database.

## Further reading

The design this crate implements lives in
[`docs/superpowers/specs/2026-11-06-initialize-project-folder-design.md`](../../../docs/superpowers/specs/2026-11-06-initialize-project-folder-design.md).
