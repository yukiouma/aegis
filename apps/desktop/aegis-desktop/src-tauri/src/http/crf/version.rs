//! HTTP functions under `/api/crf/projects/{project_code}/versions`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::http::client::HttpClient;
use crate::http::crf::form::{
    self as form_http, BulkCreateCrfFormItemInput, BulkCreateCrfFormRequest, CreateCrfFormRequest,
    CreateCrfItemRequest, CreateCrfOptionRequest, CreateCrfUnitRequest,
    CrfItemKind as WireCrfItemKind,
};
use crate::http::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfVersionViewResponse {
    pub id: i64,
    pub project_code: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrfVersionListResponse {
    pub versions: Vec<CrfVersionViewResponse>,
}

/// Local error taxonomy for the `import_als` orchestrator.
///
/// Pre-validation is a fast-fail mirror of the server-side rules in
/// `lib/crates/crf/src/domain/crf_bulk_form.rs`; we surface violations
/// as `ApiError::Parse` so the page renders them through the same Snackbar
/// path as parse failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AlsImportError {
    #[error("form #{form_index}: {target} must not be empty")]
    Empty {
        target: &'static str,
        form_index: usize,
    },

    #[error("form #{form_index} item '{item_code}': {field} must not be empty")]
    EmptyItem {
        form_index: usize,
        item_code: String,
        field: &'static str,
    },

    #[error("form #{form_index} item '{item_code}': kind={kind} requires non-empty {field}")]
    KindShapeViolation {
        form_index: usize,
        item_code: String,
        kind: String,
        field: &'static str,
    },

    #[error("I/O error: {0}")]
    Io(String),
}

impl AlsImportError {
    pub(crate) fn from_io(e: std::io::Error) -> Self {
        AlsImportError::Io(e.to_string())
    }
}

impl From<AlsImportError> for ApiError {
    fn from(err: AlsImportError) -> Self {
        ApiError::Parse {
            message: err.to_string(),
        }
    }
}

/// Wire mirror of `lib/crates/apis/src/crf.rs::CrfItemKind`. Kept local
/// to this module to avoid pulling in `http::crf::form` for what is
/// only a string-tagged translator. `Label` exists in the wire shape
/// but is never constructed here because als-resolver does not emit
/// label controls — the variant is still listed so the wire→local
/// mapping in `From<WireCrfItemKind>` covers every server variant.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrfItemKind {
    Text,
    Selection,
    Checkbox,
    Datetime,
    Label,
}

impl CrfItemKind {
    pub(crate) fn as_wire(self) -> &'static str {
        match self {
            CrfItemKind::Text => "text",
            CrfItemKind::Selection => "selection",
            CrfItemKind::Checkbox => "checkbox",
            CrfItemKind::Datetime => "datetime",
            CrfItemKind::Label => "label",
        }
    }
}

fn control_type_to_kind(
    c: entities::project::ControlType,
    not_variable: Option<bool>,
) -> CrfItemKind {
    use entities::project::ControlType as C;
    // als-resolver marks an item with `not_variable=true` when it is a
    // static label rather than a captured variable. Surface that as
    // `CrfItemKind::Label` here so the wire shape matches what the
    // server (and the detail page) already understand.
    if not_variable == Some(true) {
        return CrfItemKind::Label;
    }
    match c {
        C::TEXT => CrfItemKind::Text,
        C::DATETIME => CrfItemKind::Datetime,
        C::SELECTION => CrfItemKind::Selection,
        C::CHECKBOX => CrfItemKind::Checkbox,
    }
}

