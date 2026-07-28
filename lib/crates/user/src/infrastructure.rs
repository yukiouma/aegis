// `row` is kept `pub(crate)` and is NOT re-exported at the crate
// root. `UserRow` is an internal row shape that exists only to bridge
// SQLx's `FromRow` derive into the domain `User` type; exposing it
// outside the crate would leak the password column onto a public
// field-access surface. The manual `Debug` impl on `UserRow` enforces
// the same hash redaction as the domain `User`.
//
// `user_repo` is kept private inside the crate for the same reason:
// the public surface at the crate root re-exports `UserRepo` directly
// (see `lib.rs`), so external callers never need to name the
// `user_repo` module. This matches the other two layers (`domain`,
// `usecase`), which also keep their child modules private.
pub(crate) mod row;
#[cfg(test)]
mod tests;
mod user_repo;

// Internal re-export so `lib.rs` can name `infrastructure::UserRepo`
// even though `user_repo` itself is private. External callers reach
// `UserRepo` via the crate-root re-export (`user::UserRepo`); the
// intermediate path `user::infrastructure::UserRepo` is also visible
// but only as a flat re-export of the type, not as a passable
// `user_repo` module path.
pub use user_repo::UserRepo;
