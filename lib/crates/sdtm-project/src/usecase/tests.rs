use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::domain::{
    ConversionRule, DomainCategory, DomainError, ProjectLookup, SdtmDomainDescription,
    SdtmProjectDomain, SdtmProjectDomainNew, SdtmProjectDomainRepository, SdtmProjectDomainUpdate,
    SdtmProjectVariable, SdtmProjectVariableNew, SdtmProjectVariableRepository,
    SdtmProjectVariableUpdate, SdtmProjectVersion, SdtmProjectVersionNew,
    SdtmProjectVersionRepository, SdtmProjectVersionUpdate, SdtmRole, SdtmVariableControlled,
    SdtmVariableCore, SdtmVariableDescription, SdtmVariableType, VariableSource,
};
use crate::usecase::commands::{
    CreateSdtmProjectDomain, CreateSdtmProjectVariable, CreateSdtmProjectVersion,
    UpdateSdtmProjectDomain, UpdateSdtmProjectVariable, UpdateSdtmProjectVersion,
};
use crate::usecase::error::UsecaseError;
use crate::usecase::sdtm_project_usecase::{SdtmProjectUsecase, SdtmProjectUsecaseConfig};

// ---- shared in-memory store ---------------------------------------------
//
// One store behind all three repositories, so the domain and variable
// repositories see the same rows the way the SQL adapters do.

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
struct InMemoryVersionRepo {
    store: Arc<InMemoryStore>,
}

#[async_trait]
impl SdtmProjectVersionRepository for InMemoryVersionRepo {
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
        let now = chrono::Utc::now();
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
        v.updated_at = chrono::Utc::now();
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

#[derive(Clone)]
struct InMemoryDomainRepo {
    store: Arc<InMemoryStore>,
}

#[async_trait]
impl SdtmProjectDomainRepository for InMemoryDomainRepo {
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
        let now = chrono::Utc::now();
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
        if let Some(key_variables) = input.key_variables {
            d.key_variables = key_variables;
        }
        d.updated_at = chrono::Utc::now();
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

#[derive(Clone)]
struct InMemoryVariableRepo {
    store: Arc<InMemoryStore>,
}

#[async_trait]
impl SdtmProjectVariableRepository for InMemoryVariableRepo {
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
        let now = chrono::Utc::now();
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
        v.updated_at = chrono::Utc::now();
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

type TestUsecase = SdtmProjectUsecase<
    InMemoryVersionRepo,
    InMemoryDomainRepo,
    InMemoryVariableRepo,
    FakeProjectLookup,
>;

fn build_usecase() -> TestUsecase {
    let store = Arc::new(InMemoryStore::default());
    SdtmProjectUsecase::new(SdtmProjectUsecaseConfig {
        version_repo: InMemoryVersionRepo {
            store: store.clone(),
        },
        domain_repo: InMemoryDomainRepo {
            store: store.clone(),
        },
        variable_repo: InMemoryVariableRepo {
            store: store.clone(),
        },
        projects: Arc::new(FakeProjectLookup::new(&["P1"])),
    })
}

async fn seed_version(uc: &TestUsecase) -> i64 {
    uc.create_version(CreateSdtmProjectVersion {
        project_code: "P1".into(),
        version_name: "2024-09-27".into(),
    })
    .await
    .unwrap()
    .id
}

/// A domain with no key variables — a brand-new domain has no
/// variables yet, so every code it names would be missing.
async fn seed_domain(uc: &TestUsecase, version_id: i64) -> i64 {
    uc.create_domain(CreateSdtmProjectDomain {
        sdtm_project_version_id: version_id,
        domain_code: "AE".into(),
        category: DomainCategory::Events,
        descriptions: Vec::new(),
        key_variables: Vec::new(),
    })
    .await
    .unwrap()
    .id
}

async fn seed_variable(uc: &TestUsecase, domain_id: i64, variable_code: &str) -> i64 {
    uc.create_variable(CreateSdtmProjectVariable {
        sdtm_project_domain_id: domain_id,
        variable_code: variable_code.into(),
        variable_controlled: None,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: None,
        variable_sequence: 11,
        descriptions: Vec::new(),
        conversion_rules: Vec::new(),
    })
    .await
    .unwrap()
    .id
}

fn domain_description() -> SdtmDomainDescription {
    SdtmDomainDescription {
        lang: "en".into(),
        details: crate::domain::SdtmDomainDescriptionDetail {
            description: "Adverse events".into(),
            structure: "One record per AE".into(),
        },
    }
}

fn variable_description() -> SdtmVariableDescription {
    SdtmVariableDescription {
        lang: "en".into(),
        details: crate::domain::SdtmVariableDescriptionDetail {
            label: "Adverse Event Term".into(),
        },
    }
}

// ---- version tests -------------------------------------------------------

#[tokio::test]
async fn version_crud_round_trips() {
    let uc = build_usecase();
    let v = uc
        .create_version(CreateSdtmProjectVersion {
            project_code: "P1".into(),
            version_name: "2024-09-27".into(),
        })
        .await
        .unwrap();
    assert_eq!(v.project_code, "P1");
    assert_eq!(v.version_name, "2024-09-27");

    let listed = uc.list_versions_by_project("P1").await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, v.id);

