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

    /// A context with no branch — `{{ branch }}` is then unresolved.
    fn ctx_without_branch() -> Context {
        Context::new(
            "AK001-001",
            None,
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
        let plan = build("D:\\studies", &tree, &ctx_without_branch()).unwrap();
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
        let tree = vec![dir(
            "01_raw",
            vec![file("{{ branch }}.docx", Some("D:\\sop\\spec.docx"))],
        )];
        let plan = build("D:\\studies", &tree, &ctx_without_branch()).unwrap();
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].reason, SkipReason::UnresolvedField);
        assert_eq!(
            plan.skipped[0].path,
            "D:\\studies\\01_raw\\{{ branch }}.docx"
        );
    }

    #[test]
    fn a_file_node_with_no_reference_reports_its_resolved_path() {
        let tree = vec![dir("01_raw", vec![file("spec.docx", None)])];
        let plan = build("D:\\studies", &tree, &ctx()).unwrap();
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].reason, SkipReason::NoReference);
        assert_eq!(plan.skipped[0].path, "D:\\studies\\01_raw\\spec.docx");
    }
}
