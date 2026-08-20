//! # terminology crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for the CDISC terminology aggregates and an async
//! `TerminologyUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::{CodeItemRepo, CodeListRepo, TerminologyServiceImpl, TerminologyVersionRepo};
pub use domain::{
    CodeItem, CodeItemListQuery, CodeItemNew, CodeItemRepository, CodeItemUpdate, CodeList,
    CodeListListQuery, CodeListNew, CodeListRepository, CodeListUpdate, DomainError, Page,
    TerminologyKind, TerminologyVersion, TerminologyVersionNew, TerminologyVersionRepository,
    TerminologyVersionUpdate,
};
pub use usecase::{
    CodeItemView, CodeListView, CreateCodeItem, CreateCodeList, CreateTerminologyVersion,
    TerminologyUsecase, TerminologyUsecaseConfig, TerminologyVersionView, UpdateCodeItem,
    UpdateCodeList, UpdateTerminologyVersion, UsecaseError,
};