    let updated = uc
        .update_version(UpdateSdtmProjectVersion {
            id: v.id,
            version_name: Some("2025-01-15".into()),
        })
        .await
        .unwrap();
    assert_eq!(updated.version_name, "2025-01-15");

    uc.delete_version(v.id).await.unwrap();
    assert!(uc.list_versions_by_project("P1").await.unwrap().is_empty());
}

#[tokio::test]
async fn version_create_rejects_unknown_project() {
    let uc = build_usecase();
    let err = uc
        .create_version(CreateSdtmProjectVersion {
            project_code: "NOPE".into(),
            version_name: "2024-09-27".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::ProjectNotFound(ref code)) if code == "NOPE"
    ));
}

#[tokio::test]
async fn version_create_rejects_blank_version_name() {
    let uc = build_usecase();
    let err = uc
        .create_version(CreateSdtmProjectVersion {
            project_code: "P1".into(),
            version_name: "   ".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyVersionName)
    ));
}

// ---- domain tests --------------------------------------------------------

#[tokio::test]
async fn domain_crud_round_trips() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let d = uc
        .create_domain(CreateSdtmProjectDomain {
            sdtm_project_version_id: version_id,
            domain_code: "AE".into(),
            category: DomainCategory::Events,
            descriptions: vec![domain_description()],
            key_variables: Vec::new(),
        })
        .await
        .unwrap();
    assert_eq!(d.domain_code, "AE");
    assert_eq!(d.category, DomainCategory::Events);
    assert_eq!(d.descriptions.len(), 1);

    let by_id = uc.get_domain_by_id(d.id).await.unwrap();
    assert_eq!(by_id.id, d.id);

    let list = uc.list_domains_by_version(version_id).await.unwrap();
    assert_eq!(list.len(), 1);

    let updated = uc
        .update_domain(UpdateSdtmProjectDomain {
            id: d.id,
            domain_code: Some("AE2".into()),
            category: Some(DomainCategory::Findings),
            descriptions: None,
            key_variables: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.domain_code, "AE2");
    assert_eq!(updated.category, DomainCategory::Findings);

    uc.delete_domain(d.id).await.unwrap();
    assert!(
        uc.list_domains_by_version(version_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn domain_create_rejects_missing_version() {
    let uc = build_usecase();
    let err = uc
        .create_domain(CreateSdtmProjectDomain {
            sdtm_project_version_id: 999,
            domain_code: "AE".into(),
            category: DomainCategory::Events,
            descriptions: Vec::new(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::FkSdtmProjectVersionNotFound(999))
    ));
}

/// A brand-new domain has no variables yet, so any key variable it
/// names is missing by definition.
#[tokio::test]
async fn create_domain_with_key_variables_is_rejected() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let err = uc
        .create_domain(CreateSdtmProjectDomain {
            sdtm_project_version_id: version_id,
            domain_code: "AE".into(),
            category: DomainCategory::Events,
            descriptions: Vec::new(),
            key_variables: vec!["AETERM".into()],
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::KeyVariableNotFound { variable_code })
            if variable_code == "AETERM"
    ));
}

#[tokio::test]
async fn update_domain_accepts_key_variable_that_names_an_existing_variable() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    seed_variable(&uc, domain_id, "AETERM").await;

    let updated = uc
        .update_domain(UpdateSdtmProjectDomain {
            id: domain_id,
            domain_code: None,
            category: None,
            descriptions: None,
            key_variables: Some(vec!["AETERM".into()]),
        })
        .await
        .unwrap();
    assert_eq!(updated.key_variables, vec!["AETERM".to_string()]);
}

#[tokio::test]
async fn update_domain_rejects_key_variable_that_names_no_variable() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    seed_variable(&uc, domain_id, "AETERM").await;

    let err = uc
        .update_domain(UpdateSdtmProjectDomain {
            id: domain_id,
            domain_code: None,
            category: None,
            descriptions: None,
            key_variables: Some(vec!["AESTDTC".into()]),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::KeyVariableNotFound { variable_code })
            if variable_code == "AESTDTC"
    ));
}

