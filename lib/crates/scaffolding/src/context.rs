use chrono::NaiveDate;

/// The values a template can interpolate.
///
/// `product` and `trial` are derived from `study` and are absent when
/// the study code does not carry them — a code with no `-` has no
/// `trial`, and a template using `{{ trial }}` is then unresolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub study: String,
    pub product: Option<String>,
    pub trial: Option<String>,
    pub branch: Option<String>,
    /// The current date, `yyyy-mm-dd`.
    pub date: String,
}

impl Context {
    /// Build the context, deriving `product` and `trial` from `study`
    /// and formatting `date`.
    pub fn new(study: &str, branch: Option<&str>, date: NaiveDate) -> Context {
        let mut parts = study.split('-');
        let product = parts.next().filter(|p| !p.is_empty()).map(str::to_string);
        let trial = parts.next().filter(|p| !p.is_empty()).map(str::to_string);
        Context {
            study: study.to_string(),
            product,
            trial,
            branch: branch.map(str::to_string),
            date: date.format("%Y-%m-%d").to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn derives_product_and_trial_from_a_two_part_study_code() {
        let ctx = Context::new("AK001-001", None, date(2026, 11, 6));
        assert_eq!(ctx.study, "AK001-001");
        assert_eq!(ctx.product.as_deref(), Some("AK001"));
        assert_eq!(ctx.trial.as_deref(), Some("001"));
    }

    #[test]
    fn a_study_code_without_a_dash_has_no_trial() {
        let ctx = Context::new("AK001", None, date(2026, 11, 6));
        assert_eq!(ctx.product.as_deref(), Some("AK001"));
        assert_eq!(ctx.trial, None);
    }

    #[test]
    fn keeps_the_branch_when_given_and_omits_it_otherwise() {
        let with = Context::new("AK001-001", Some("2026Q1"), date(2026, 11, 6));
        assert_eq!(with.branch.as_deref(), Some("2026Q1"));
        let without = Context::new("AK001-001", None, date(2026, 11, 6));
        assert_eq!(without.branch, None);
    }

    #[test]
    fn formats_the_date_as_year_month_day() {
        let ctx = Context::new("AK001-001", None, date(2026, 11, 6));
        assert_eq!(ctx.date, "2026-11-06");
    }
}
