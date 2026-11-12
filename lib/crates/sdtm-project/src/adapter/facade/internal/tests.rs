//! Unit tests for `SdtmProjectServiceImpl`.
//!
//! Wires the adapter on top of in-memory repositories so the behaviour
//! is exercised without touching PostgreSQL. The in-memory
//! repositories are private to this test module — they are
//! intentionally not shared with the usecase tests so each layer can
//! evolve independently.
//!
//! One store backs all three repositories (a single struct implements
//! all three ports), mirroring the way the postgres adapters read the
//! same rows.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use apis::sdtm_project::{
    ConversionRule, CreateSdtmProjectDomainRequest, CreateSdtmProjectVariableRequest,
    CreateSdtmProjectVersionRequest, DomainCategory, FromCrf, SdtmProjectApiError,
    SdtmProjectService, SdtmRole, SdtmVariableCore, SdtmVariableType,
    UpdateSdtmProjectDomainRequest, UpdateSdtmProjectVariableRequest,
    UpdateSdtmProjectVersionRequest, VariableSource,
};

use crate::domain::{
    DomainError, ProjectLookup, SdtmProjectDomain, SdtmProjectDomainNew,
    SdtmProjectDomainRepository, SdtmProjectDomainUpdate, SdtmProjectVariable,
    SdtmProjectVariableNew, SdtmProjectVariableRepository, SdtmProjectVariableUpdate,
    SdtmProjectVersion, SdtmProjectVersionNew, SdtmProjectVersionRepository,
    SdtmProjectVersionUpdate,
};

use super::SdtmProjectServiceImpl;

// ---- helpers ----

fn epoch() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

// ---- in-memory store + the single repo implementing all five ports ----

#[derive(Default)]
struct InMemoryStore {
    versions: Mutex<Vec<SdtmProjectVersion>>,
    domains: Mutex<Vec<SdtmProjectDomain>>,
    variables: Mutex<Vec<SdtmProjectVariable>>,
    next_id: AtomicI64,
}

impl InMemoryStore {
    fn next_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::SeqCst) + 1
    }
}

#[derive(Clone)]
struct InMemoryRepo {
    store: Arc<InMemoryStore>,
}

