use apis::mission::Actor;

use crate::domain::{DomainError, MissionKind, MissionRole};
use crate::usecase::{CreateMission, MissionUsecase, MissionUsecaseConfig, UsecaseError};

#[path = "../test_support.rs"]
#[allow(clippy::duplicate_mod)]
mod test_support;
use test_support::{FakeAssigneeRepo, FakeMissionRepo, FakeProject, FakeUser};

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
    for (kind, code) in [
        (MissionKind::Crf, "c1"),
        (MissionKind::Sdtm, "s1"),
    ] {
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