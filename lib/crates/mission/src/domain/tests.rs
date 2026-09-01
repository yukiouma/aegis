use super::{
    AssigneeNew, DomainError, Mission, MissionKind, MissionRole,
    assignees_within_mission_are_unique,
};

#[test]
fn mission_kind_round_trip() {
    for k in [
        MissionKind::Crf,
        MissionKind::Sdtm,
        MissionKind::Adam,
        MissionKind::Tfl,
    ] {
        let s = k.as_str();
        let parsed = MissionKind::try_from(s).expect("parses");
        assert_eq!(parsed, k);
    }
}

#[test]
fn mission_kind_unknown_rejected() {
    let err = MissionKind::try_from("not_a_kind").unwrap_err();
    assert!(matches!(err, DomainError::UnknownMissionKind(ref s) if s == "not_a_kind"));
}

#[test]
fn mission_role_round_trip() {
    for r in [MissionRole::Dev, MissionRole::Qc] {
        let s = r.as_str();
        let parsed = MissionRole::try_from(s).expect("parses");
        assert_eq!(parsed, r);
    }
}

#[test]
fn mission_role_unknown_rejected() {
    let err = MissionRole::try_from("manager").unwrap_err();
    assert!(matches!(err, DomainError::UnknownMissionRole(ref s) if s == "manager"));
}

#[test]
fn mission_new_rejects_empty_code() {
    let m = Mission::new(
        1,
        "p1".into(),
        MissionKind::Crf,
        "   ".into(),
        vec![],
        now(),
        now(),
    );
    assert!(matches!(m, Err(DomainError::EmptyMissionCode)));
}

#[test]
fn mission_new_accepts_non_empty_code() {
    let m = Mission::new(
        1,
        "p1".into(),
        MissionKind::Crf,
        "c1".into(),
        vec![],
        now(),
        now(),
    )
    .unwrap();
    assert_eq!(m.mission_code, "c1");
}

#[test]
fn assignee_new_rejects_empty_user_code() {
    let a = super::Assignee::new(1, "".into(), MissionRole::Dev, now(), now());
    assert!(matches!(a, Err(DomainError::EmptyUserCode)));
}

#[test]
fn assignees_within_mission_are_unique_detects_duplicate() {
    let assignees = vec![
        AssigneeNew {
            user_code: "u1".into(),
            role: MissionRole::Dev,
        },
        AssigneeNew {
            user_code: "u1".into(),
            role: MissionRole::Dev,
        },
    ];
    let err = assignees_within_mission_are_unique(&assignees).unwrap_err();
    assert!(matches!(err, DomainError::DuplicateAssignee { .. }));
}

#[test]
fn assignees_within_mission_are_unique_accepts_distinct_roles() {
    let assignees = vec![
        AssigneeNew {
            user_code: "u1".into(),
            role: MissionRole::Dev,
        },
        AssigneeNew {
            user_code: "u1".into(),
            role: MissionRole::Qc,
        },
    ];
    assert!(assignees_within_mission_are_unique(&assignees).is_ok());
}

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}
