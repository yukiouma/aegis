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
    pub domain_model: Arc<dyn apis::domain_model::DomainModelService>,
    pub crf: Arc<dyn apis::crf::CrfService>,
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
            _query: apis::terminology::CodeListListQuery,
        ) -> Result<
            apis::terminology::Page<apis::terminology::CodeListView>,
            apis::terminology::TerminologyApiError,
        > {
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
        async fn create_code_item(
            &self,
            _req: apis::terminology::CreateCodeItemRequest,
        ) -> Result<apis::terminology::CodeItemView, apis::terminology::TerminologyApiError>
        {
            unimplemented!()
        }
        async fn list_code_items(
            &self,
            _query: apis::terminology::CodeItemListQuery,
        ) -> Result<
            apis::terminology::Page<apis::terminology::CodeItemView>,
            apis::terminology::TerminologyApiError,
        > {
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
        async fn batch_create_code_items(
            &self,
            _req: apis::terminology::BatchCreateCodeItemsRequest,
        ) -> Result<
            apis::terminology::BatchCreateCodeItemsResponse,
            apis::terminology::TerminologyApiError,
        > {
            unimplemented!()
        }
    }

    /// Null domain-model service for tests that don't exercise the
    /// SDTM domain-model surface. Every method panics with
    /// `unimplemented!()` — the corresponding handler is never reached
    /// because the test either uses a different route or short-
    /// circuits on auth / role checks before the usecase is invoked.
    #[derive(Clone)]
    pub(crate) struct NullDomainModelService;

    #[async_trait]
    impl apis::domain_model::DomainModelService for NullDomainModelService {
        async fn create_version(
            &self,
            _req: apis::domain_model::CreateSdtmVersionRequest,
        ) -> Result<apis::domain_model::SdtmVersionView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn list_versions(
            &self,
        ) -> Result<apis::domain_model::SdtmVersionList, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn update_version(
            &self,
            _req: apis::domain_model::UpdateSdtmVersionRequest,
        ) -> Result<apis::domain_model::SdtmVersionView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn delete_version(
            &self,
            _id: i64,
        ) -> Result<(), apis::domain_model::DomainModelApiError> {
            unimplemented!()
        }
        async fn create_domain(
            &self,
            _req: apis::domain_model::CreateSdtmDomainRequest,
        ) -> Result<apis::domain_model::SdtmDomainView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn get_domain_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::domain_model::SdtmDomainView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn list_domains_by_version(
            &self,
            _version_id: i64,
        ) -> Result<apis::domain_model::SdtmDomainList, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn update_domain(
            &self,
            _req: apis::domain_model::UpdateSdtmDomainRequest,
        ) -> Result<apis::domain_model::SdtmDomainView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn delete_domain(
            &self,
            _id: i64,
        ) -> Result<(), apis::domain_model::DomainModelApiError> {
            unimplemented!()
        }
        async fn create_variable(
            &self,
            _req: apis::domain_model::CreateSdtmVariableRequest,
        ) -> Result<apis::domain_model::SdtmVariableView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn get_variable_by_id(
            &self,
            _id: i64,
        ) -> Result<apis::domain_model::SdtmVariableView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn list_variables_by_domain(
            &self,
            _domain_id: i64,
        ) -> Result<apis::domain_model::SdtmVariableList, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn update_variable(
            &self,
            _req: apis::domain_model::UpdateSdtmVariableRequest,
        ) -> Result<apis::domain_model::SdtmVariableView, apis::domain_model::DomainModelApiError>
        {
            unimplemented!()
        }
        async fn delete_variable(
            &self,
            _id: i64,
        ) -> Result<(), apis::domain_model::DomainModelApiError> {
            unimplemented!()
        }
    }

    /// Null CRF service for tests that don't exercise the
    /// Case Report Form surface. Every method panics with
    /// `unimplemented!()` — the corresponding handler is never
    /// reached because the test either uses a different route or
    /// short-circuits on auth / role checks before the usecase is
    /// invoked.
    #[derive(Clone)]
    pub(crate) struct NullCrfService;

    #[async_trait]
    impl apis::crf::CrfService for NullCrfService {
        async fn create_version(
            &self,
            _req: apis::crf::CreateCrfVersionRequest,
        ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_version_by_id(
            &self,
            _req: apis::crf::GetCrfVersionByIdRequest,
        ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_versions_by_project(
            &self,
            _req: apis::crf::ListCrfVersionsByProjectRequest,
        ) -> Result<Vec<apis::crf::CrfVersionView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_version(
            &self,
            _req: apis::crf::UpdateCrfVersionRequest,
        ) -> Result<apis::crf::CrfVersionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_version(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_form(
            &self,
            _req: apis::crf::CreateCrfFormRequest,
        ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_form_by_id(
            &self,
            _req: apis::crf::GetCrfFormByIdRequest,
        ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_forms_by_version(
            &self,
            _req: apis::crf::ListCrfFormsByVersionRequest,
        ) -> Result<Vec<apis::crf::CrfFormView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_form(
            &self,
            _req: apis::crf::UpdateCrfFormRequest,
        ) -> Result<apis::crf::CrfFormView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_form(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_item(
            &self,
            _req: apis::crf::CreateCrfItemRequest,
        ) -> Result<apis::crf::CrfItemView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_item_by_id(
            &self,
            _req: apis::crf::GetCrfItemByIdRequest,
        ) -> Result<apis::crf::CrfItemView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_items_by_form(
            &self,
            _req: apis::crf::ListCrfItemsByFormRequest,
        ) -> Result<Vec<apis::crf::CrfItemView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_item(
            &self,
            _req: apis::crf::UpdateCrfItemRequest,
        ) -> Result<apis::crf::CrfItemView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_item(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_option(
            &self,
            _req: apis::crf::CreateCrfOptionRequest,
        ) -> Result<apis::crf::CrfOptionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_option_by_id(
            &self,
            _req: apis::crf::GetCrfOptionByIdRequest,
        ) -> Result<apis::crf::CrfOptionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_options_by_item(
            &self,
            _req: apis::crf::ListCrfOptionsByItemRequest,
        ) -> Result<Vec<apis::crf::CrfOptionView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_option(
            &self,
            _req: apis::crf::UpdateCrfOptionRequest,
        ) -> Result<apis::crf::CrfOptionView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_option(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_unit(
            &self,
            _req: apis::crf::CreateCrfUnitRequest,
        ) -> Result<apis::crf::CrfUnitView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_unit_by_id(
            &self,
            _req: apis::crf::GetCrfUnitByIdRequest,
        ) -> Result<apis::crf::CrfUnitView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_units_by_item(
            &self,
            _req: apis::crf::ListCrfUnitsByItemRequest,
        ) -> Result<Vec<apis::crf::CrfUnitView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_unit(
            &self,
            _req: apis::crf::UpdateCrfUnitRequest,
        ) -> Result<apis::crf::CrfUnitView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_unit(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_domain_annotation(
            &self,
            _req: apis::crf::CreateDomainAnnotationRequest,
        ) -> Result<apis::crf::DomainAnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_domain_annotation_by_id(
            &self,
            _req: apis::crf::GetDomainAnnotationByIdRequest,
        ) -> Result<apis::crf::DomainAnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_domain_annotations_by_form(
            &self,
            _req: apis::crf::ListDomainAnnotationsByFormRequest,
        ) -> Result<Vec<apis::crf::DomainAnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_domain_annotation(
            &self,
            _req: apis::crf::UpdateDomainAnnotationRequest,
        ) -> Result<apis::crf::DomainAnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_domain_annotation(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn create_annotation(
            &self,
            _req: apis::crf::CreateAnnotationRequest,
        ) -> Result<apis::crf::AnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn get_annotation_by_id(
            &self,
            _req: apis::crf::GetAnnotationByIdRequest,
        ) -> Result<apis::crf::AnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_annotations_by_form(
            &self,
            _req: apis::crf::ListAnnotationsByFormRequest,
        ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_annotations_by_item(
            &self,
            _req: apis::crf::ListAnnotationsByItemRequest,
        ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_annotations_by_option(
            &self,
            _req: apis::crf::ListAnnotationsByOptionRequest,
        ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn list_annotations_by_unit(
            &self,
            _req: apis::crf::ListAnnotationsByUnitRequest,
        ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn update_annotation(
            &self,
            _req: apis::crf::UpdateAnnotationRequest,
        ) -> Result<apis::crf::AnnotationView, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn delete_annotation(&self, _id: i64) -> Result<(), apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_forms_by_version(
            &self,
            _req: apis::crf::SearchCrfFormsByVersionRequest,
        ) -> Result<Vec<apis::crf::CrfFormView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_items_by_version(
            &self,
            _req: apis::crf::SearchCrfItemsByVersionRequest,
        ) -> Result<Vec<apis::crf::CrfItemView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_options_by_version(
            &self,
            _req: apis::crf::SearchCrfOptionsByVersionRequest,
        ) -> Result<Vec<apis::crf::CrfOptionView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_units_by_version(
            &self,
            _req: apis::crf::SearchCrfUnitsByVersionRequest,
        ) -> Result<Vec<apis::crf::CrfUnitView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_domain_annotations_by_version(
            &self,
            _req: apis::crf::SearchDomainAnnotationsByVersionRequest,
        ) -> Result<Vec<apis::crf::DomainAnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
        async fn search_annotations_by_version(
            &self,
            _req: apis::crf::SearchAnnotationsByVersionRequest,
        ) -> Result<Vec<apis::crf::AnnotationView>, apis::crf::CrfApiError> {
            unimplemented!()
        }
    }
}
