//! Version ordering for the release channel.

use semver::Version;

/// True when `candidate` is strictly newer than `held`.
///
/// Both sides are parsed as semver with a leading `v` tolerated. If
/// either side fails to parse the answer is `false`: an unparsable pair
/// cannot prove that the candidate is not a downgrade, and treating it
/// as newer is how a second prompt for an older release gets in.
pub fn is_newer(candidate: &str, held: &str) -> bool {
    match (
        Version::parse(candidate.trim_start_matches('v')),
        Version::parse(held.trim_start_matches('v')),
    ) {
        (Ok(candidate), Ok(held)) => candidate > held,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_higher_version_is_newer() {
        assert!(is_newer("0.2.0", "0.1.1"));
        assert!(is_newer("0.1.2", "0.1.1"));
    }

    #[test]
    fn the_same_version_is_not_newer() {
        assert!(!is_newer("0.1.1", "0.1.1"));
    }

    #[test]
    fn an_older_version_is_not_newer() {
        assert!(!is_newer("0.1.5", "0.2.0"));
    }

    #[test]
    fn a_leading_v_is_tolerated() {
        assert!(is_newer("v0.2.0", "0.1.1"));
    }

    #[test]
    fn an_unparsable_pair_is_never_newer() {
        assert!(!is_newer("not-a-version", "0.1.1"));
        assert!(!is_newer("0.1.1", "not-a-version"));
    }
}
