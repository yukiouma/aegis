use async_trait::async_trait;

use apis::mission::{
    Actor, AssigneeData, AssigneeView as ApiAssigneeView, CreateMissionRequest,
    ListMissionsByProjectRequest, ListMissionsByUserRequest, MissionApiError, MissionKind as ApiKind,
    MissionRole as ApiRole, MissionService, MissionView as ApiMissionView,
};

use crate::domain::{DomainError, MissionRepository, ProjectLookup, UserLookup};
use crate::usecase::{
    AssigneeData as UcAssigneeData, CreateMission as UcCreateMission, MissionUsecase,
    MissionUsecaseConfig, UsecaseError,
};

use crate::usecase::AssigneeView as UcAssigneeView;
use crate::usecase::MissionView as UcMissionView;

pub struct MissionServiceImpl<M, A, P, U> {
    usecase: MissionUsecase<M, A, P, U>,
}

impl<M, A, P, U> MissionServiceImpl<M, A, P, U>
where
    M: MissionRepository,
    A: crate::domain::AssigneeRepository,
    P: ProjectLookup,
    U: UserLookup,
{
    pub fn from_usecase(usecase: MissionUsecase<M, A, P, U>) -> Self {
        Self { usecase }
    }

    pub fn from_repos(
        mission_repo: M,
        assignee_repo: A,
        projects: std::sync::Arc<P>,
        users: std::sync::Arc<U>,
    ) -> Self
    where
        A: Clone,
        P: Clone,
        U: Clone,
    {
        Self::from_usecase(MissionUsecase::new(MissionUsecaseConfig {
            mission_repo,
            assignee_repo,
            project_lookup: (*projects).clone(),
            user_lookup: (*users).clone(),
        }))
    }
}

#[async_trait]
impl<M, A, P, U> MissionService for MissionServiceImpl<M, A, P, U>
where
    M: MissionRepository + 'static,
    A: crate::domain::AssigneeRepository + 'static,
    P: ProjectLookup + 'static,
    U: UserLookup + 'static,
{
    async fn create_mission(
        &self,
        actor: &Actor,
        req: CreateMissionRequest,
    ) -> Result<ApiMissionView, MissionApiError> {
        self.usecase
            .create_mission(
                actor,
                UcCreateMission {
                    project_code: req.project_code,
                    mission_kind: req.mission_kind.into(),
                    mission_code: req.mission_code,
                    assignees: req
                        .assignees
                        .into_iter()
                        .map(|a| UcAssigneeData {
                            user_code: a.user_code,
                            role: a.role.into(),
                        })
                        .collect(),
                },
            )
            .await
            .map(into_api_mission)
            .map_err(map_error)
    }

    async fn get_mission_by_id(&self, id: i64) -> Result<ApiMissionView, MissionApiError> {
        self.usecase
            .get_mission_by_id(id)
            .await
            .map(into_api_mission)
            .map_err(map_error)
    }

    async fn list_missions_by_project(
        &self,
        req: ListMissionsByProjectRequest,
    ) -> Result<Vec<ApiMissionView>, MissionApiError> {
        self.usecase
            .list_missions_by_project(&req.project_code, req.kind.map(Into::into))
            .await
            .map(|v| v.into_iter().map(into_api_mission).collect())
            .map_err(map_error)
    }

    async fn list_missions_by_user(
        &self,
        req: ListMissionsByUserRequest,
    ) -> Result<Vec<ApiMissionView>, MissionApiError> {
        self.usecase
            .list_missions_by_user(&req.user_code)
            .await
            .map(|v| v.into_iter().map(into_api_mission).collect())
            .map_err(map_error)
    }

    async fn delete_mission(
        &self,
        actor: &Actor,
        id: i64,
    ) -> Result<(), MissionApiError> {
        self.usecase.delete_mission(actor, id).await.map_err(map_error)
    }

    async fn add_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        data: AssigneeData,
    ) -> Result<ApiAssigneeView, MissionApiError> {
        self.usecase
            .add_assignee(
                actor,
                mission_id,
                UcAssigneeData {
                    user_code: data.user_code,
                    role: data.role.into(),
                },
            )
            .await
            .map(into_api_assignee)
            .map_err(map_error)
    }

    async fn remove_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        assignee_id: i64,
    ) -> Result<(), MissionApiError> {
        self.usecase
            .remove_assignee(actor, mission_id, assignee_id)
            .await
            .map_err(map_error)
    }
}