impl InMemoryRepo {
    fn new(store: Arc<InMemoryStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl SdtmProjectVersionRepository for InMemoryRepo {
    async fn create(
        &self,
        input: SdtmProjectVersionNew,
    ) -> Result<SdtmProjectVersion, DomainError> {
        let mut g = self.store.versions.lock().unwrap();
        if g.iter()
            .any(|v| v.project_code == input.project_code && v.version_name == input.version_name)
        {
            return Err(DomainError::DuplicateSdtmProjectVersion {
                project_code: input.project_code,
                version_name: input.version_name,
            });
        }
        let now = epoch();
        let v = SdtmProjectVersion::for_repository(
            self.store.next_id(),
            input.project_code,
            input.version_name,
            now,
            now,
        );
        g.push(v.clone());
        Ok(v)
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectVersion, DomainError> {
        self.store
            .versions
            .lock()
            .unwrap()
            .iter()
            .find(|v| v.id == id)
            .cloned()
            .ok_or(DomainError::SdtmProjectVersionNotFound(id))
    }

    async fn list_by_project(
        &self,
        project_code: &str,
    ) -> Result<Vec<SdtmProjectVersion>, DomainError> {
        let g = self.store.versions.lock().unwrap();
        Ok(g.iter()
            .filter(|v| v.project_code == project_code)
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        input: SdtmProjectVersionUpdate,
    ) -> Result<SdtmProjectVersion, DomainError> {
        let mut g = self.store.versions.lock().unwrap();
        let v = g
            .iter_mut()
            .find(|v| v.id == input.id)
            .ok_or(DomainError::SdtmProjectVersionNotFound(input.id))?;
        if let Some(name) = input.version_name {
            v.version_name = name;
        }
        v.updated_at = epoch();
        Ok(v.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.store.versions.lock().unwrap();
        let before = g.len();
        g.retain(|v| v.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmProjectVersionNotFound(id));
        }
        Ok(())
    }
}

#[async_trait]
impl SdtmProjectDomainRepository for InMemoryRepo {
    async fn create(&self, input: SdtmProjectDomainNew) -> Result<SdtmProjectDomain, DomainError> {
        let mut g = self.store.domains.lock().unwrap();
        if g.iter().any(|d| {
            d.sdtm_project_version_id == input.sdtm_project_version_id
                && d.domain_code == input.domain_code
        }) {
            return Err(DomainError::DuplicateSdtmProjectDomain {
                version_id: input.sdtm_project_version_id,
                domain_code: input.domain_code,
            });
        }
        let now = epoch();
        let d = SdtmProjectDomain::for_repository(
            self.store.next_id(),
            input.sdtm_project_version_id,
            input.domain_code,
            input.category,
            input.descriptions,
            input.key_variables,
            now,
            now,
        );
        g.push(d.clone());
        Ok(d)
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectDomain, DomainError> {
        self.store
            .domains
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or(DomainError::SdtmProjectDomainNotFound(id))
    }

    async fn list_by_version(
        &self,
        version_id: i64,
    ) -> Result<Vec<SdtmProjectDomain>, DomainError> {
        let g = self.store.domains.lock().unwrap();
        Ok(g.iter()
            .filter(|d| d.sdtm_project_version_id == version_id)
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        input: SdtmProjectDomainUpdate,
    ) -> Result<SdtmProjectDomain, DomainError> {
        let mut g = self.store.domains.lock().unwrap();
        let d = g
            .iter_mut()
            .find(|d| d.id == input.id)
            .ok_or(DomainError::SdtmProjectDomainNotFound(input.id))?;
        if let Some(code) = input.domain_code {
            d.domain_code = code;
        }
        if let Some(category) = input.category {
            d.category = category;
        }
        if let Some(descriptions) = input.descriptions {
            d.descriptions = descriptions;
        }
        if let Some(kv) = input.key_variables {
            d.key_variables = kv;
        }
        d.updated_at = epoch();
        Ok(d.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.store.domains.lock().unwrap();
        let before = g.len();
        g.retain(|d| d.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmProjectDomainNotFound(id));
        }
        Ok(())
    }
}

#[async_trait]
impl SdtmProjectVariableRepository for InMemoryRepo {
    async fn create(
        &self,
        input: SdtmProjectVariableNew,
    ) -> Result<SdtmProjectVariable, DomainError> {
        let mut g = self.store.variables.lock().unwrap();
        if g.iter().any(|v| {
            v.sdtm_project_domain_id == input.sdtm_project_domain_id
                && v.variable_code == input.variable_code
        }) {
            return Err(DomainError::DuplicateSdtmProjectVariable {
                domain_id: input.sdtm_project_domain_id,
                variable_code: input.variable_code,
            });
        }
        let now = epoch();
        let v = SdtmProjectVariable::for_repository(
            self.store.next_id(),
            input.sdtm_project_domain_id,
            input.variable_code,
            input.variable_controlled,
            input.variable_type,
            input.variable_core,
            input.variable_role,
            input.variable_sequence,
            input.descriptions,
            input.conversion_rules,
            now,
            now,
        );
        g.push(v.clone());
        Ok(v)
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectVariable, DomainError> {
        self.store
            .variables
            .lock()
            .unwrap()
            .iter()
            .find(|v| v.id == id)
            .cloned()
            .ok_or(DomainError::SdtmProjectVariableNotFound(id))
    }

    async fn list_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<Vec<SdtmProjectVariable>, DomainError> {
        let g = self.store.variables.lock().unwrap();
        Ok(g.iter()
            .filter(|v| v.sdtm_project_domain_id == domain_id)
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        input: SdtmProjectVariableUpdate,
    ) -> Result<SdtmProjectVariable, DomainError> {
        let mut g = self.store.variables.lock().unwrap();
        let v = g
            .iter_mut()
            .find(|v| v.id == input.id)
            .ok_or(DomainError::SdtmProjectVariableNotFound(input.id))?;
        if let Some(code) = input.variable_code {
            v.variable_code = code;
        }
        if let Some(controlled) = input.variable_controlled {
            v.variable_controlled = controlled;
        }
        if let Some(t) = input.variable_type {
            v.variable_type = t;
        }
        if let Some(c) = input.variable_core {
            v.variable_core = c;
        }
        if let Some(role) = input.variable_role {
            v.variable_role = role;
        }
        if let Some(seq) = input.variable_sequence {
            v.variable_sequence = seq;
        }
        if let Some(descriptions) = input.descriptions {
            v.descriptions = descriptions;
        }
        if let Some(rules) = input.conversion_rules {
            v.conversion_rules = rules;
        }
        v.updated_at = epoch();
        Ok(v.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.store.variables.lock().unwrap();
        let before = g.len();
        g.retain(|v| v.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmProjectVariableNotFound(id));
        }
        Ok(())
    }
}

// ---- project lookup ----

#[derive(Default)]
struct FakeProjectLookup {
    known: Mutex<Vec<String>>,
}

impl FakeProjectLookup {
    fn new(known: &[&str]) -> Self {
        Self {
            known: Mutex::new(known.iter().map(|s| s.to_string()).collect()),
        }
    }
}

#[async_trait]
impl ProjectLookup for FakeProjectLookup {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        if self.known.lock().unwrap().iter().any(|c| c == code) {
            Ok(())
        } else {
            Err(DomainError::ProjectNotFound(code.to_string()))
        }
    }
}

// ---- service construction ----

type TestService =
    SdtmProjectServiceImpl<InMemoryRepo, InMemoryRepo, InMemoryRepo, FakeProjectLookup>;

fn make_service() -> TestService {
    let store = Arc::new(InMemoryStore::default());
    let repo = InMemoryRepo::new(store);
    SdtmProjectServiceImpl::from_repos(
        repo.clone(),
        repo.clone(),
        repo.clone(),
        Arc::new(FakeProjectLookup::new(&["P1"])),
    )
}

async fn seed_version(svc: &TestService) -> i64 {
    svc.create_version(CreateSdtmProjectVersionRequest {
        project_code: "P1".into(),
        version_name: "2024-09-27".into(),
    })
    .await
    .unwrap()
    .id
}

// ---- version ----

#[tokio::test]
async fn version_crud_round_trips_through_api() {
    let svc = make_service();
    let created = svc
        .create_version(CreateSdtmProjectVersionRequest {
            project_code: "P1".into(),
            version_name: "2024-09-27".into(),
        })
        .await
        .unwrap();
    assert_eq!(created.project_code, "P1");

    let listed = svc.list_versions_by_project("P1").await.unwrap();
    assert_eq!(listed.versions.len(), 1);

    let updated = svc
        .update_version(UpdateSdtmProjectVersionRequest {
            id: created.id,
            version_name: Some("2025-01-15".into()),
        })
        .await
        .unwrap();
    assert_eq!(updated.version_name, "2025-01-15");

    svc.delete_version(created.id).await.unwrap();
    assert!(
        svc.list_versions_by_project("P1")
            .await
            .unwrap()
            .versions
            .is_empty()
    );
}

#[tokio::test]
async fn duplicate_version_maps_to_api_error() {
    let svc = make_service();
    let _ = seed_version(&svc).await;
    let err = svc
        .create_version(CreateSdtmProjectVersionRequest {
            project_code: "P1".into(),
            version_name: "2024-09-27".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        SdtmProjectApiError::DuplicateSdtmProjectVersion { .. }
    ));
}

#[tokio::test]
async fn unknown_version_maps_to_api_error() {
    let svc = make_service();
    let err = svc.get_version_by_id(999).await.unwrap_err();
    assert!(matches!(
        err,
        SdtmProjectApiError::SdtmProjectVersionNotFound(999)
    ));
}

#[tokio::test]
async fn missing_project_maps_to_api_error() {
    let svc = make_service();
    let err = svc
        .create_version(CreateSdtmProjectVersionRequest {
            project_code: "NOPE".into(),
            version_name: "2024-09-27".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        SdtmProjectApiError::ProjectNotFound(ref code) if code == "NOPE"
    ));
}

#[tokio::test]
async fn validation_error_maps_to_validation_api_error() {
    let svc = make_service();
    let err = svc
        .create_version(CreateSdtmProjectVersionRequest {
            project_code: "P1".into(),
            version_name: "   ".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, SdtmProjectApiError::Validation(_)));
}

// ---- domain ----

#[tokio::test]
async fn domain_crud_round_trips_through_api() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let created = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            sdtm_project_version_id: version_id,
            domain_code: "AE".into(),
            category: DomainCategory::Events,
            descriptions: Vec::new(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap();
    assert_eq!(created.domain_code, "AE");
    assert_eq!(created.category, DomainCategory::Events);

    let by_id = svc.get_domain_by_id(created.id).await.unwrap();
    assert_eq!(by_id.id, created.id);

    let listed = svc.list_domains_by_version(version_id).await.unwrap();
    assert_eq!(listed.domains.len(), 1);

    let updated = svc
        .update_domain(UpdateSdtmProjectDomainRequest {
            id: created.id,
            domain_code: Some("AE2".into()),
            category: Some(DomainCategory::Findings),
            descriptions: None,
            key_variables: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.domain_code, "AE2");
    assert_eq!(updated.category, DomainCategory::Findings);

    svc.delete_domain(created.id).await.unwrap();
    assert!(
        svc.list_domains_by_version(version_id)
            .await
            .unwrap()
            .domains
            .is_empty()
    );
}

#[tokio::test]
async fn key_variables_order_survives_the_api() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let created = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            sdtm_project_version_id: version_id,
            domain_code: "AE".into(),
            category: DomainCategory::Events,
            descriptions: Vec::new(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap();
    for code in ["AESEV", "AETERM"] {
        svc.create_variable(CreateSdtmProjectVariableRequest {
            sdtm_project_domain_id: created.id,
            variable_code: code.into(),
            variable_controlled: None,
            variable_type: SdtmVariableType::Character,
            variable_core: SdtmVariableCore::Req,
            variable_role: None,
            variable_sequence: 11,
            descriptions: Vec::new(),
            conversion_rules: Vec::new(),
        })
        .await
        .unwrap();
    }
    let updated = svc
        .update_domain(UpdateSdtmProjectDomainRequest {
            id: created.id,
            domain_code: None,
            category: None,
            descriptions: None,
            key_variables: Some(vec!["AESEV".into(), "AETERM".into()]),
        })
        .await
        .unwrap();
    // The caller's order is authoritative and is not re-sorted.
    assert_eq!(
        updated.key_variables,
        vec!["AESEV".to_string(), "AETERM".to_string()]
    );

    let by_id = svc.get_domain_by_id(created.id).await.unwrap();
    assert_eq!(
        by_id.key_variables,
        vec!["AESEV".to_string(), "AETERM".to_string()]
    );
}

// ---- variable ----

#[tokio::test]
async fn variable_crud_round_trips_through_api() {
    let svc = make_service();
    let version_id = seed_version(&svc).await;
    let domain = svc
        .create_domain(CreateSdtmProjectDomainRequest {
            sdtm_project_version_id: version_id,
            domain_code: "AE".into(),
            category: DomainCategory::Events,
            descriptions: Vec::new(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap();

    let created = svc
        .create_variable(CreateSdtmProjectVariableRequest {
            sdtm_project_domain_id: domain.id,
            variable_code: "AETERM".into(),
            variable_controlled: None,
            variable_type: SdtmVariableType::Character,
            variable_core: SdtmVariableCore::Req,
            variable_role: Some(SdtmRole::Topic),
            variable_sequence: 11,
            descriptions: Vec::new(),
            conversion_rules: vec![ConversionRule {
                source: VariableSource::Crf(Some(FromCrf {
                    form_code: "AE".into(),
                })),
                rule: "AESEV".into(),
            }],
        })
        .await
        .unwrap();
    assert_eq!(created.variable_code, "AETERM");
    assert_eq!(created.variable_sequence, 11);
    assert_eq!(created.variable_role, Some(SdtmRole::Topic));
    assert_eq!(created.conversion_rules.len(), 1);
    assert_eq!(created.conversion_rules[0].rule, "AESEV");
    assert_eq!(
        created.conversion_rules[0].source,
        VariableSource::Crf(Some(FromCrf {
            form_code: "AE".into(),
        }))
    );

    let listed = svc.list_variables_by_domain(domain.id).await.unwrap();
    assert_eq!(listed.variables.len(), 1);

    let updated = svc
        .update_variable(UpdateSdtmProjectVariableRequest {
            id: created.id,
            variable_code: None,
            variable_controlled: None,
            variable_type: None,
            variable_core: None,
            variable_role: None,
            variable_sequence: Some(12),
            descriptions: None,
            conversion_rules: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.variable_sequence, 12);

    svc.delete_variable(created.id).await.unwrap();
    assert!(
        svc.list_variables_by_domain(domain.id)
            .await
            .unwrap()
            .variables
            .is_empty()
    );
}
