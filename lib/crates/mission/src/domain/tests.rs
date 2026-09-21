use super::{
    AssigneeNew, DomainError, IssueComment, IssueState, Mission, MissionIssue, MissionKind,
    MissionRole, assignees_within_mission_are_unique,
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

#[test]
fn issue_state_round_trip() {
    for s in [IssueState::Opened, IssueState::Closed] {
        let str = s.as_str();
        let parsed = IssueState::try_from(str).expect("parses");
        assert_eq!(parsed, s);
    }
}

#[test]
fn issue_state_unknown_rejected() {
    let err = IssueState::try_from("pending").unwrap_err();
    assert!(matches!(err, DomainError::UnknownMissionRole(_)));
}

#[test]
fn issue_comment_new_rejects_empty_user() {
    let c = IssueComment::new("".into(), "content".into(), now());
    assert!(matches!(c, Err(DomainError::EmptyUserCode)));
}

#[test]
fn issue_comment_new_rejects_empty_content() {
    let c = IssueComment::new("u1".into(), "   ".into(), now());
    assert!(matches!(c, Err(DomainError::EmptyCommentContent)));
}

#[test]
fn issue_comment_new_accepts_valid() {
    let c = IssueComment::new("u1".into(), "hello".into(), now()).unwrap();
    assert_eq!(c.user, "u1");
    assert_eq!(c.content, "hello");
}

#[test]
fn mission_issue_new_rejects_empty_description() {
    let m = MissionIssue::new(
        1,
        1,
        None,
        "u1".into(),
        "   ".into(),
        IssueState::Opened,
        vec![],
        now(),
        now(),
    );
    assert!(matches!(m, Err(DomainError::EmptyIssueDescription)));
}

#[test]
fn mission_issue_new_rejects_empty_issuer() {
    let m = MissionIssue::new(
        1,
        1,
        None,
        "".into(),
        "desc".into(),
        IssueState::Opened,
        vec![],
        now(),
        now(),
    );
    assert!(matches!(m, Err(DomainError::EmptyIssueIssuer)));
}

#[test]
fn mission_issue_new_accepts_valid() {
    let m = MissionIssue::new(
        1,
        1,
        Some("form AE".into()),
        "u1".into(),
        "desc".into(),
        IssueState::Opened,
        vec![],
        now(),
        now(),
    )
    .unwrap();
    assert_eq!(m.issuer, "u1");
    assert_eq!(m.target_item.as_deref(), Some("form AE"));
    assert_eq!(m.state, IssueState::Opened);
}

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}
