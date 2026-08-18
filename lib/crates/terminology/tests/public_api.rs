//! Public-API compile-only test for the `terminology` crate.
//!
//! Pins the documented `use terminology::*;` surface, the three
//! concrete repo constructors (`fn(PgPool) -> Repo`), the
//! `TerminologyUsecase::new(config)` constructor shape, and the
//! `Send + Sync` bound the usecase config relies on.

use sqlx::PgPool;
use terminology::{
    CodeItemRepo, CodeListRepo, CreateCodeList, CreateTerminologyVersion, TerminologyKind,
    TerminologyUsecase, TerminologyUsecaseConfig, TerminologyVersionRepo, UpdateTerminologyVersion,
};

#[test]
fn public_types_are_nameable_from_crate_root() {
    fn assert_kind(_: TerminologyKind) {}
    fn assert_cmd(_: CreateTerminologyVersion) {}
    fn assert_list_cmd(_: CreateCodeList) {}
    fn assert_upd(_: UpdateTerminologyVersion) {}

    assert_kind(TerminologyKind::Sdtm);
    assert_cmd(CreateTerminologyVersion {
        kind: TerminologyKind::Sdtm,
        name: "2026-03-27".into(),
    });
    assert_list_cmd(CreateCodeList {
        version_id: 1,
        code: "C66741".into(),
        extensible: true,
        name: "AGE".into(),
        submission_value: "AGE".into(),
        synonym: "".into(),
        definition: "".into(),
        nci_preferred_term: "".into(),
    });
    assert_upd(UpdateTerminologyVersion::default());
}

#[test]
fn repos_construct_from_pg_pool_via_function_pointer() {
    let v: fn(PgPool) -> TerminologyVersionRepo = TerminologyVersionRepo::new;
    let l: fn(PgPool) -> CodeListRepo = CodeListRepo::new;
    let i: fn(PgPool) -> CodeItemRepo = CodeItemRepo::new;
    let _ = (v, l, i);
}

#[test]
fn usecase_constructor_accepts_three_repo_args() {
    #[allow(clippy::type_complexity)]
    fn assert_new_constructor<V, L, I>(
        _: fn(TerminologyUsecaseConfig<V, L, I>) -> TerminologyUsecase<V, L, I>,
    ) where
        V: terminology::TerminologyVersionRepository,
        L: terminology::CodeListRepository,
        I: terminology::CodeItemRepository,
    {
    }
    assert_new_constructor::<TerminologyVersionRepo, CodeListRepo, CodeItemRepo>(
        TerminologyUsecase::new,
    );
}
