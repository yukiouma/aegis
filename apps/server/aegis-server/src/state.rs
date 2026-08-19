//! Shared state injected into every handler via `axum::extract::State`.

use std::sync::Arc;

/// Shared state injected into every handler.
///
/// Cloned per worker task (axum's `State<T>: Clone` requires it);
/// all services are `Arc`, so the clone is cheap.
#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<dyn apis::auth::AuthService>,
    pub user: Arc<dyn apis::user::UserService>,
    pub project: Arc<dyn apis::project::ProjectService>,
    pub terminology: Arc<dyn apis::terminology::TerminologyService>,
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Shared test doubles used by every per-module `tests` block.
    //!
    //! Each `tests` block that builds an `AppState` literally needs a
    //! stand-in implementation for every service field. To avoid
    //! copy-pasting 100-line `#[async_trait] impl` blocks across six
    //! test modules, the common null doubles live here.

    use async_trait::async_trait;

    /// Null terminology service for tests that don't exercise the
    /// terminology surface. Every method panics with
    /// `unimplemented!()` — the corresponding handler is never reached
    /// because the test either uses a different route or short-
    /// circuits on auth / role checks before the usecase is invoked.
    #[derive(Clone)]
    pub(crate) struct NullTerminologyService;

    #[async_trait]
    impl apis::terminology::TerminologyService for NullTerminologyService {
        async fn create_version(
            &self,
            _req: apis::terminology::CreateTerminologyVersionRequest,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn list_versions(
            &self,
        ) -> Result<
            Vec<apis::terminology::TerminologyVersionView>,
            apis::terminology::TerminologyApiError,
        > {
            unimplemented!()
        }
        async fn get_version_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn update_version(
            &self,
            _req: apis::terminology::UpdateTerminologyVersionRequest,
        ) -> Result<apis::terminology::TerminologyVersionView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn delete_version(
            &self,
            _id: i64,
        ) -> Result<(), apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn create_code_list(
            &self,
            _req: apis::terminology::CreateCodeListRequest,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn list_code_lists(
            &self,
            _version_id: i64,
        ) -> Result<Vec<apis::terminology::CodeListView>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn get_code_list_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn update_code_list(
            &self,
            _req: apis::terminology::UpdateCodeListRequest,
        ) -> Result<apis::terminology::CodeListView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn delete_code_list(
            &self,
            _id: i64,
        ) -> Result<(), apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn search_code_lists(
            &self,
            _query: apis::terminology::CodeListSearchQuery,
        ) -> Result<Vec<apis::terminology::CodeListSearchHit>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn create_code_item(
            &self,
            _req: apis::terminology::CreateCodeItemRequest,
        ) -> Result<apis::terminology::CodeItemView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn list_code_items(
            &self,
            _codelist_id: i64,
        ) -> Result<Vec<apis::terminology::CodeItemView>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn list_code_items_by_version_and_code(
            &self,
            _version_id: i64,
            _code: &str,
        ) -> Result<Vec<apis::terminology::CodeItemView>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn update_code_item(
            &self,
            _req: apis::terminology::UpdateCodeItemRequest,
        ) -> Result<apis::terminology::CodeItemView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn delete_code_item(
            &self,
            _id: i64,
        ) -> Result<(), apis::terminology::TerminologyApiError> {
            unimplemented!()
        }
        async fn search_code_items(
            &self,
            _query: apis::terminology::CodeItemSearchQuery,
        ) -> Result<Vec<apis::terminology::CodeItemSearchHit>, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn batch_create_code_items(
            &self,
            _req: apis::terminology::BatchCreateCodeItemsRequest,
        ) -> Result<apis::terminology::BatchCreateCodeItemsResponse, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
    }
}
