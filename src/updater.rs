//! Metra-style update contract: GitHub latest.json discovery and explicit release-page download.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{io::Read, time::Duration};

pub const MANIFEST_URL: &str =
    "https://github.com/eyotang/RPCRad/releases/latest/download/latest.json";
const LIMIT: u64 = 64 * 1024;
#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub version: String,
    #[serde(default)]
    pub notes: String,
}
fn version(value: &str) -> Option<[u64; 3]> {
    let parts: Vec<_> = value
        .split('.')
        .map(|p| {
            if p.is_empty()
                || !p.bytes().all(|b| b.is_ascii_digit())
                || p.len() > 1 && p.starts_with('0')
            {
                None
            } else {
                p.parse().ok()
            }
        })
        .collect::<Option<_>>()?;
    parts.try_into().ok()
}
pub fn release_page(value: &str) -> Option<String> {
    version(value)?;
    Some(format!(
        "https://github.com/eyotang/RPCRad/releases/tag/v{value}"
    ))
}
fn parse(bytes: &[u8], current: &str) -> Result<Option<Release>> {
    ensure!(
        bytes.len() as u64 <= LIMIT,
        tr!("Update manifest exceeds the size limit")
    );
    let release: Release = serde_json::from_slice(bytes).context(tr!("Invalid update manifest"))?;
    let available = version(&release.version).context(tr!("Invalid release version"))?;
    let current = version(current).context(tr!("Invalid current version"))?;
    Ok((available > current).then_some(release))
}
pub fn check() -> Result<Option<Release>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("RPCRad/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || attempt.url().scheme() != "https" {
                attempt.error(tr!("Invalid update redirect"))
            } else {
                attempt.follow()
            }
        }))
        .build()?;
    let response = client
        .get(MANIFEST_URL)
        .send()
        .context(tr!("Cannot connect to the GitHub update service"))?;
    ensure!(
        response.status() != reqwest::StatusCode::NOT_FOUND,
        tr!("No public update manifest is available yet")
    );
    let response = response
        .error_for_status()
        .context(tr!("The update service returned an error"))?;
    let mut bytes = Vec::new();
    response.take(LIMIT + 1).read_to_end(&mut bytes)?;
    parse(&bytes, env!("CARGO_PKG_VERSION"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_new_stable_versions_are_offered() {
        assert!(parse(br#"{"version":"0.2.0"}"#, "0.1.0").unwrap().is_some());
        for v in ["0.1.0", "0.0.9"] {
            assert!(
                parse(format!(r#"{{"version":"{v}"}}"#).as_bytes(), "0.1.0")
                    .unwrap()
                    .is_none()
            );
        }
    }
    #[test]
    fn rejects_malformed_versions_and_oversized_manifests() {
        for v in ["../bad", "1.0", "01.0.0", "1.0.0-beta", "1.0.0.0"] {
            assert!(release_page(v).is_none());
        }
        assert!(parse(&vec![b' '; 65_537], "0.1.0").is_err());
        assert!(parse(br#"{"version":"9999999999999999999999.0.0"}"#, "0.1.0").is_err());
    }
    #[test]
    fn download_page_is_always_the_official_repository() {
        assert_eq!(
            release_page("0.2.0").as_deref(),
            Some("https://github.com/eyotang/RPCRad/releases/tag/v0.2.0")
        );
    }
}
