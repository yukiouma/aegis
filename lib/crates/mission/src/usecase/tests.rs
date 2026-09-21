use apis::mission::Actor;

use crate::domain::{Assignee, AssigneeNew, DomainError, IssueState, MissionKind, MissionRole};
use crate::usecase::{
    CreateIssue, CreateMission, MissionIssueUsecase, MissionIssueUsecaseConfig, MissionUsecase,
    MissionUsecaseConfig, UsecaseError,
};

#[path = "../test_support.rs"]
#[allow(clippy::duplicate_mod)]
mod test_support;
use test_support::{FakeAssigneeRepo, FakeIssueRepo, FakeMissionRepo, FakeProject, FakeUser};

fn usecase() -> MissionUsecase<FakeMissionRepo, FakeAssigneeRepo, FakeProject, FakeUser> {
    MissionUsecase::new(MissionUsecaseConfig {
        mission_repo: FakeMissionRepo::default(),
        assignee_repo: FakeAssigneeRepo::default(),
        project_lookup: FakeProject {
            leader_for: vec!["alice"],
        },
        user_lookup: FakeUser,
    })
}

#[tokio::test]
async fn create_mission_enforces_leadership() {
    let uc = usecase();
    let err = uc
        .create_mission(
            &Actor {
                user_code: "carol".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: MissionKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![],
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
}

#[tokio::test]
async fn create_mission_succeeds_for_leader() {
    let uc = usecase();
    let view = uc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: MissionKind::Sdtm,
                mission_code: "c1".into(),
                assignees: vec![crate::usecase::AssigneeData {
                    user_code: "u1".into(),
                    role: MissionRole::Dev,
                }],
            },
        )
        .await
        .unwrap();
    assert_eq!(view.project_code, "p1");
    assert_eq!(view.assignees.len(), 1);
}

#[tokio::test]
async fn create_mission_rejects_unknown_user_in_assignees() {
    let uc = usecase();
    let err = uc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: MissionKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![crate::usecase::AssigneeData {
                    user_code: "ghost".into(),
                    role: MissionRole::Dev,
                }],
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Domain(DomainError::UserNotFound(_))
    ));
}

#[tokio::test]
async fn list_missions_by_project_filters_by_kind() {
    let uc = usecase();
    for (kind, code) in [(MissionKind::Crf, "c1"), (MissionKind::Sdtm, "s1")] {
        uc.create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: kind,
                mission_code: code.into(),
                assignees: vec![],
            },
        )
        .await
        .unwrap();
    }
    let only_crf = uc
        .list_missions_by_project("p1", Some(MissionKind::Crf))
        .await
        .unwrap();
    assert_eq!(only_crf.len(), 1);
    assert_eq!(only_crf[0].mission_kind, MissionKind::Crf);
}

#[tokio::test]
async fn delete_mission_requires_leader() {
    let uc = usecase();
    let m = uc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: MissionKind::Adam,
                mission_code: "a1".into(),
                assignees: vec![],
            },
        )
        .await
        .unwrap();
    let err = uc
        .delete_mission(
            &Actor {
                user_code: "carol".into(),
            },
            m.id,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
}

// ---- issue usecase ----

struct Shared {
    mission_repo: FakeMissionRepo,
    assignee_repo: FakeAssigneeRepo,
}

impl Shared {
    fn new() -> Self {
        // Share the same Arc<Mutex<Vec<(i64, Assignee)>>> between
        // the mission repo (which pushes initial assignees on
        // create) and the assignee repo (which the issue usecase
        // reads via is_assignee). Mirrors what the live Postgres
        // MissionRepo::create transaction does in one statement.
        let assignee_store: std::sync::Arc<std::sync::Mutex<Vec<(i64, Assignee)>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mission_repo = FakeMissionRepo {
            assignee_repo: Some(assignee_store.clone()),
            ..FakeMissionRepo::default()
        };
        let assignee_repo = FakeAssigneeRepo {
            assignees: assignee_store,
            ..FakeAssigneeRepo::default()
        };
        Self {
            mission_repo,
            assignee_repo,
        }
    }
    fn mission_usecase(
        &self,
    ) -> MissionUsecase<FakeMissionRepo, FakeAssigneeRepo, FakeProject, FakeUser> {
        MissionUsecase::new(MissionUsecaseConfig {
            mission_repo: self.mission_repo.clone(),
            assignee_repo: self.assignee_repo.clone(),
            project_lookup: FakeProject {
                leader_for: vec!["alice"],
            },
            user_lookup: FakeUser,
        })
    }
    fn issue_usecase(
        &self,
    ) -> MissionIssueUsecase<FakeMissionRepo, FakeAssigneeRepo, FakeProject, FakeIssueRepo> {
        MissionIssueUsecase::new(MissionIssueUsecaseConfig {
            mission_repo: self.mission_repo.clone(),
            assignee_repo: self.assignee_repo.clone(),
            project_lookup: FakeProject {
                leader_for: vec!["alice"],
            },
            issue_repo: FakeIssueRepo::default(),
        })
    }
}