impl From<CrfItemKind> for WireCrfItemKind {
    fn from(k: CrfItemKind) -> Self {
        match k {
            CrfItemKind::Text => WireCrfItemKind::Text,
            CrfItemKind::Selection => WireCrfItemKind::Selection,
            CrfItemKind::Checkbox => WireCrfItemKind::Checkbox,
            CrfItemKind::Datetime => WireCrfItemKind::Datetime,
            CrfItemKind::Label => WireCrfItemKind::Label,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdcType {
    Rave,
    EcollectV6,
    EcollectLegacy,
}

fn parse_als_dispatch<R: std::io::Read + std::io::Seek>(
    edc: EdcType,
    reader: R,
) -> Result<als_resolver::Project, als_resolver::AlsParseError> {
    match edc {
        EdcType::Rave => als_resolver::parse_rave_als(reader),
        EdcType::EcollectV6 => als_resolver::parse_ecollect_v6_als(reader),
        EdcType::EcollectLegacy => als_resolver::parse_ecollect_legacy_als(reader),
    }
}

fn pre_validate(project: &als_resolver::Project) -> Result<(), Vec<AlsImportError>> {
    let mut errs = Vec::new();

    for (form_index, f) in project.forms.iter().enumerate() {
        if f.name.trim().is_empty() {
            errs.push(AlsImportError::Empty {
                target: "form code",
                form_index,
            });
        }
        if f.description.trim().is_empty() {
            errs.push(AlsImportError::Empty {
                target: "form name",
                form_index,
            });
        }

        for item in &f.items {
            if item.name.trim().is_empty() {
                errs.push(AlsImportError::EmptyItem {
                    form_index,
                    item_code: item.name.clone(),
                    field: "code",
                });
            }
            if item.label.trim().is_empty() {
                errs.push(AlsImportError::EmptyItem {
                    form_index,
                    item_code: item.name.clone(),
                    field: "name",
                });
            }
            let opts = item.item_option.as_deref().unwrap_or(&[]);
            for opt in opts {
                if opt.option_display.trim().is_empty() {
                    errs.push(AlsImportError::EmptyItem {
                        form_index,
                        item_code: item.name.clone(),
                        field: "option value",
                    });
                }
            }
            if let Some(u) = &item.item_unit {
                if u.value.trim().is_empty() {
                    errs.push(AlsImportError::EmptyItem {
                        form_index,
                        item_code: item.name.clone(),
                        field: "unit value",
                    });
                }
            }

            let kind = control_type_to_kind(item.control_type, item.not_variable);
            match kind {
                CrfItemKind::Selection if opts.is_empty() => {
                    errs.push(AlsImportError::KindShapeViolation {
                        form_index,
                        item_code: item.name.clone(),
                        kind: kind.as_wire().to_string(),
                        field: "options",
                    });
                }
                CrfItemKind::Text | CrfItemKind::Datetime | CrfItemKind::Label
                    if !opts.is_empty() =>
                {
                    errs.push(AlsImportError::KindShapeViolation {
                        form_index,
                        item_code: item.name.clone(),
                        kind: kind.as_wire().to_string(),
                        field: "options",
                    });
                }
                _ => {}
            }
        }
    }

    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

pub async fn list_by_project(
    c: &HttpClient,
    project_code: &str,
) -> Result<CrfVersionListResponse, ApiError> {
    c.request(
        reqwest::Method::GET,
        &format!("/api/crf/projects/{project_code}/versions"),
        None::<&()>,
    )
    .await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCrfVersionRequest {
    pub name: String,
}

pub async fn create(
    c: &HttpClient,
    project_code: &str,
    body: CreateCrfVersionRequest,
) -> Result<CrfVersionViewResponse, ApiError> {
    c.request(
        reqwest::Method::POST,
        &format!("/api/crf/projects/{project_code}/versions"),
        Some(&body),
    )
    .await
}

/// Top-level orchestrator for `commands::crf::version::import_als`.
///
/// Steps:
/// 1. Open the user-supplied file and parse it via `parse_als_dispatch`
///    off the runtime thread.
/// 2. Pre-validate the parsed `Project` against the same kind-shape
///    and non-empty rules the server's `bulk_create_form` enforces.
///    Failures here abort *before* any HTTP call — no DB writes happen.
/// 3. POST `/api/crf/projects/{project_code}/versions` to get back a
///    fresh version id.
/// 4. For each form in the project, POST
///    `/api/crf/versions/{id}/forms/bulk` with the items subtree.
///
/// Partial-failure semantics: the orchestrator does NOT roll back
/// forms whose bulk-create succeeded when a later one fails; it
/// surfaces the first error and the user re-runs the import. The
/// pre-validate pass exists precisely to catch shape problems before
/// any DB writes so this almost never fires.
pub async fn import_als(
    c: &HttpClient,
    project_code: &str,
    name: &str,
    filepath: &str,
    edc_type: EdcType,
) -> Result<CrfVersionViewResponse, ApiError> {
    use std::io::BufReader;

    // 1. parse off-thread
    let filepath_owned = filepath.to_string();
    let parsed: Result<als_resolver::Project, ApiError> =
        tokio::task::spawn_blocking(move || -> Result<als_resolver::Project, AlsImportError> {
            let file = std::fs::File::open(&filepath_owned).map_err(AlsImportError::from_io)?;
            parse_als_dispatch(edc_type, BufReader::new(file))
                .map_err(|e| AlsImportError::Io(e.to_string()))
        })
        .await
        .map_err(|e| ApiError::Parse {
            message: format!("join error: {e}"),
        })?
        .map_err(|e: AlsImportError| ApiError::Parse {
            message: e.to_string(),
        });

    let parsed = parsed?;

    // 2. pre-validate
    if let Err(errs) = pre_validate(&parsed) {
        let messages: Vec<String> = errs.iter().map(|e| e.to_string()).collect();
        return Err(ApiError::Parse {
            message: format!(
                "{} validation error(s): {}",
                messages.len(),
                messages.join("; ")
            ),
        });
    }

    // 3. create version
    let version_view = create(
        c,
        project_code,
        CreateCrfVersionRequest {
            name: name.to_string(),
        },
    )
    .await?;
    if version_view.id <= 0 {
        return Err(ApiError::Parse {
            message: format!(
                "invalid version_id {} from create_version response",
                version_view.id
            ),
        });
    }

    // 4. insert each form via bulk_create
    for f in &parsed.forms {
        let items: Vec<BulkCreateCrfFormItemInput> = f
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let opts: &[entities::project::ItemOption] =
                    item.item_option.as_deref().unwrap_or(&[]);
                let units: Vec<CreateCrfUnitRequest> = item
                    .item_unit
                    .as_ref()
                    .map(|u| CreateCrfUnitRequest {
                        value: u.value.clone(),
                        not_submitted: false,
                    })
                    .into_iter()
                    .collect();
                BulkCreateCrfFormItemInput {
                    item: CreateCrfItemRequest {
                        code: item.name.clone(),
                        name: item.label.clone(),
                        kind: control_type_to_kind(item.control_type, item.not_variable).into(),
                        order: i as i32,
                        not_submitted: false,
                    },
                    options: opts
                        .iter()
                        .map(|o| CreateCrfOptionRequest {
                            value: o.option_display.clone(),
                            not_submitted: false,
                        })
                        .collect(),
                    units,
                }
            })
            .collect();
        let _: form_http::BulkCreateCrfFormResponse = form_http::bulk_create(
            c,
            version_view.id,
            BulkCreateCrfFormRequest {
                form: CreateCrfFormRequest {
                    code: f.name.clone(),
                    name: f.description.clone(),
                    order: f.order,
                    not_submitted: false,
                },
                items,
            },
        )
        .await?;
    }

    Ok(version_view)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::http::client::{HttpClient, MemoryStore, TokenStore};

    fn client(server: &MockServer) -> HttpClient {
        let store = Arc::new(MemoryStore::default());
        let _ = store.set_access_token("AT");
        let _ = store.set_refresh_token("RT");
        HttpClient::new(server.uri(), store)
    }

    fn tmpfile_als_path() -> std::path::PathBuf {
        let path = std::env::temp_dir().join("aegis-desktop-import-als-test.als");
        std::fs::write(&path, b"<root/>").unwrap();
        path
    }

    #[tokio::test]
    async fn list_by_project_returns_versions() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/crf/projects/abc/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "versions": [{
                    "id": 1, "projectCode": "abc", "name": "v1",
                    "createdAt": "2026-01-01T00:00:00Z",
                    "updatedAt": "2026-01-02T00:00:00Z"
                }]
            })))
            .mount(&server)
            .await;
        let resp = list_by_project(&client(&server), "abc").await.unwrap();
        assert_eq!(resp.versions.len(), 1);
        assert_eq!(resp.versions[0].id, 1);
        assert_eq!(resp.versions[0].project_code, "abc");
        assert_eq!(resp.versions[0].name, "v1");
        assert_eq!(
            resp.versions[0].created_at,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
        );
    }

    #[tokio::test]
    async fn create_returns_version_view() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/crf/projects/P1/versions"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 7, "projectCode": "P1", "name": "v1",
                "createdAt": "2026-01-01T00:00:00Z",
                "updatedAt": "2026-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;
        let resp = create(
            &client(&server),
            "P1",
            CreateCrfVersionRequest {
                name: "v1".to_string(),
            },
        )
        .await
        .unwrap();
        assert_eq!(resp.id, 7);
        assert_eq!(resp.name, "v1");
        assert_eq!(resp.project_code, "P1");
    }

    #[tokio::test]
    async fn import_als_returns_parse_error_on_empty_file() {
        // Empty ALS file aborts at the parser; no mock is mounted for
        // /api/crf/projects/.../versions — if the orchestrator reaches
        // that call, mock will 404 and the test fails. This pins that
        // parse failure aborts BEFORE any HTTP call (no auto-rollback
        // needed since nothing was written).
        let server = MockServer::start().await;
        let res = import_als(
            &client(&server),
            "P1",
            "v1",
            tmpfile_als_path().to_str().unwrap(),
            EdcType::Rave,
        )
        .await;
        match res.unwrap_err() {
            ApiError::Parse { message } => {
                assert!(!message.is_empty(), "Parse message should not be empty")
            }
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn als_import_error_wraps_to_api_error_parse() {
        let err = AlsImportError::KindShapeViolation {
            form_index: 0,
            item_code: "X".to_string(),
            kind: "selection".to_string(),
            field: "options",
        };
        let api: ApiError = err.into();
        match api {
            ApiError::Parse { message } => {
                assert!(
                    message.contains("selection") && message.contains("X"),
                    "got: {message}"
                );
            }
            other => panic!("expected Parse variant, got {other:?}"),
        }
    }

    #[test]
    fn control_type_to_kind_covers_all_variants() {
        use entities::project::ControlType;
        assert_eq!(
            control_type_to_kind(ControlType::TEXT, None),
            CrfItemKind::Text
        );
        assert_eq!(
            control_type_to_kind(ControlType::DATETIME, None),
            CrfItemKind::Datetime
        );
        assert_eq!(
            control_type_to_kind(ControlType::SELECTION, None),
            CrfItemKind::Selection
        );
        assert_eq!(
            control_type_to_kind(ControlType::CHECKBOX, None),
            CrfItemKind::Checkbox
        );
        // `not_variable = Some(true)` overrides the control type and
        // surfaces a label — covers the als-resolver case where a
        // captured field has been re-emitted as a static label.
        assert_eq!(
            control_type_to_kind(ControlType::TEXT, Some(true)),
            CrfItemKind::Label
        );
        assert_eq!(
            control_type_to_kind(ControlType::CHECKBOX, Some(true)),
            CrfItemKind::Label
        );
        // `not_variable = Some(false)` mirrors `None` and must NOT
        // promote the item to a label.
        assert_eq!(
            control_type_to_kind(ControlType::TEXT, Some(false)),
            CrfItemKind::Text
        );
    }

    #[test]
    fn edc_type_dispatch_picks_correct_parser() {
        use std::io::Cursor;
        // The dispatch boundary is the only thing under test; each
        // parser will return Err on empty input, but the dispatch must
        // reach all three branches without panicking.
        let _ = parse_als_dispatch(EdcType::Rave, Cursor::new(Vec::<u8>::new()));
        let _ = parse_als_dispatch(EdcType::EcollectV6, Cursor::new(Vec::<u8>::new()));
        let _ = parse_als_dispatch(EdcType::EcollectLegacy, Cursor::new(Vec::<u8>::new()));
    }

    #[test]
    fn crf_item_kind_as_wire_matches_shared_types_ts() {
        assert_eq!(CrfItemKind::Text.as_wire(), "text");
        assert_eq!(CrfItemKind::Selection.as_wire(), "selection");
        assert_eq!(CrfItemKind::Checkbox.as_wire(), "checkbox");
        assert_eq!(CrfItemKind::Datetime.as_wire(), "datetime");
        assert_eq!(CrfItemKind::Label.as_wire(), "label");
    }

    #[test]
    fn pre_validate_rejects_selection_with_empty_options() {
        let p = selection_with_zero_options();
        let errs = pre_validate(&p).unwrap_err();
        assert_eq!(
            errs,
            vec![AlsImportError::KindShapeViolation {
                form_index: 0,
                item_code: "BAD".into(),
                kind: "selection".into(),
                field: "options",
            }],
        );
    }

    #[test]
    fn pre_validate_rejects_text_with_options() {
        let p = text_with_one_option();
        let errs = pre_validate(&p).unwrap_err();
        assert!(matches!(
            errs[0],
            AlsImportError::KindShapeViolation {
                field: "options",
                ..
            }
        ));
    }

    #[test]
    fn pre_validate_rejects_empty_form_code() {
        let p = form_with_whitespace_code();
        let errs = pre_validate(&p).unwrap_err();
        assert!(matches!(
            errs[0],
            AlsImportError::Empty {
                target: "form code",
                form_index: 0
            }
        ));
    }

    #[test]
    fn pre_validate_accepts_zero_items() {
        let p = form_with_no_items();
        assert!(pre_validate(&p).is_ok());
    }

    #[test]
    fn pre_validate_isolates_per_form_errors() {
        let p = three_forms_middle_broken();
        let errs = pre_validate(&p).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(matches!(
            errs[0],
            AlsImportError::Empty { form_index: 1, .. }
        ));
    }

    // --- helpers ---

    fn selection_with_zero_options() -> als_resolver::Project {
        use entities::project::{CRFForm, CRFItem, ControlType};
        als_resolver::Project {
            forms: vec![CRFForm {
                name: "F1".into(),
                description: "Form 1".into(),
                order: 0,
                items: vec![CRFItem {
                    name: "BAD".into(),
                    label: "Bad".into(),
                    item_option: None,
                    annotations: vec![],
                    format: String::new(),
                    control_type: ControlType::SELECTION,
                    item_unit: None,
                    not_variable: None,
                }],
                domains: vec![],
                annotations: vec![],
            }],
            visit: vec![],
        }
    }

    fn text_with_one_option() -> als_resolver::Project {
        use entities::project::{CRFForm, CRFItem, ControlType, ItemOption};
        als_resolver::Project {
            forms: vec![CRFForm {
                name: "F1".into(),
                description: "F1".into(),
                order: 0,
                items: vec![CRFItem {
                    name: "TXT".into(),
                    label: "T".into(),
                    item_option: Some(vec![ItemOption {
                        option_display: "x".into(),
                        annotations: vec![],
                    }]),
                    annotations: vec![],
                    format: String::new(),
                    control_type: ControlType::TEXT,
                    item_unit: None,
                    not_variable: None,
                }],
                domains: vec![],
                annotations: vec![],
            }],
            visit: vec![],
        }
    }

    fn form_with_whitespace_code() -> als_resolver::Project {
        use entities::project::CRFForm;
        als_resolver::Project {
            forms: vec![CRFForm {
                name: "   ".into(),
                description: "ok".into(),
                order: 0,
                items: vec![],
                domains: vec![],
                annotations: vec![],
            }],
            visit: vec![],
        }
    }

    fn form_with_no_items() -> als_resolver::Project {
        use entities::project::CRFForm;
        als_resolver::Project {
            forms: vec![CRFForm {
                name: "F1".into(),
                description: "F1".into(),
                order: 0,
                items: vec![],
                domains: vec![],
                annotations: vec![],
            }],
            visit: vec![],
        }
    }

    fn three_forms_middle_broken() -> als_resolver::Project {
        use entities::project::CRFForm;
        let good = || CRFForm {
            name: "ok".into(),
            description: "ok".into(),
            order: 0,
            items: vec![],
            domains: vec![],
            annotations: vec![],
        };
        let broken = CRFForm {
            name: "   ".into(),
            description: "bad".into(),
            order: 1,
            items: vec![],
            domains: vec![],
            annotations: vec![],
        };
        als_resolver::Project {
            forms: vec![good(), broken, good()],
            visit: vec![],
        }
    }
}
