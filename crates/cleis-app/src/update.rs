//! Updates (D-74): the newest version published on GitHub. On Windows its installer (D-75) is
//! downloaded, checked against its published checksum and started silently with `/UPDATE`: it
//! asks this copy to quit (`--quit`), installs, then starts Cleis again. Elsewhere the release page
//! opens. Network and files: call off the interface thread.

use std::time::Duration;

/// Where versions are published (public repository).
const RELEASES: &str = "https://api.github.com/repos/matv-studio/Cleis/releases?per_page=30";
/// Looked again after this long while Cleis runs.
pub const EVERY: Duration = Duration::from_secs(6 * 3600);
/// Largest download accepted.
const MAX_BYTES: u64 = 300 * 1024 * 1024;

/// A version newer than the running one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// `0.2.1`, without the `v`.
    pub version: String,
    pub prerelease: bool,
    /// The release's page.
    pub page: String,
    /// The package for this computer and its checksum, when one is published.
    pub package: Option<(String, String)>,
}

/// The running version.
pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The end of the package name for this computer (`None`: Cleis cannot replace itself here,
/// the release page opens instead).
fn package_suffix() -> Option<&'static str> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        Some("-windows-x64-setup.exe")
    } else {
        None
    }
}

/// `0.2.1-beta.1` → comparable; `None` if it is not a version.
fn parse(v: &str) -> Option<semver::Version> {
    semver::Version::parse(v.trim().trim_start_matches('v')).ok()
}

/// The newest version above `current` in GitHub's answer (`releases`: its JSON list). Drafts
/// never count; pre-releases only with `pre`.
pub fn pick(
    releases: &serde_json::Value,
    current: &str,
    pre: bool,
    suffix: Option<&str>,
) -> Option<Release> {
    let now = parse(current)?;
    releases
        .as_array()?
        .iter()
        .filter(|r| !r["draft"].as_bool().unwrap_or(true))
        .filter(|r| pre || !r["prerelease"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let tag = r["tag_name"].as_str()?;
            let v = parse(tag)?;
            (v > now).then_some((v, r))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(v, r)| {
            let assets = r["assets"].as_array().cloned().unwrap_or_default();
            let url_of = |name: &str| {
                assets
                    .iter()
                    .find(|a| a["name"].as_str() == Some(name))
                    .and_then(|a| a["browser_download_url"].as_str())
                    .map(str::to_owned)
            };
            let package = suffix.and_then(|suffix| {
                let name = assets
                    .iter()
                    .filter_map(|a| a["name"].as_str())
                    .find(|n| n.starts_with(&format!("Cleis-{v}-")) && n.ends_with(suffix))?;
                Some((url_of(name)?, url_of(&format!("{name}.sha256"))?))
            });
            Release {
                version: v.to_string(),
                prerelease: r["prerelease"].as_bool().unwrap_or(false),
                page: r["html_url"].as_str().unwrap_or_default().to_owned(),
                package,
            }
        })
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .user_agent(concat!("Cleis/", env!("CARGO_PKG_VERSION")))
        .build()
        .new_agent()
}

/// Asks GitHub for the newest version (network).
pub fn check(pre: bool) -> Result<Option<Release>, String> {
    let list: serde_json::Value = agent()
        .get(RELEASES)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("GitHub: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| format!("GitHub: {e}"))?;
    Ok(pick(&list, current(), pre, package_suffix()))
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    agent()
        .get(url)
        .call()
        .map_err(|e| format!("download: {e}"))?
        .body_mut()
        .with_config()
        .limit(MAX_BYTES)
        .read_to_vec()
        .map_err(|e| format!("download: {e}"))
}

/// Checks the package against its published checksum (`<hex>  <name>`).
pub fn verify(package: &[u8], sums: &str) -> Result<(), String> {
    let want = sums
        .split_whitespace()
        .next()
        .ok_or("empty checksum file")?
        .to_ascii_lowercase();
    let got: String = cleis_crypto::sha256(package)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if got == want {
        Ok(())
    } else {
        Err("the download does not match its checksum".into())
    }
}

/// Downloads and checks the installer, then starts it silently (network and files); the caller
/// then quits, the installer waits for it.
pub fn install(r: &Release) -> Result<(), String> {
    let (url, sum_url) = r.package.as_ref().ok_or("no package for this computer")?;
    let setup = download(url)?;
    let sums = String::from_utf8(download(sum_url)?).map_err(|e| e.to_string())?;
    verify(&setup, &sums)?;
    let path = std::env::temp_dir().join(format!("Cleis-{}-setup.exe", r.version));
    std::fs::write(&path, &setup).map_err(|e| format!("{}: {e}", path.display()))?;
    std::process::Command::new(&path)
        .args(["/S", "/UPDATE"])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rel(tag: &str, pre: bool, draft: bool) -> serde_json::Value {
        let v = tag.trim_start_matches('v');
        json!({
            "tag_name": tag, "prerelease": pre, "draft": draft,
            "html_url": format!("https://github.com/matv-studio/Cleis/releases/tag/{tag}"),
            "assets": [
                {"name": format!("Cleis-{v}-abc1234-windows-x64-setup.exe"),
                 "browser_download_url": format!("https://example-dl.com/{v}.exe")},
                {"name": format!("Cleis-{v}-abc1234-windows-x64-setup.exe.sha256"),
                 "browser_download_url": format!("https://example-dl.com/{v}.exe.sha256")},
            ]
        })
    }

    #[test]
    fn picks_the_newest_allowed_version() {
        let list = json!([
            rel("v0.2.0", true, false),
            rel("v0.3.0", true, false),
            rel("v0.2.5", false, false),
            rel("v0.4.0", false, true),
        ]);
        let suffix = Some("-windows-x64-setup.exe");
        let r = pick(&list, "0.2.0", true, suffix).unwrap();
        assert_eq!(r.version, "0.3.0");
        assert!(r.prerelease);
        assert_eq!(
            r.package,
            Some((
                "https://example-dl.com/0.3.0.exe".into(),
                "https://example-dl.com/0.3.0.exe.sha256".into()
            ))
        );
        // Without pre-releases: the stable one; the draft never.
        assert_eq!(
            pick(&list, "0.2.0", false, suffix).unwrap().version,
            "0.2.5"
        );
        assert_eq!(pick(&list, "0.3.0", true, suffix), None);
        // No package for this computer: the page only.
        assert_eq!(pick(&list, "0.2.0", true, None).unwrap().package, None);
    }

    #[test]
    fn checks_the_package() {
        let data = b"package";
        let hex: String = cleis_crypto::sha256(data)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(verify(data, &format!("{}  x.zip\r\n", hex.to_uppercase())).is_ok());
        assert!(verify(b"other", &format!("{hex}  x.zip")).is_err());
    }
}
