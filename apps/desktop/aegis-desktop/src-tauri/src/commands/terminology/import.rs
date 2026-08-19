//! Tauri command shim for bulk terminology import.
//!
//! Parses an xls/xlsx workbook, then orchestrates the existing
//! per-resource HTTP wrappers to create the version, its code lists,
//! and the items of each code list.

use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::{ApiError, TerminologyKind};
use crate::http::terminology::code_item::{
    self, BatchCodeItemEntry, BatchCreateCodeItemsRequest,
};
use crate::http::terminology::code_list::{self, CreateCodeListRequest};
use crate::http::terminology::version::{self, CreateTerminologyVersionRequest,
    TerminologyVersionViewResponse};

#[tauri::command]
pub async fn import_terminology(
    client: State<'_, HttpClient>,
    kind: TerminologyKind,
    filepath: String,
) -> Result<TerminologyVersionViewResponse, ApiError> {
    // 1. Parse the workbook off-thread (calamine is sync / CPU-bound).
    let parsed = tokio::task::spawn_blocking(move || terminology::from_path(&filepath))
        .await
        .map_err(|e| ApiError::Parse {
            message: format!("join error: {e}"),
        })?
        .map_err(|e| ApiError::Parse {
            message: e.to_string(),
        })?;

    // 2. Create the version.
    let version_view = version::create(
        &client,
        CreateTerminologyVersionRequest {
            kind,
            name: parsed.name,
        },
    )
    .await?;

    // 3. For each code list, create the list and batch-create its items.
    for cl in parsed.codelist {
        let cl_view = code_list::create(
            &client,
            CreateCodeListRequest {
                version_id: version_view.id,
                code: cl.code,
                extensible: cl.extensible,
                name: cl.name,
                submission_value: cl.submission_value,
                synonym: cl.synonym,
                definition: cl.definition,
                nci_preferred_term: cl.nci_preferred_term,
            },
        )
        .await?;

        if cl.code_list.is_empty() {
            continue;
        }

        code_item::batch_create(
            &client,
            BatchCreateCodeItemsRequest {
                codelist_id: cl_view.id,
                version_id: version_view.id,
                items: cl
                    .code_list
                    .into_iter()
                    .map(|i| BatchCodeItemEntry {
                        code: i.code,
                        submission_value: i.submission_value,
                        synonym: i.synonym,
                        definition: i.definition,
                        nci_preferred_term: i.nci_preferred_term,
                    })
                    .collect(),
            },
        )
        .await?;
    }

    Ok(version_view)
}