use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, Ordering};

use async_trait::async_trait;

use crate::domain::{
    DomainCategory, DomainError, SdtmDomain, SdtmDomainDescription, SdtmDomainNew,
    SdtmDomainRepository, SdtmDomainUpdate, SdtmRole, SdtmVariable, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableNew, SdtmVariableRepository, SdtmVariableType,
    SdtmVariableUpdate, SdtmVersion, SdtmVersionNew, SdtmVersionRepository, SdtmVersionUpdate,
};
use crate::usecase::commands::{
    CreateSdtmDomain, CreateSdtmVariable, CreateSdtmVersion, UpdateSdtmDomain, UpdateSdtmVariable,
    UpdateSdtmVersion,
};
use crate::usecase::domain_model_usecase::{DomainModelUsecase, DomainModelUsecaseConfig};
use crate::usecase::error::UsecaseError;

// ---- in-memory fakes -----------------------------------------------------

#[derive(Default)]
struct InMemorySdtmVersionRepo {
    inner: Mutex<Vec<SdtmVersion>>,
    next_id: AtomicI64,
}

impl InMemorySdtmVersionRepo {
    fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SdtmVersionRepository for InMemorySdtmVersionRepo {
    async fn create(&self, input: SdtmVersionNew) -> Result<SdtmVersion, DomainError> {
        let mut g = self.inner.lock().unwrap();
        if g.iter().any(|v| v.name == input.name) {
            return Err(DomainError::DuplicateSdtmVersion {
                name: input.name.clone(),
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let v = SdtmVersion::for_repository(id, input.name, chrono::Utc::now(), chrono::Utc::now());
        g.push(v.clone());
        Ok(v)
    }
    async fn list(&self) -> Result<Vec<SdtmVersion>, DomainError> {
        Ok(self.inner.lock().unwrap().clone())
    }
    async fn update(&self, input: SdtmVersionUpdate) -> Result<SdtmVersion, DomainError> {
        let mut g = self.inner.lock().unwrap();
        let v = g
            .iter_mut()
            .find(|v| v.id == input.id)
            .ok_or(DomainError::SdtmVersionNotFound(input.id))?;
        if let Some(name) = input.name {
            v.name = name;
        }
        v.updated_at = chrono::Utc::now();
        Ok(v.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.inner.lock().unwrap();
        let before = g.len();
        g.retain(|v| v.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmVersionNotFound(id));
        }
        Ok(())
    }
}

#[derive(Default)]
struct InMemorySdtmDomainRepo {
    inner: Mutex<Vec<SdtmDomain>>,
    next_id: AtomicI64,
}

impl InMemorySdtmDomainRepo {
    fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SdtmDomainRepository for InMemorySdtmDomainRepo {
    async fn create(&self, input: SdtmDomainNew) -> Result<SdtmDomain, DomainError> {
        let mut g = self.inner.lock().unwrap();
        if g.iter()
            .any(|d| d.version_id == input.version_id && d.name == input.name)
        {
            return Err(DomainError::DuplicateSdtmDomain {
                version_id: input.version_id,
                name: input.name.clone(),
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let d = SdtmDomain::for_repository(
            id,
            input.version_id,
            input.name,
            input.category,
            input.descriptions,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        g.push(d.clone());
        Ok(d)
    }
    async fn find_by_id(&self, id: i64) -> Result<SdtmDomain, DomainError> {
        let g = self.inner.lock().unwrap();
        g.iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or(DomainError::SdtmDomainNotFound(id))
    }
    async fn list_by_version(&self, version_id: i64) -> Result<Vec<SdtmDomain>, DomainError> {
        let g = self.inner.lock().unwrap();
        Ok(g.iter()
            .filter(|d| d.version_id == version_id)
            .cloned()
            .collect())
    }
    async fn update(&self, input: SdtmDomainUpdate) -> Result<SdtmDomain, DomainError> {
        let mut g = self.inner.lock().unwrap();
        let d = g
            .iter_mut()
            .find(|d| d.id == input.id)
            .ok_or(DomainError::SdtmDomainNotFound(input.id))?;
        if let Some(name) = input.name {
            d.name = name;
        }
        if let Some(category) = input.category {
            d.category = category;
        }
        if let Some(descriptions) = input.descriptions {
            d.descriptions = descriptions;
        }
        d.updated_at = chrono::Utc::now();
        Ok(d.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.inner.lock().unwrap();
        let before = g.len();
        g.retain(|d| d.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmDomainNotFound(id));
        }
        Ok(())
    }
}

#[derive(Default)]
struct InMemorySdtmVariableRepo {
    inner: Mutex<Vec<SdtmVariable>>,
    next_id: AtomicI64,
}

impl InMemorySdtmVariableRepo {
    fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SdtmVariableRepository for InMemorySdtmVariableRepo {
    async fn create(&self, input: SdtmVariableNew) -> Result<SdtmVariable, DomainError> {
        let mut g = self.inner.lock().unwrap();
        if g.iter()
            .any(|v| v.domain_id == input.domain_id && v.name == input.name)
        {
            return Err(DomainError::DuplicateSdtmVariable {
                domain_id: input.domain_id,
                name: input.name.clone(),
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let v = SdtmVariable::for_repository(
            id,
            input.domain_id,
            input.name,
            input.variable_controlled,
            input.variable_type,
            input.variable_core,
            input.variable_role,
            input.variable_sequence,
            input.descriptions,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        g.push(v.clone());
        Ok(v)
    }
    async fn find_by_id(&self, id: i64) -> Result<SdtmVariable, DomainError> {
        let g = self.inner.lock().unwrap();
        g.iter()
            .find(|v| v.id == id)
            .cloned()
            .ok_or(DomainError::SdtmVariableNotFound(id))
    }
    async fn list_by_domain(&self, domain_id: i64) -> Result<Vec<SdtmVariable>, DomainError> {
        let g = self.inner.lock().unwrap();
        Ok(g.iter()
            .filter(|v| v.domain_id == domain_id)
            .cloned()
            .collect())
    }
    async fn update(&self, input: SdtmVariableUpdate) -> Result<SdtmVariable, DomainError> {
        let mut g = self.inner.lock().unwrap();
        let v = g
            .iter_mut()
            .find(|v| v.id == input.id)
            .ok_or(DomainError::SdtmVariableNotFound(input.id))?;
        if let Some(name) = input.name {
            v.name = name;
        }
        if let Some(vc) = input.variable_controlled {
            v.variable_controlled = vc;
        }
        if let Some(vt) = input.variable_type {
            v.variable_type = vt;
        }
        if let Some(vc) = input.variable_core {
            v.variable_core = vc;
        }
        if let Some(vr) = input.variable_role {
            v.variable_role = vr;
        }
        if let Some(seq) = input.variable_sequence {
            v.variable_sequence = seq;
        }
        if let Some(descriptions) = input.descriptions {
            v.descriptions = descriptions;
        }
        v.updated_at = chrono::Utc::now();
        Ok(v.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.inner.lock().unwrap();
        let before = g.len();
        g.retain(|v| v.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmVariableNotFound(id));
        }
        Ok(())
    }
}

fn build_usecase()
-> DomainModelUsecase<InMemorySdtmVersionRepo, InMemorySdtmDomainRepo, InMemorySdtmVariableRepo> {
    DomainModelUsecase::new(DomainModelUsecaseConfig {
        version_repo: InMemorySdtmVersionRepo::new(),
        domain_repo: InMemorySdtmDomainRepo::new(),
        variable_repo: InMemorySdtmVariableRepo::new(),
    })
}

// ---- version tests -------------------------------------------------------

#[tokio::test]
async fn version_crud_round_trips() {
    let uc = build_usecase();
    let v = uc
        .create_version(CreateSdtmVersion {
            name: "2024-09-27".into(),
        })
        .await
        .unwrap();
    assert_eq!(v.name, "2024-09-27");

    let listed = uc.list_versions().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, v.id);

    let updated = uc
        .update_version(UpdateSdtmVersion {
            id: v.id,
            name: Some("2025-01-15".into()),
        })
        .await
        .unwrap();
    assert_eq!(updated.name, "2025-01-15");

    uc.delete_version(v.id).await.unwrap();
    assert!(uc.list_versions().await.unwrap().is_empty());
}

#[tokio::test]
async fn version_create_rejects_empty_name() {
    let uc = build_usecase();
    let err = uc
        .create_version(CreateSdtmVersion { name: "   ".into() })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyName)
    ));
}

// ---- domain tests --------------------------------------------------------

#[tokio::test]
async fn domain_crud_round_trips() {
    let uc = build_usecase();
    let v = uc
        .create_version(CreateSdtmVersion {
            name: "2024-09-27".into(),
        })
        .await
        .unwrap();

    let desc = SdtmDomainDescription {
        lang: "en".into(),
        details: crate::domain::SdtmDomainDescriptionDetail {
            description: "Adverse events".into(),
            structure: "One record per AE".into(),
        },
    };
    let d = uc
        .create_domain(CreateSdtmDomain {
            version_id: v.id,
            name: "AE".into(),
            category: DomainCategory::Events,
            descriptions: vec![desc],
        })
        .await
        .unwrap();
    assert_eq!(d.name, "AE");
    assert_eq!(d.descriptions.len(), 1);

    let by_id = uc.get_domain_by_id(d.id).await.unwrap();
    assert_eq!(by_id.id, d.id);

    let list = uc.list_domains_by_version(v.id).await.unwrap();
    assert_eq!(list.len(), 1);

    let updated = uc
        .update_domain(UpdateSdtmDomain {
            id: d.id,
            name: Some("AE2".into()),
            category: None,
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.name, "AE2");

    uc.delete_domain(d.id).await.unwrap();
    assert!(uc.list_domains_by_version(v.id).await.unwrap().is_empty());
}

// ---- variable tests ------------------------------------------------------

#[tokio::test]
async fn variable_crud_round_trips() {
    let uc = build_usecase();
    let v = uc
        .create_version(CreateSdtmVersion {
            name: "2024-09-27".into(),
        })
        .await
        .unwrap();
    let d = uc
        .create_domain(CreateSdtmDomain {
            version_id: v.id,
            name: "AE".into(),
            category: DomainCategory::Events,
            descriptions: Vec::new(),
        })
        .await
        .unwrap();

    let desc = SdtmVariableDescription {
        lang: "en".into(),
        details: crate::domain::SdtmVariableDescriptionDetail {
            label: "Term".into(),
        },
    };
    let var = uc
        .create_variable(CreateSdtmVariable {
            domain_id: d.id,
            name: "AETERM".into(),
            variable_controlled: None,
            variable_type: SdtmVariableType::Character,
            variable_core: SdtmVariableCore::Req,
            variable_role: Some(SdtmRole::Topic),
            variable_sequence: 11,
            descriptions: vec![desc],
        })
        .await
        .unwrap();
    assert_eq!(var.name, "AETERM");

    let list = uc.list_variables_by_domain(d.id).await.unwrap();
    assert_eq!(list.len(), 1);

    // Clear variable_role via outer-Some(inner-None).
    let updated = uc
        .update_variable(UpdateSdtmVariable {
            id: var.id,
            name: None,
            variable_controlled: None,
            variable_type: None,
            variable_core: None,
            variable_role: Some(None),
            variable_sequence: None,
            descriptions: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.variable_role, None);

    uc.delete_variable(var.id).await.unwrap();
    assert!(uc.list_variables_by_domain(d.id).await.unwrap().is_empty());
}