/// The check is scoped to the domain being written — a code that
/// exists in a *sibling* domain is still missing here.
#[tokio::test]
async fn update_domain_rejects_key_variable_from_another_domain() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_a = seed_domain(&uc, version_id).await;
    seed_variable(&uc, domain_a, "AETERM").await;

    let domain_b = uc
        .create_domain(CreateSdtmProjectDomain {
            sdtm_project_version_id: version_id,
            domain_code: "VS".into(),
            category: DomainCategory::Findings,
            descriptions: Vec::new(),
            key_variables: Vec::new(),
        })
        .await
        .unwrap();

    let err = uc
        .update_domain(UpdateSdtmProjectDomain {
            id: domain_b.id,
            domain_code: None,
            category: None,
            descriptions: None,
            key_variables: Some(vec!["AETERM".into()]),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::KeyVariableNotFound { variable_code })
            if variable_code == "AETERM"
    ));
}

/// An explicit empty list clears the column; it is not read as
/// "leave unchanged".
#[tokio::test]
async fn update_domain_with_empty_key_variables_clears_the_list() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    seed_variable(&uc, domain_id, "AETERM").await;

    uc.update_domain(UpdateSdtmProjectDomain {
        id: domain_id,
        domain_code: None,
        category: None,
        descriptions: None,
        key_variables: Some(vec!["AETERM".into()]),
    })
    .await
    .unwrap();

    let cleared = uc
        .update_domain(UpdateSdtmProjectDomain {
            id: domain_id,
            domain_code: None,
            category: None,
            descriptions: None,
            key_variables: Some(Vec::new()),
        })
        .await
        .unwrap();
    assert!(cleared.key_variables.is_empty());
}

// ---- variable tests ------------------------------------------------------

#[tokio::test]
async fn variable_crud_round_trips() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;

    let v = uc
        .create_variable(CreateSdtmProjectVariable {
            sdtm_project_domain_id: domain_id,
            variable_code: "AETERM".into(),
            variable_controlled: Some(SdtmVariableControlled {
                code: Some(vec!["C66731".into()]),
                format: None,
            }),
            variable_type: SdtmVariableType::Character,
            variable_core: SdtmVariableCore::Req,
            variable_role: Some(SdtmRole::Topic),
            variable_sequence: 11,
            descriptions: vec![variable_description()],
            conversion_rules: Vec::new(),
        })
        .await
        .unwrap();
    assert_eq!(v.variable_code, "AETERM");
    assert_eq!(v.variable_sequence, 11);
    assert_eq!(v.variable_type, SdtmVariableType::Character);
    assert_eq!(v.variable_core, SdtmVariableCore::Req);
    assert_eq!(v.variable_role, Some(SdtmRole::Topic));
    assert_eq!(v.descriptions.len(), 1);
    assert_eq!(
        v.variable_controlled,
        Some(SdtmVariableControlled {
            code: Some(vec!["C66731".into()]),
            format: None,
        })
    );

    let list = uc.list_variables_by_domain(domain_id).await.unwrap();
    assert_eq!(list.len(), 1);

    let updated = uc
        .update_variable(UpdateSdtmProjectVariable {
            id: v.id,
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

    uc.delete_variable(v.id).await.unwrap();
    assert!(
        uc.list_variables_by_domain(domain_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn variable_create_rejects_missing_domain() {
    let uc = build_usecase();
    let err = uc
        .create_variable(CreateSdtmProjectVariable {
            sdtm_project_domain_id: 999,
            variable_code: "AETERM".into(),
            variable_controlled: None,
            variable_type: SdtmVariableType::Character,
            variable_core: SdtmVariableCore::Req,
            variable_role: None,
            variable_sequence: 1,
            descriptions: Vec::new(),
            conversion_rules: Vec::new(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::FkSdtmProjectDomainNotFound(999))
    ));
}

#[tokio::test]
async fn create_variable_stores_conversion_rules() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;

    let v = uc
        .create_variable(CreateSdtmProjectVariable {
            sdtm_project_domain_id: domain_id,
            variable_code: "AETERM".into(),
            variable_controlled: None,
            variable_type: SdtmVariableType::Character,
            variable_core: SdtmVariableCore::Req,
            variable_role: None,
            variable_sequence: 11,
            descriptions: Vec::new(),
            conversion_rules: vec![ConversionRule {
                source: VariableSource::Crf(Some(crate::domain::FromCrf {
                    form_code: "AE".into(),
                })),
                rule: "AESEV".into(),
            }],
        })
        .await
        .unwrap();
    assert_eq!(v.conversion_rules.len(), 1);
    assert_eq!(v.conversion_rules[0].rule, "AESEV");
    assert_eq!(
        v.conversion_rules[0].source,
        VariableSource::Crf(Some(crate::domain::FromCrf {
            form_code: "AE".into(),
        }))
    );
}

#[tokio::test]
async fn create_variable_rejects_a_blank_conversion_rule() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;

    let err = uc
        .create_variable(CreateSdtmProjectVariable {
            sdtm_project_domain_id: domain_id,
            variable_code: "AETERM".into(),
            variable_controlled: None,
            variable_type: SdtmVariableType::Character,
            variable_core: SdtmVariableCore::Req,
            variable_role: None,
            variable_sequence: 11,
            descriptions: Vec::new(),
            conversion_rules: vec![ConversionRule {
                source: VariableSource::Protocol,
                rule: "  ".into(),
            }],
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyConversionRule)
    ));
}

/// A variable a domain names as a key variable cannot be renamed out
/// from under it.
#[tokio::test]
async fn update_variable_rejects_rename_when_the_old_code_is_a_key_variable() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    let variable_id = seed_variable(&uc, domain_id, "AETERM").await;
    uc.update_domain(UpdateSdtmProjectDomain {
        id: domain_id,
        domain_code: None,
        category: None,
        descriptions: None,
        key_variables: Some(vec!["AETERM".into()]),
    })
    .await
    .unwrap();

    let err = uc
        .update_variable(UpdateSdtmProjectVariable {
            id: variable_id,
            variable_code: Some("AETERMX".into()),
            variable_controlled: None,
            variable_type: None,
            variable_core: None,
            variable_role: None,
            variable_sequence: None,
            descriptions: None,
            conversion_rules: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::VariableReferencedByKeyVariables { variable_code })
            if variable_code == "AETERM"
    ));

    // The variable still carries its old code.
    let still = uc.get_variable_by_id(variable_id).await.unwrap();
    assert_eq!(still.variable_code, "AETERM");
}

