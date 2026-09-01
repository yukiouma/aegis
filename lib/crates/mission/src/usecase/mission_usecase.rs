use apis::mission::Actor;

use crate::domain::{
    assignees_within_mission_are_unique, AssigneeNew, AssigneeRepository, MissionKind,
    MissionRepository, ProjectLookup, UserLookup,
};

use super::commands::{AssigneeData, CreateMission};
use super::error::UsecaseError;
use super::views::{AssigneeView, MissionView};

pub struct MissionUsecaseConfig<M, A, P, U> {
    pub mission_repo: M,
    pub assignee_repo: A,
    pub project_lookup: P,
    pub user_lookup: U,
}

pub struct MissionUsecase<M, A, P, U> {
    pub(crate) mission_repo: M,
    pub(crate) assignee_repo: A,
    pub(crate) project_lookup: P,
    pub(crate) user_lookup: U,
}

impl<M, A, P, U> MissionUsecase<M, A, P, U>
where
    M: MissionRepository,
    A: AssigneeRepository,
    P: ProjectLookup,
    U: UserLookup,
{
    pub fn new(config: MissionUsecaseConfig<M, A, P, U>) -> Self {
        Self {
            mission_repo: config.mission_repo,
            assignee_repo: config.assignee_repo,
            project_lookup: config.project_lookup,
            user_lookup: config.user_lookup,
        }
    }

    async fn ensure_leader(
        &self,
        actor: &Actor,
        project_code: &str,
    ) -> Result<(), UsecaseError> {
        let is_leader = self
            .project_lookup
            .is_leader(project_code, &actor.user_code)
            .await?;
        if !is_leader {
            return Err(UsecaseError::Forbidden {
                user_code: actor.user_code.clone(),
                project_code: project_code.to_string(),
            });
        }
        Ok(())
    }

    async fn ensure_project_exists(&self, project_code: &str) -> Result<(), UsecaseError> {
        self.project_lookup
            .get_by_code(project_code)
            .await
            .map_err(UsecaseError::from)
    }

    async fn ensure_user_exists(&self, user_code: &str) -> Result<(), UsecaseError> {
        self.user_lookup
            .get_by_code(user_code)
            .await
            .map_err(UsecaseError::from)
    }

    pub async fn create_mission(
        &self,
        actor: &Actor,
        input: CreateMission,
    ) -> Result<MissionView, UsecaseError> {
        self.ensure_leader(actor, &input.project_code).await?;
        self.ensure_project_exists(&input.project_code).await?;

        // Validate every assignee user exists up front so the
        // usecase surfaces a structured `UserNotFound` before
        // the repo transaction starts. The DB CHECK + UNIQUE
        // remain the safety net.
        for a in &input.assignees {
            self.ensure_user_exists(&a.user_code).await?;
        }

        assignees_within_mission_are_unique(
            &input
                .assignees
                .iter()
                .map(|a| AssigneeNew {
                    user_code: a.user_code.clone(),
                    role: a.role,
                })
                .collect::<Vec<_>>(),
        )?;

        let mission = self
            .mission_repo
            .create(crate::domain::MissionNew {
                project_code: input.project_code,
                mission_kind: input.mission_kind,
                mission_code: input.mission_code,
                assignees: input
                    .assignees
                    .into_iter()
                    .map(|a| AssigneeNew {
                        user_code: a.user_code,
                        role: a.role,
                    })
                    .collect(),
            })
            .await?;

        Ok(mission.into())
    }

    pub async fn get_mission_by_id(&self, id: i64) -> Result<MissionView, UsecaseError> {
        Ok(self.mission_repo.find_by_id(id).await?.into())
    }

    pub async fn list_missions_by_project(
        &self,
        project_code: &str,
        kind: Option<MissionKind>,
    ) -> Result<Vec<MissionView>, UsecaseError> {
        Ok(self
            .mission_repo
            .list_by_project(project_code, kind)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn list_missions_by_user(
        &self,
        user_code: &str,
    ) -> Result<Vec<MissionView>, UsecaseError> {
        Ok(self
            .mission_repo
            .list_by_user(user_code)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn delete_mission(
        &self,
        actor: &Actor,
        id: i64,
    ) -> Result<(), UsecaseError> {
        let m = self.mission_repo.find_by_id(id).await?;
        self.ensure_leader(actor, &m.project_code).await?;
        self.mission_repo.delete(id).await?;
        Ok(())
    }

    pub async fn add_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        data: AssigneeData,
    ) -> Result<AssigneeView, UsecaseError> {
        let m = self.mission_repo.find_by_id(mission_id).await?;
        self.ensure_leader(actor, &m.project_code).await?;
        self.ensure_user_exists(&data.user_code).await?;
        let assignee = self
            .assignee_repo
            .add(
                mission_id,
                AssigneeNew {
                    user_code: data.user_code,
                    role: data.role,
                },
            )
            .await?;
        Ok(assignee.into())
    }

    pub async fn remove_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        assignee_id: i64,
    ) -> Result<(), UsecaseError> {
        let m = self.mission_repo.find_by_id(mission_id).await?;
        self.ensure_leader(actor, &m.project_code).await?;
        self.assignee_repo.remove(mission_id, assignee_id).await?;
        Ok(())
    }
}