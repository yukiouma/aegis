use chrono::Utc;

use apis::mission::Actor;

use crate::domain::{
    AssigneeRepository, DomainError, IssueComment, IssueState, MissionIssueNew,
    MissionIssueRepository, MissionRepository, MissionRole, ProjectLookup,
};

use super::commands::CreateIssue;
use super::error::UsecaseError;
use super::views::IssueView;

pub struct MissionIssueUsecaseConfig<M, A, P, I> {
    pub mission_repo: M,
    pub assignee_repo: A,
    pub project_lookup: P,
    pub issue_repo: I,
}

pub struct MissionIssueUsecase<M, A, P, I> {
    pub(crate) mission_repo: M,
    pub(crate) assignee_repo: A,
    pub(crate) project_lookup: P,
    pub(crate) issue_repo: I,
}

impl<M, A, P, I> MissionIssueUsecase<M, A, P, I>
where
    M: MissionRepository,
    A: AssigneeRepository,
    P: ProjectLookup,
    I: MissionIssueRepository,
{
    pub fn new(config: MissionIssueUsecaseConfig<M, A, P, I>) -> Self {
        Self {
            mission_repo: config.mission_repo,
            assignee_repo: config.assignee_repo,
            project_lookup: config.project_lookup,
            issue_repo: config.issue_repo,
        }
    }

    /// Authorisation shared by create / close / reopen /
    /// update-description: project leader OR mission QC assignee.
    async fn ensure_can_create_or_close(
        &self,
        actor: &Actor,
        project_code: &str,
        mission_id: i64,
    ) -> Result<(), UsecaseError> {
        if self
            .project_lookup
            .is_leader(project_code, &actor.user_code)
            .await?
        {
            return Ok(());
        }
        if self
            .assignee_repo
            .is_assignee(mission_id, &actor.user_code, MissionRole::Qc)
            .await?
        {
            return Ok(());
        }
        Err(UsecaseError::Forbidden {
            user_code: actor.user_code.clone(),
            project_code: project_code.to_string(),
        })
    }

    /// Authorisation for appending a comment: project leader OR
    /// mission DEV assignee OR mission QC assignee.
    async fn ensure_can_comment(
        &self,
        actor: &Actor,
        project_code: &str,
        mission_id: i64,
    ) -> Result<(), UsecaseError> {
        if self
            .project_lookup
            .is_leader(project_code, &actor.user_code)
            .await?
        {
            return Ok(());
        }
        for role in [MissionRole::Dev, MissionRole::Qc] {
            if self
                .assignee_repo
                .is_assignee(mission_id, &actor.user_code, role)
                .await?
            {
                return Ok(());
            }
        }
        Err(UsecaseError::Forbidden {
            user_code: actor.user_code.clone(),
            project_code: project_code.to_string(),
        })
    }

    pub async fn list_issues_by_mission(
        &self,
        mission_id: i64,
        state: Option<IssueState>,
    ) -> Result<Vec<IssueView>, UsecaseError> {
        Ok(self
            .issue_repo
            .list_by_mission(mission_id, state)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn create_issue(
        &self,
        actor: &Actor,
        input: CreateIssue,
    ) -> Result<IssueView, UsecaseError> {
        // Empty description is rejected here so the repo's CHECK
        // is a safety net rather than the first line of defence.
        if input.description.trim().is_empty() {
            return Err(UsecaseError::Domain(DomainError::EmptyIssueDescription));
        }
        let mission = self.mission_repo.find_by_id(input.mission_id).await?;
        self.ensure_can_create_or_close(actor, &mission.project_code, mission.id)
            .await?;
        let issue = self
            .issue_repo
            .create(MissionIssueNew {
                mission_id: input.mission_id,
                target_item: input.target_item,
                issuer: actor.user_code.clone(),
                description: input.description,
            })
            .await?;
        Ok(issue.into())
    }

    pub async fn close_issue(
        &self,
        actor: &Actor,
        issue_id: i64,
    ) -> Result<IssueView, UsecaseError> {
        let issue = self.issue_repo.find_by_id(issue_id).await?;
        let mission = self.mission_repo.find_by_id(issue.mission_id).await?;
        self.ensure_can_create_or_close(actor, &mission.project_code, mission.id)
            .await?;
        let issue = self.issue_repo.close(issue_id).await?;
        Ok(issue.into())
    }

    pub async fn reopen_issue(
        &self,
        actor: &Actor,
        issue_id: i64,
    ) -> Result<IssueView, UsecaseError> {
        let issue = self.issue_repo.find_by_id(issue_id).await?;
        let mission = self.mission_repo.find_by_id(issue.mission_id).await?;
        self.ensure_can_create_or_close(actor, &mission.project_code, mission.id)
            .await?;
        let issue = self.issue_repo.open(issue_id).await?;
        Ok(issue.into())
    }

    pub async fn update_issue_description(
        &self,
        actor: &Actor,
        issue_id: i64,
        description: String,
    ) -> Result<IssueView, UsecaseError> {
        if description.trim().is_empty() {
            return Err(UsecaseError::Domain(DomainError::EmptyIssueDescription));
        }
        let issue = self.issue_repo.find_by_id(issue_id).await?;
        let mission = self.mission_repo.find_by_id(issue.mission_id).await?;
        self.ensure_can_create_or_close(actor, &mission.project_code, mission.id)
            .await?;
        let issue = self
            .issue_repo
            .update_description(issue_id, description)
            .await?;
        Ok(issue.into())
    }

    pub async fn append_comment(
        &self,
        actor: &Actor,
        issue_id: i64,
        content: String,
    ) -> Result<IssueView, UsecaseError> {
        if content.trim().is_empty() {
            return Err(UsecaseError::Domain(DomainError::EmptyCommentContent));
        }
        let issue = self.issue_repo.find_by_id(issue_id).await?;
        let mission = self.mission_repo.find_by_id(issue.mission_id).await?;
        self.ensure_can_comment(actor, &mission.project_code, mission.id)
            .await?;
        let now = Utc::now();
        let comment = IssueComment::new(actor.user_code.clone(), content, now)?;
        let issue = self.issue_repo.append_comment(issue_id, comment).await?;
        Ok(issue.into())
    }
}
