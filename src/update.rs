//! Checking GitHub for a newer DBDock release.
//!
//! One anonymous request to the public GitHub API on start; nothing about
//! the user or their databases is sent.

use std::time::Duration;

use serde::Deserialize;

/// The release DBDock is built from.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

const LATEST_RELEASE_API: &str = "https://api.github.com/repos/Pascold/DBDock/releases/latest";
const TIMEOUT: Duration = Duration::from_secs(10);

/// The newest published release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Tag such as `v0.1.1`.
    pub tag: String,
    /// Release page with the downloads.
    pub url: String,
}

#[derive(Deserialize)]
struct ApiRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// `major.minor.patch` from `v1.2.3` or `1.2.3-beta`; `None` if malformed.
pub fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let core = text.trim().trim_start_matches('v');
    let core = core.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

/// True if `tag` is a newer version than `current`.
pub fn is_newer(tag: &str, current: &str) -> bool {
    match (parse_version(tag), parse_version(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => false,
    }
}

/// Parses the `releases/latest` API response.
fn parse_release(json: &str) -> Option<Release> {
    let release: ApiRelease = serde_json::from_str(json).ok()?;
    (!release.draft && !release.prerelease).then_some(Release {
        tag: release.tag_name,
        url: release.html_url,
    })
}

/// Asks GitHub for the latest release. Blocking; call it on a background
/// thread. Returns `None` on any network or parsing problem, which simply
/// means "no update shown".
pub fn fetch_latest() -> Option<Release> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let body = agent
        .get(LATEST_RELEASE_API)
        .header("User-Agent", &format!("DBDock/{CURRENT_VERSION}"))
        .header("Accept", "application/vnd.github+json")
        .call()
        .ok()?
        .body_mut()
        .read_to_string()
        .ok()?;
    parse_release(&body)
}

/// The latest release if it is newer than this build.
pub fn check() -> Option<Release> {
    fetch_latest().filter(|release| is_newer(&release.tag, CURRENT_VERSION))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(parse_version("v0.1.0"), Some((0, 1, 0)));
        assert_eq!(parse_version("1.12.3"), Some((1, 12, 3)));
        assert_eq!(parse_version("v2.0.0-beta.1"), Some((2, 0, 0)));
        assert_eq!(parse_version("v1.2"), None);
        assert_eq!(parse_version("v1.2.3.4"), None);
        assert_eq!(parse_version("latest"), None);
    }

    #[test]
    fn newer_versions_are_detected() {
        assert!(is_newer("v0.1.1", "0.1.0"));
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("v1.0.0", "0.10.0"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("v0.0.9", "0.1.0"));
        assert!(!is_newer("nonsense", "0.1.0"));
    }

    #[test]
    fn api_response() {
        let json = r#"{"tag_name":"v0.2.0","html_url":"https://github.com/Pascold/DBDock/releases/tag/v0.2.0","draft":false,"prerelease":false,"assets":[]}"#;
        assert_eq!(
            parse_release(json),
            Some(Release {
                tag: "v0.2.0".into(),
                url: "https://github.com/Pascold/DBDock/releases/tag/v0.2.0".into(),
            })
        );
        let pre = json.replace("\"prerelease\":false", "\"prerelease\":true");
        assert_eq!(parse_release(&pre), None);
        assert_eq!(parse_release("{}"), None);
        assert_eq!(parse_release("not json"), None);
    }
}