/// Re-sending the *same* code is not a rename: an ordinary metadata
/// edit on a key variable stays possible.
#[tokio::test]
async fn update_variable_allows_an_unchanged_code_on_a_key_variable() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    let variable_id = seed_variable(&uc, domain_id, "AETERM").await;
    uc.update_domain(UpdateSdtmProjectDomain {
        id: domain_id,
        domain_code: None,
        category: None,
        descriptions: None,
        key_variables: Some(vec!["AETERM".into()]),
    })
    .await
    .unwrap();

    let updated = uc
        .update_variable(UpdateSdtmProjectVariable {
            id: variable_id,
            variable_code: Some("AETERM".into()),
            variable_controlled: None,
            variable_type: Some(SdtmVariableType::Numeric),
            variable_core: None,
            variable_role: None,
            variable_sequence: None,
            descriptions: None,
            conversion_rules: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.variable_type, SdtmVariableType::Numeric);
}

#[tokio::test]
async fn delete_variable_rejects_when_the_code_is_a_key_variable() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    let variable_id = seed_variable(&uc, domain_id, "AETERM").await;
    uc.update_domain(UpdateSdtmProjectDomain {
        id: domain_id,
        domain_code: None,
        category: None,
        descriptions: None,
        key_variables: Some(vec!["AETERM".into()]),
    })
    .await
    .unwrap();

    let err = uc.delete_variable(variable_id).await.unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::VariableReferencedByKeyVariables { variable_code })
            if variable_code == "AETERM"
    ));

    // The variable still exists.
    assert!(uc.get_variable_by_id(variable_id).await.is_ok());
}

#[tokio::test]
async fn delete_variable_succeeds_when_the_code_is_not_a_key_variable() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    let variable_id = seed_variable(&uc, domain_id, "AETERM").await;

    uc.delete_variable(variable_id).await.unwrap();
    assert!(
        uc.list_variables_by_domain(domain_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn update_variable_replaces_conversion_rules() {
    let uc = build_usecase();
    let version_id = seed_version(&uc).await;
    let domain_id = seed_domain(&uc, version_id).await;
    let variable_id = seed_variable(&uc, domain_id, "AETERM").await;

    let updated = uc
        .update_variable(UpdateSdtmProjectVariable {
            id: variable_id,
            variable_code: None,
            variable_controlled: None,
            variable_type: None,
            variable_core: None,
            variable_role: None,
            variable_sequence: None,
            descriptions: None,
            conversion_rules: Some(vec![ConversionRule {
                source: VariableSource::Derived,
                rule: "AESTDTC".into(),
            }]),
        })
        .await
        .unwrap();
    assert_eq!(updated.conversion_rules.len(), 1);
    assert_eq!(updated.conversion_rules[0].source, VariableSource::Derived);
}

/// A missing domain is a 404, not a 400 — even when the request also
/// carries key variables. Otherwise a client updating a deleted domain
/// is told the *variable* is wrong instead of the domain.
#[tokio::test]
async fn update_domain_of_missing_domain_is_not_found_even_with_key_variables() {
    let uc = build_usecase();
    let err = uc
        .update_domain(UpdateSdtmProjectDomain {
            id: 999,
            domain_code: None,
            category: None,
            descriptions: None,
            key_variables: Some(vec!["AETERM".into()]),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::SdtmProjectDomainNotFound(999))
    ));
}