// ---- apis <-> domain enum bridges ----

impl From<ApiKind> for crate::domain::MissionKind {
    fn from(k: ApiKind) -> Self {
        match k {
            ApiKind::Crf => crate::domain::MissionKind::Crf,
            ApiKind::Sdtm => crate::domain::MissionKind::Sdtm,
            ApiKind::Adam => crate::domain::MissionKind::Adam,
            ApiKind::Tfl => crate::domain::MissionKind::Tfl,
        }
    }
}

impl From<crate::domain::MissionKind> for ApiKind {
    fn from(k: crate::domain::MissionKind) -> Self {
        match k {
            crate::domain::MissionKind::Crf => ApiKind::Crf,
            crate::domain::MissionKind::Sdtm => ApiKind::Sdtm,
            crate::domain::MissionKind::Adam => ApiKind::Adam,
            crate::domain::MissionKind::Tfl => ApiKind::Tfl,
        }
    }
}

impl From<ApiRole> for crate::domain::MissionRole {
    fn from(r: ApiRole) -> Self {
        match r {
            ApiRole::Dev => crate::domain::MissionRole::Dev,
            ApiRole::Qc => crate::domain::MissionRole::Qc,
        }
    }
}

impl From<crate::domain::MissionRole> for ApiRole {
    fn from(r: crate::domain::MissionRole) -> Self {
        match r {
            crate::domain::MissionRole::Dev => ApiRole::Dev,
            crate::domain::MissionRole::Qc => ApiRole::Qc,
        }
    }
}

// ---- view bridges ----

fn into_api_mission(m: UcMissionView) -> ApiMissionView {
    ApiMissionView {
        id: m.id,
        project_code: m.project_code,
        mission_kind: m.mission_kind.into(),
        mission_code: m.mission_code,
        assignees: m.assignees.into_iter().map(into_api_assignee).collect(),
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn into_api_assignee(a: UcAssigneeView) -> ApiAssigneeView {
    ApiAssigneeView {
        id: a.id,
        user_code: a.user_code,
        role: a.role.into(),
        created_at: a.created_at,
        updated_at: a.updated_at,
    }
}

fn map_error(e: UsecaseError) -> MissionApiError {
    match e {
        UsecaseError::Forbidden {
            user_code,
            project_code,
        } => MissionApiError::Forbidden {
            user_code,
            project_code,
        },
        UsecaseError::Domain(d) => match d {
            DomainError::EmptyMissionCode
            | DomainError::EmptyUserCode
            | DomainError::UnknownMissionKind(_)
            | DomainError::UnknownMissionRole(_) => {
                MissionApiError::Validation(d.to_string())
            }
            DomainError::NotFound => MissionApiError::NotFound,
            DomainError::AssigneeNotFound => MissionApiError::AssigneeNotFound,
            DomainError::ProjectNotFound(c) => MissionApiError::ProjectNotFound(c),
            DomainError::UserNotFound(c) => MissionApiError::UserNotFound(c),
            DomainError::DuplicateMission {
                project_code,
                mission_kind,
                mission_code,
            } => MissionApiError::DuplicateMission {
                project_code,
                mission_kind: mission_kind.into(),
                mission_code,
            },
            DomainError::DuplicateAssignee {
                mission_id,
                user_code,
                role,
            } => MissionApiError::DuplicateAssignee {
                mission_id,
                user_code,
                role: role.into(),
            },
            DomainError::Repository(s) => MissionApiError::Repository(s),
        },
    }
}