//! Pagination envelope shared by every repository / usecase method
//! that returns more than one row.

/// One page of a paginated result set.
///
/// `items` are the rows for this page. `next_offset` is the cursor
/// to pass on the next request: pass it as `?offset=<value>` to read
/// the next page. `None` means this is the last page — there are no
/// more rows beyond the current `items`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_offset: Option<u32>,
}
