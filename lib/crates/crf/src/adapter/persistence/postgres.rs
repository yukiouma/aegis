// SQLx runtime API is used throughout this crate. The workspace
// does not currently ship a `.sqlx/` offline cache, and the
// compile-time-checked macros would require either a live
// `DATABASE_URL` at build time or a checked-in `sqlx-data.json`.
// `sqlx::query_as` + `sqlx::query` + `FromRow` + `QueryBuilder`
// are sufficient and keep the crate reproducible.

pub mod annotation_repo;
pub mod crf_form_repo;
pub mod crf_item_repo;
pub mod crf_option_repo;
pub mod crf_unit_repo;
pub mod crf_version_repo;
pub mod domain_annotation_repo;
