use std::collections::HashMap;

use crate::domain::{
    DomainError, Project, ProjectMember, ProjectNew, ProjectRepository, ProjectTag, ProjectUpdate,
    UserService, UserSummary,
};

use super::commands::{CreateProject, UpdateProject};
use super::error::UsecaseError;
use super::views::{ProjectMemberView, ProjectView};

pub struct ProjectUsecaseConfig<R: ProjectRepository, U: UserService> {
    pub project_repo: R,
    pub users: U,
}

pub struct ProjectUsecase<R: ProjectRepository, U: UserService> {
    project_repo: R,
    users: U,
}

impl<R: ProjectRepository, U: UserService> ProjectUsecase<R, U> {
    pub fn new(cfg: ProjectUsecaseConfig<R, U>) -> Self {
        Self {
            project_repo: cfg.project_repo,
            users: cfg.users,
        }
    }

    // -------- Projects --------

    pub async fn create_project(&self, cmd: CreateProject) -> Result<ProjectView, UsecaseError> {
        validate_create_project(&cmd)?;

        let new_project = self
            .project_repo
            .create(ProjectNew {
                code: cmd.code,
                description: cmd.description,
                members: cmd.members,
                unblind_members: cmd.unblind_members,
                tags: cmd.tags,
            })
            .await?;

        self.hydrate_project_view(new_project).await
    }

    pub async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, UsecaseError> {
        let project = self.project_repo.find_by_id(id).await?;
        self.hydrate_project_view(project).await
    }

    pub async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, UsecaseError> {
        if code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCode));
        }
        let project = self.project_repo.find_by_code(code).await?;
        self.hydrate_project_view(project).await
    }

    pub async fn list_projects(&self) -> Result<Vec<ProjectView>, UsecaseError> {
        let projects = self.project_repo.list().await?;
        let all_users = self.users.list().await?;
        let mut out = Vec::with_capacity(projects.len());
        for project in projects {
            let view = hydrate_with(&all_users, project)?;
            out.push(view);
        }
        Ok(out)
    }

    pub async fn update_project(&self, cmd: UpdateProject) -> Result<ProjectView, UsecaseError> {
        validate_update_project(&cmd)?;
        let updated = self
            .project_repo
            .update(ProjectUpdate {
                id: cmd.id,
                code: cmd.code,
                description: cmd.description,
                active: cmd.active,
                members: cmd.members,
                unblind_members: cmd.unblind_members,
                tags: cmd.tags,
            })
            .await?;
        self.hydrate_project_view(updated).await
    }

    // -------- helpers --------

    async fn hydrate_project_view(&self, project: Project) -> Result<ProjectView, UsecaseError> {
        let all_users = self.users.list().await?;
        hydrate_with(&all_users, project)
    }
}

/// Bucket the supplied user summaries into a project's two teams and
/// produce a `ProjectView`. Pure (no I/O) so tests can exercise it
/// directly through the usecase.
fn hydrate_with(
    users: &[UserSummary],
    project: Project,
) -> Result<ProjectView, UsecaseError> {
    let by_code: HashMap<&str, &UserSummary> = users.iter().map(|u| (u.code.as_str(), u)).collect();
    let members = project.members.clone();
    let unblind_members = project.unblind_members.clone();

    let leaders: Vec<UserSummary> = lookup_set(&by_code, &members.leaders)?;
    let workers: Vec<UserSummary> = lookup_set(&by_code, &members.workers)?;
    let members_view = ProjectMemberView {
        leaders: leaders.into_iter().map(Into::into).collect(),
        workers: workers.into_iter().map(Into::into).collect(),
    };

    let unblind_leaders: Vec<UserSummary> = lookup_set(&by_code, &unblind_members.leaders)?;
    let unblind_workers: Vec<UserSummary> = lookup_set(&by_code, &unblind_members.workers)?;
    let unblind_view = ProjectMemberView {
        leaders: unblind_leaders.into_iter().map(Into::into).collect(),
        workers: unblind_workers.into_iter().map(Into::into).collect(),
    };

    Ok(ProjectView::from_project(project, members_view, unblind_view))
}

fn lookup_set<'a>(
    by_code: &HashMap<&'a str, &'a UserSummary>,
    codes: &[String],
) -> Result<Vec<UserSummary>, UsecaseError> {
    let mut out = Vec::with_capacity(codes.len());
    for code in codes {
        match by_code.get(code.as_str()) {
            Some(summary) => out.push((*summary).clone()),
            None => {
                return Err(UsecaseError::Repository(DomainError::UserNotFound(
                    code.clone(),
                )));
            }
        }
    }
    Ok(out)
}

fn validate_create_project(cmd: &CreateProject) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if let Some(ref m) = cmd.members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref m) = cmd.unblind_members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref tags) = cmd.tags {
        for tag in tags {
            // Tag validation surfaces as `Validation`, not
            // `Repository`. The domain `From<DomainError>` impl maps
            // straight to `Repository`, so map the two tag variants
            // explicitly.
            match ProjectTag::new(tag.key.clone(), tag.value.clone()) {
                Ok(_) => {}
                Err(DomainError::EmptyTagKey) => {
                    return Err(UsecaseError::Validation(DomainError::EmptyTagKey));
                }
                Err(DomainError::EmptyTagValue) => {
                    return Err(UsecaseError::Validation(DomainError::EmptyTagValue));
                }
                Err(other) => return Err(UsecaseError::Repository(other)),
            }
        }
    }
    Ok(())
}

fn validate_update_project(cmd: &UpdateProject) -> Result<(), UsecaseError> {
    if let Some(ref c) = cmd.code
        && c.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if let Some(ref m) = cmd.members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref m) = cmd.unblind_members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref tags) = cmd.tags {
        for tag in tags {
            match ProjectTag::new(tag.key.clone(), tag.value.clone()) {
                Ok(_) => {}
                Err(DomainError::EmptyTagKey) => {
                    return Err(UsecaseError::Validation(DomainError::EmptyTagKey));
                }
                Err(DomainError::EmptyTagValue) => {
                    return Err(UsecaseError::Validation(DomainError::EmptyTagValue));
                }
                Err(other) => return Err(UsecaseError::Repository(other)),
            }
        }
    }
    Ok(())
}