//! Executes a `scaffolding::Scaffold` against the real filesystem.
//!
//! This is the desktop's half of the initialize-project-folder command:
//! the *rules* live in the `scaffolding` crate, and this module only
//! touches the disk. Nothing here overwrites an existing file — an
//! existing target is skipped and reported.

use std::fs;
use std::path::Path;

use scaffolding::{Scaffold, SkipReason, Step};

use super::report::InitReportResponse;

/// Apply a scaffold, returning what was created and what was skipped.
pub fn apply(scaffold: &Scaffold) -> InitReportResponse {
    let mut created = Vec::new();
    let mut skipped = scaffold.skipped.clone();

    // The root is the base, not a node. It is created if missing.
    if !scaffold.root.exists() {
        if let Err(e) = fs::create_dir_all(&scaffold.root) {
            skipped.push(scaffolding::SkippedItem {
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
                    skipped.push(skipped_item(
                        path,
                        SkipReason::MigrationFailed,
                        Some(e.to_string()),
                    ));
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
            } => match fs::read_dir(from_dir) {
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
                        let target = to_dir.join(&*name);
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
            },
        }
    }

    InitReportResponse {
        root: scaffold.root.display().to_string(),
        created,
        skipped,
    }
}

/// A skipped item.
fn skipped_item(
    path: &Path,
    reason: SkipReason,
    detail: Option<String>,
) -> scaffolding::SkippedItem {
    scaffolding::SkippedItem {
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
        let suffix = pattern.strip_prefix("*.").unwrap_or(pattern).to_lowercase();
        name.ends_with(&suffix)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use scaffolding::{Context, Migration, StructureNode};
    use std::fs;
    use std::path::Path;
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
        let tree = vec![dir("ak001", vec![dir("001", vec![dir("01_raw", vec![])])])];
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
        let copied = fs::metadata(root.join("001.docx"))
            .unwrap()
            .modified()
            .unwrap();
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
        // The root, the dir, and the one copied file.
        assert_eq!(report.created.len(), 3);
    }

    #[test]
    fn a_migration_file_already_present_is_skipped_not_overwritten() {
        let tmp = tempdir().unwrap();
        let root = tmp.path().join("studies");
        let migration = tmp.path().join("sop");
        fs::create_dir_all(&migration).unwrap();
        fs::write(migration.join("a.sas7bdat"), b"new").unwrap();
        fs::create_dir_all(root.join("01_raw")).unwrap();
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
