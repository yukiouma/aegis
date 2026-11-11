//! The initialize-project-folder command.
//!
//! Unlike every other command in this crate, this one writes to the
//! local disk: it fetches the structure's tree over HTTP, plans the
//! scaffold with the `scaffolding` crate, and applies it. The rules
//! live in that crate; this file only wires them together.

use tracing::Instrument;

use crate::http::client::TRACE_ID;
use crate::http::dto::ApiError;
use crate::http::structures::{self, NodeViewResponse};
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
                let context =
                    Context::new(&study, branch.as_deref(), chrono::Local::now().date_naive());
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

#[cfg(test)]
mod tests {
    //! Wire-shape tests. Tauri routes JSON keys to function parameters
    //! by camelCase name, so the frontend wrapper must send exactly:
    //!   initialize_project_folder: { structureId, study, branch }

    use crate::commands::test_support::assert_invoke_keys;

    const EXPECTED_INVOKE_KEYS: &[(&str, &[&str])] = &[(
        "initialize_project_folder",
        &["structureId", "study", "branch"],
    )];

    #[test]
    fn invoke_keys_match_the_documented_shape() {
        assert_invoke_keys(include_str!("scaffold.rs"), EXPECTED_INVOKE_KEYS);
    }
}