async fn seed_mission_with_assignees(shared: &Shared, pairs: &[(&str, MissionRole)]) -> i64 {
    let uc = shared.mission_usecase();
    let mut assignees = Vec::new();
    for (i, (user, role)) in pairs.iter().enumerate() {
        if i == 0 {
            assignees.push(crate::usecase::AssigneeData {
                user_code: (*user).into(),
                role: *role,
            });
        }
    }
    let m = uc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: MissionKind::Crf,
                mission_code: format!("c{}", pairs.len()),
                assignees,
            },
        )
        .await
        .unwrap();
    for (user, role) in pairs.iter().skip(1) {
        uc.add_assignee(
            &Actor {
                user_code: "alice".into(),
            },
            m.id,
            crate::usecase::AssigneeData {
                user_code: (*user).into(),
                role: *role,
            },
        )
        .await
        .unwrap();
    }
    let _ = AssigneeNew {
        user_code: String::new(),
        role: MissionRole::Dev,
    };
    m.id
}

#[tokio::test]
async fn issue_create_close_reopen_by_leader() {
    let shared = Shared::new();
    let mission_id = seed_mission_with_assignees(&shared, &[("u1", MissionRole::Dev)]).await;
    let iuc = shared.issue_usecase();

    let view = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: Some("form AE".into()),
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(view.state, IssueState::Opened);
    let closed = iuc
        .close_issue(
            &Actor {
                user_code: "alice".into(),
            },
            view.id,
        )
        .await
        .unwrap();
    assert_eq!(closed.state, IssueState::Closed);
    let reopened = iuc
        .reopen_issue(
            &Actor {
                user_code: "alice".into(),
            },
            view.id,
        )
        .await
        .unwrap();
    assert_eq!(reopened.state, IssueState::Opened);
}

#[tokio::test]
async fn issue_qc_assignee_can_create_and_close() {
    let shared = Shared::new();
    let mission_id = seed_mission_with_assignees(&shared, &[("u1", MissionRole::Qc)]).await;
    let iuc = shared.issue_usecase();
    let view = iuc
        .create_issue(
            &Actor {
                user_code: "u1".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(view.issuer, "u1");
    iuc.close_issue(
        &Actor {
            user_code: "u1".into(),
        },
        view.id,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn issue_dev_assignee_can_only_comment() {
    let shared = Shared::new();
    let mission_id = seed_mission_with_assignees(&shared, &[("u1", MissionRole::Dev)]).await;
    let iuc = shared.issue_usecase();

    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();

    let dev = Actor {
        user_code: "u1".into(),
    };
    iuc.append_comment(&dev, issue.id, "hello".into())
        .await
        .unwrap();
    let err = iuc
        .create_issue(
            &dev,
            CreateIssue {
                mission_id,
                target_item: None,
                description: "second".into(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
    let err = iuc.close_issue(&dev, issue.id).await.unwrap_err();
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
    let err = iuc.reopen_issue(&dev, issue.id).await.unwrap_err();
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
    let err = iuc
        .update_issue_description(&dev, issue.id, "new".into())
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
}

#[tokio::test]
async fn issue_close_is_idempotent() {
    let shared = Shared::new();
    let mission_id = seed_mission_with_assignees(&shared, &[("u1", MissionRole::Dev)]).await;
    let iuc = shared.issue_usecase();
    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    iuc.append_comment(
        &Actor {
            user_code: "alice".into(),
        },
        issue.id,
        "before close".into(),
    )
    .await
    .unwrap();
    let once = iuc
        .close_issue(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
        )
        .await
        .unwrap();
    let twice = iuc
        .close_issue(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
        )
        .await
        .unwrap();
    assert_eq!(once.state, IssueState::Closed);
    assert_eq!(twice.state, IssueState::Closed);
    assert_eq!(twice.comments.len(), 1);
}

#[tokio::test]
async fn issue_reopen_is_idempotent() {
    let shared = Shared::new();
    let mission_id = seed_mission_with_assignees(&shared, &[("u1", MissionRole::Dev)]).await;
    let iuc = shared.issue_usecase();
    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    let first = iuc
        .reopen_issue(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
        )
        .await
        .unwrap();
    let second = iuc
        .reopen_issue(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
        )
        .await
        .unwrap();
    assert_eq!(first.state, IssueState::Opened);
    assert_eq!(second.state, IssueState::Opened);
}

#[tokio::test]
async fn issue_update_description_allowed_on_closed() {
    let shared = Shared::new();
    let mission_id = seed_mission_with_assignees(&shared, &[("u1", MissionRole::Dev)]).await;
    let iuc = shared.issue_usecase();
    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    iuc.close_issue(
        &Actor {
            user_code: "alice".into(),
        },
        issue.id,
    )
    .await
    .unwrap();
    let updated = iuc
        .update_issue_description(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
            "second".into(),
        )
        .await
        .unwrap();
    assert_eq!(updated.description, "second");
    assert_eq!(updated.state, IssueState::Closed);
}

#[tokio::test]
async fn issue_update_description_rejects_whitespace() {
    let shared = Shared::new();
    let mission_id = seed_mission_with_assignees(&shared, &[("u1", MissionRole::Dev)]).await;
    let iuc = shared.issue_usecase();
    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    let err = iuc
        .update_issue_description(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
            "   ".into(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Domain(DomainError::EmptyIssueDescription)
    ));
}

#[tokio::test]
async fn issue_append_comment_rejects_whitespace() {
    let shared = Shared::new();
    let mission_id = seed_mission_with_assignees(&shared, &[("u1", MissionRole::Dev)]).await;
    let iuc = shared.issue_usecase();
    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    let err = iuc
        .append_comment(
            &Actor {
                user_code: "u1".into(),
            },
            issue.id,
            "  ".into(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Domain(DomainError::EmptyCommentContent)
    ));
}
