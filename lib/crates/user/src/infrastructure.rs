// `row` is kept `pub(crate)` so external callers can name `UserRow`
// only via the crate-root re-export. That keeps the password column
// off the casual field-access surface; the redaction is enforced by
// `UserRow`'s manual `Debug` impl.
pub(crate) mod row;
#[cfg(test)]
mod tests;
pub mod user_repo;
