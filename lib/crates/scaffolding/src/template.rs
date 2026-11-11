use crate::context::Context;

/// A field a template can interpolate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Study,
    Product,
    Trial,
    Branch,
    Date,
}

/// The transformation applied to a field's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Lowercase,
    Uppercase,
}

/// Why a template could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateError {
    #[error("unknown template field: {0}")]
    UnknownField(String),
    #[error("unknown template operation: {0}")]
    UnknownOperation(String),
    #[error("malformed template: {0}")]
    Malformed(String),
}

/// Resolve every `{{ … }}` template in `entry_name` against `ctx`.
///
/// A name with no templates is returned unchanged. The first failure
/// is returned as an error — the caller decides whether that is a skip
/// or a hard failure.
pub fn resolve(entry_name: &str, ctx: &Context) -> Result<String, TemplateError> {
    let mut out = String::with_capacity(entry_name.len());
    let mut rest = entry_name;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("}}").ok_or_else(|| {
            TemplateError::Malformed(format!("unclosed '{{{{' in {entry_name:?}"))
        })?;
        let body = &after[..end];
        out.push_str(&render(body, ctx)?);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Render one template body — the text between `{{` and `}}`.
fn render(body: &str, ctx: &Context) -> Result<String, TemplateError> {
    let mut parts = body.split('|');
    let name = parts.next().unwrap_or("").trim();
    let operation = parts.next().map(|op| op.trim());
    if parts.next().is_some() {
        return Err(TemplateError::Malformed(format!(
            "too many parts in {body:?}"
        )));
    }
    let value = match name {
        "study" => Some(ctx.study.clone()),
        "product" => ctx.product.clone(),
        "trial" => ctx.trial.clone(),
        "branch" => ctx.branch.clone(),
        "date" => Some(ctx.date.clone()),
        other => return Err(TemplateError::UnknownField(other.to_string())),
    };
    let value = value.ok_or_else(|| TemplateError::UnknownField(name.to_string()))?;
    Ok(match operation {
        None => value,
        Some("lowercase") => value.to_lowercase(),
        Some("uppercase") => value.to_uppercase(),
        Some(other) => return Err(TemplateError::UnknownOperation(other.to_string())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use chrono::NaiveDate;

    fn ctx() -> Context {
        Context::new(
            "AK001-001",
            Some("2026Q1"),
            NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
        )
    }

    #[test]
    fn resolves_a_bare_field() {
        assert_eq!(resolve("{{ study }}", &ctx()).unwrap(), "AK001-001");
    }

    #[test]
    fn applies_the_lowercase_operation() {
        assert_eq!(
            resolve("{{ product | lowercase }}", &ctx()).unwrap(),
            "ak001"
        );
    }

    #[test]
    fn applies_the_uppercase_operation() {
        assert_eq!(
            resolve("{{ branch | uppercase }}", &ctx()).unwrap(),
            "2026Q1"
        );
    }

    #[test]
    fn resolves_several_templates_in_one_name() {
        assert_eq!(
            resolve("{{ product }}-{{ trial }}", &ctx()).unwrap(),
            "AK001-001"
        );
    }

    #[test]
    fn ignores_whitespace_inside_the_braces() {
        assert_eq!(resolve("{{study}}", &ctx()).unwrap(), "AK001-001");
        assert_eq!(resolve("{{   product   }}", &ctx()).unwrap(), "AK001");
    }

    #[test]
    fn leaves_text_outside_the_templates_alone() {
        assert_eq!(
            resolve("raw_{{ trial }}_data", &ctx()).unwrap(),
            "raw_001_data"
        );
    }

    #[test]
    fn an_unknown_field_is_an_error() {
        let err = resolve("{{ prodcut }}", &ctx()).unwrap_err();
        assert!(matches!(err, TemplateError::UnknownField(f) if f == "prodcut"));
    }

    #[test]
    fn an_unknown_operation_is_an_error() {
        let err = resolve("{{ product | shout }}", &ctx()).unwrap_err();
        assert!(matches!(err, TemplateError::UnknownOperation(o) if o == "shout"));
    }

    #[test]
    fn an_unclosed_brace_is_malformed() {
        let err = resolve("{{ study", &ctx()).unwrap_err();
        assert!(matches!(err, TemplateError::Malformed(_)));
    }

    #[test]
    fn a_missing_optional_field_is_an_error() {
        let no_branch = Context::new(
            "AK001-001",
            None,
            NaiveDate::from_ymd_opt(2026, 11, 6).unwrap(),
        );
        let err = resolve("{{ branch }}", &no_branch).unwrap_err();
        assert!(matches!(err, TemplateError::UnknownField(f) if f == "branch"));
    }

    #[test]
    fn a_name_without_templates_is_returned_unchanged() {
        assert_eq!(resolve("01_raw", &ctx()).unwrap(), "01_raw");
    }
}
